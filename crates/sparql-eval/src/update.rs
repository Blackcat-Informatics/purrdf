// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! SPARQL 1.1 **UPDATE** evaluation over a [`MutableDataset`].
//!
//! `eval_update` applies a parsed [`Update`] to a copy-on-write
//! [`MutableDataset`] in request order — each operation observes the effects of the
//! earlier ones through the shared `m`. The engine seam ([`engine`](crate::engine))
//! drives this; the read query path is unchanged.
//!
//! # Doctrine / boundary decisions
//!
//! - **Implicit graph existence.** This is a quad store: a named graph *exists* iff
//!   it holds at least one quad. There is no empty-graph registry, so `CREATE GRAPH`
//!   is a no-op success and `CLEAR` ≡ `DROP` (both just remove every quad of the
//!   target — the only observable state a graph has). `CLEAR`/`DROP`/`CREATE` SILENT
//!   never errors here (there is no missing-graph condition to fail on).
//! - **Snapshot per WHERE op + value-space round-trip.** A `DELETE/INSERT … WHERE`
//!   evaluates its `WHERE` against a *frozen snapshot* of the current effective set
//!   (`m.freeze()`), because the evaluator reads a concrete [`RdfDataset`]. Each
//!   solution term is resolved to a dataset-independent [`TermValue`] (the template
//!   helpers do this), so the resulting quads stay valid after the snapshot is
//!   dropped and are applied back to `m` by value. DELETE is applied before INSERT
//!   (SPARQL §3.1.3), per solution row.
//! - **`WITH` / `USING` active dataset.** `WITH <g>` scopes the `WHERE` default graph
//!   to `g` (and is the default target for template quads). `USING` / `USING NAMED`
//!   build a custom `WHERE` active dataset (§3.1.3): when present they replace `WITH`'s
//!   effect on the `WHERE` (but `WITH` still targets the templates). A `USING` IRI that
//!   names no graph contributes nothing (never an error).
//! - **Blank nodes.** `INSERT DATA` blanks are minted fresh, ONE shared blank-map
//!   for the whole op (they co-refer within the op). `DELETE DATA` is blank-free (a
//!   parser invariant). Template (`DELETE`/`INSERT … WHERE`) blanks are minted fresh
//!   per solution row, exactly like `CONSTRUCT`. All of this mints from ONE
//!   monotonic counter threaded across the whole request (`eval_update`'s
//!   `bnode_counter`), never reset between operations, so a `_:b` label in one
//!   operation is a distinct blank from the same label in another operation
//!   (SPARQL 1.1 Update §4.1.1 / §19.6).
//! - **`LOAD` host seam.** The core is network-free. `LOAD <iri>` needs a host
//!   [`GraphResolver`] to fetch + parse the source into a frozen dataset; with no
//!   resolver, `LOAD` hard-fails unless `SILENT`.
//!
//! # Governors: what an UPDATE is charged for, and why a trip applies nothing
//!
//! An UPDATE runs under the same [`GovernorState`] a governed query runs under, threaded in
//! through the engine seam's evaluation config, and a trip leaves this module as a typed
//! governor rather than as a stringly diagnostic.
//!
//! Two kinds of work are charged, because an UPDATE does two kinds of work:
//!
//! - The `WHERE` clause of a `DELETE/INSERT` is evaluated through an [`EvalCtx`] carrying
//!   the request's governors, so it charges and stops **exactly** as the same pattern
//!   inside a governed `SELECT` does — the same fuel, the same intermediate-cell peak, the
//!   same scratch bytes, the same admission refusal.
//! - The **mutation itself** is charged per quad, at
//!   [`ChargePoint::UpdateMutatedQuad`]. This is not decoration. `CLEAR ALL`, `MOVE`,
//!   `COPY`, `ADD`, `LOAD` and the two `DATA` forms do work proportional to the *store*,
//!   and none of it enters the evaluator — so a ceiling that only bound the `WHERE` would
//!   be one an entire half of the UPDATE surface could never reach, on exactly the
//!   operations whose cost is unbounded by the request text.
//!
//! The stop signal is polled before each operation and immediately before the `LOAD` host
//! seam issues I/O.
//!
//! ## A trip applies nothing
//!
//! `m` **is** mutated incrementally, both within an operation and across the operations of
//! a request: operation N+1 must observe operation N's effects (SPARQL 1.1 Update §3.1), so
//! there is no version of this loop in which earlier operations are held back, and a bulk
//! operation that trips part-way through has already written part of itself. None of that
//! is a rollback problem, because none of it is visible.
//!
//! [`MutableDataset`] is a copy-on-write **branch** off the caller's frozen base, and
//! `crate::engine` publishes it back with a single `m.freeze()` assignment that happens on
//! the success path and nowhere else. Every non-success exit — a diagnostic or a governor
//! trip — simply drops `m`, and dropping a branch is the rollback: the caller's
//! `Arc<RdfDataset>` was never written, so it is the same handle it was before, not merely
//! an equal one. Whatever a half-run operation had reached is dropped with it.
//!
//! Nothing here therefore needs a snapshot/restore of its own, and adding one would be a
//! second, weaker copy of a guarantee the branch already gives structurally. It is also why
//! charging *before* applying a batch is a courtesy rather than a correctness requirement:
//! it wastes less work on a request that is going to be discarded, and discards it either
//! way.

use std::sync::Arc;

use purrdf_core::{
    DatasetMut, GraphMatchValue, MutableDataset, QuadValues, RdfDataset, RdfDiagnostic,
    ResourceDimension, TermValue, TrippedGovernor,
};
use purrdf_sparql_algebra::{
    GraphTarget, GraphUpdateOperation, NamedNodePattern, QuadPattern, Update, UsingClause,
};

use crate::DetHashMap;
use crate::convert::named_node_to_value;
use crate::dataset_spec::ActiveDataset;
use crate::engine::{QueryOptions, apply_query_options};
use crate::eval::{
    AdmittedRequest, BgpOrderCache, EvalCtx, StandpointPredicates, admit_version, eval_evaluated,
};
use crate::governor::{ChargePoint, GovernorState, StopSignal};
use crate::solution::{Solution, VarSchema};
use crate::template::{
    instantiate_ground_term, instantiate_predicate, instantiate_term, positionally_ill_formed,
};

/// Why an UPDATE request stopped before applying.
///
/// The two arms are genuinely different events and the caller acts on them differently, so
/// they are not one type with a string in it. A [`Self::Failed`] says the request could not
/// be carried out — it is malformed, or it asked for a host seam that is not there. A
/// [`Self::Tripped`] says the request was *fine* and a resource ceiling stopped it; the
/// remedy is a larger budget, and the caller needs the [`TrippedGovernor`] to know which
/// ceiling to raise. Rendering the second as an `RdfDiagnostic` with a formatted message —
/// which is what this path used to do, on a branch nothing could reach — throws that away
/// and tells the caller the engine misbehaved.
///
/// Either arm applies **nothing**: see the module docs.
#[derive(Debug)]
pub(crate) enum UpdateAbort {
    /// The request could not be carried out.
    Failed(RdfDiagnostic),
    /// A governor stopped the request. Only reachable when a [`GovernorState`] was
    /// threaded in through [`UpdateEvalConfig::governors`].
    Tripped(TrippedGovernor),
}

impl From<RdfDiagnostic> for UpdateAbort {
    fn from(diagnostic: RdfDiagnostic) -> Self {
        Self::Failed(diagnostic)
    }
}

/// Refuse a mutation whose quad carries a non-absolute IRI, reporting the workspace's
/// shared [`purrdf_core::IriError::diagnostic_code`] spelling.
///
/// A SPARQL IRI is resolved against `BASE` while the request is parsed, so a
/// well-formed request cannot reach this. It is still checked rather than assumed:
/// `INSERT DATA` is a genuine ingress into the store, and the absoluteness invariant
/// is enforced where terms ENTER the store, not where we believe they came from.
fn iri_abort(err: &purrdf_core::IriError) -> UpdateAbort {
    UpdateAbort::Failed(RdfDiagnostic::error(
        err.diagnostic_code(),
        format!("UPDATE cannot insert a quad carrying a non-absolute IRI: {err}"),
    ))
}

/// The engine-level WHERE-evaluation config threaded into UPDATE, mirroring the
/// query path's `EvalCtx` build (order cache + standpoint predicate table) so a
/// `DELETE/INSERT … WHERE` evaluates identically to a `SELECT`.
pub(crate) struct UpdateEvalConfig<'e> {
    pub(crate) standpoint_predicates: Option<&'e StandpointPredicates>,
    pub(crate) order_cache: &'e BgpOrderCache,
    /// This request's live governor accounting, or `None` for an ungoverned request.
    ///
    /// `None` is not "unbounded": it is the *absence* of the state, which is what keeps an
    /// ungoverned UPDATE byte-for-byte the execution it was before governors existed —
    /// `EvalCtx` never acquires a governor, so no charge site, no stop poll, and no
    /// truncation channel is reachable. It is also what makes [`UpdateAbort::Tripped`]
    /// structurally impossible on the ungoverned seam.
    pub(crate) governors: Option<&'e Arc<GovernorState>>,
    /// The registries this request's `WHERE` clauses run under: the property-function
    /// registry (read at admission and applied to the `WHERE` [`EvalCtx`] through the
    /// same seam a governed query applies it — see [`apply_query_options`]), the
    /// SHACL-AF function registry, and the blank-mint prefix. An UPDATE `WHERE` is a
    /// triple-pattern context exactly like a query's, so it takes the identical
    /// [`QueryOptions`] a query takes — [`QueryOptions::EMPTY`] is "configure nothing",
    /// what every UPDATE ran under before this seam existed.
    pub(crate) options: QueryOptions<'e>,
}

/// Poll the request's stop signal, converting a fired signal into a trip.
///
/// Called at the two places the evaluator's own charge-point polling cannot reach: before
/// starting each operation of a request, and — the load-bearing one — immediately before
/// the `LOAD` host seam issues I/O. [`StopSignal`](crate::governor::StopSignal) latches by
/// contract, so the trip reported is the one already recorded on the state rather than a
/// fresh derivation of the same condition; that is what keeps a stop that raced some other
/// ceiling reporting one governor rather than two.
fn check_stop(governors: Option<&Arc<GovernorState>>) -> Result<(), UpdateAbort> {
    let Some(state) = governors else {
        return Ok(());
    };
    let Some(cause) = state.poll_stop() else {
        return Ok(());
    };
    // `poll_stop` latched the cause into the state, so `tripped()` is the reported
    // governor; the fallback keeps this a total function and is not reachable through a
    // signal that honours the latching contract.
    Err(UpdateAbort::Tripped(
        state
            .tripped()
            .unwrap_or(TrippedGovernor::Stopped { cause }),
    ))
}

/// Charge the one request a `LOAD` issues to the host, before it is issued.
///
/// The same pair the `SERVICE` federation seam charges — the fuel charge point and the
/// [`ResourceDimension::RemoteRequests`] dimension — because it is the same event: the
/// engine handing a dereferenceable IRI to a host and waiting. A caller who bounds how many
/// endpoints a federated query may consult means the same thing by that number when the
/// request is a `LOAD`, and a ceiling that governed one and not the other would be a
/// ceiling whose meaning depended on which clause spelled the fetch.
///
/// This is also the only ceiling that can act on a `LOAD` *before* the network is touched.
/// Fuel cannot: the document's size is unknown until it has been fetched, so the
/// per-quad ingest charge is necessarily after the fact. Bounding the number of fetches is
/// the bound that is knowable in advance, so it is the one taken in advance.
fn charge_host_fetch(governors: Option<&Arc<GovernorState>>) -> Result<(), UpdateAbort> {
    let Some(state) = governors else {
        return Ok(());
    };
    check_stop(governors)?;
    state
        .charge_point_if_engaged(ChargePoint::RemoteRequestIssued)
        .and_then(|()| state.charge_if_engaged(ResourceDimension::RemoteRequests, 1))
        .map_err(UpdateAbort::Tripped)
}

/// Charge `quads` mutated quads against the request's fuel.
///
/// The one charge site for the mutation half of an UPDATE, called by every operation that
/// writes to `m`. Charged as a batch rather than a quad at a time wherever the count is
/// known in advance, which is everywhere except the two `DATA` forms (whose quads are
/// instantiated one at a time and may individually be skipped as ill-formed, so charging a
/// batch would charge for quads no operation ever wrote).
///
/// An ungoverned request pays one `Option` test; a request whose fuel is unbounded pays one
/// further array read, through
/// [`charge_if_engaged`](GovernorState::charge_if_engaged)'s short-circuit — the same
/// short-circuit that keeps an ungoverned query costing what it cost before governors
/// existed.
///
/// Saturating rather than wrapping: a store large enough to overflow the product is one no
/// budget could admit anyway, so failing closed at `u64::MAX` is the honest answer.
fn charge_mutations(
    governors: Option<&Arc<GovernorState>>,
    quads: usize,
) -> Result<(), UpdateAbort> {
    let Some(state) = governors else {
        return Ok(());
    };
    // A deadline/cancellation-only request deliberately leaves fuel unbounded, but every
    // mutation boundary must still observe its stop signal.
    check_stop(governors)?;
    let units = u64::try_from(quads)
        .unwrap_or(u64::MAX)
        .saturating_mul(ChargePoint::UpdateMutatedQuad.cost());
    state
        .charge_if_engaged(ResourceDimension::Fuel, units)
        .map_err(UpdateAbort::Tripped)
}

/// One governed request handed to a SPARQL `LOAD` host resolver.
#[derive(Clone, Copy)]
pub struct GraphResolveRequest<'a> {
    /// Absolute source IRI from the parsed `LOAD` operation.
    pub iri: &'a str,
    /// The executing request's latching stop signal, when one was supplied.
    ///
    /// A resolver that can abandon an in-flight fetch should poll it while waiting. The
    /// evaluator also polls immediately after [`GraphResolver::resolve`] returns, so a
    /// resolver that cannot abandon still cannot publish work completed after a stop.
    pub stop: Option<&'a dyn StopSignal>,
}

impl core::fmt::Debug for GraphResolveRequest<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("GraphResolveRequest")
            .field("iri", &self.iri)
            .field("stop", &self.stop.is_some())
            .finish()
    }
}

/// Host seam for SPARQL `LOAD <iri>`: resolves a source IRI to a frozen dataset.
///
/// The evaluator core is **network-free** (it builds clean for wasm and pulls no
/// HTTP/parse stack). A host that wants `LOAD` to dereference real documents injects
/// a resolver: it is responsible for fetching the IRI and parsing the response into
/// a frozen [`RdfDataset`]. Without a resolver, `LOAD` hard-fails (unless `SILENT`).
pub trait GraphResolver {
    /// Resolve `request.iri` to a frozen dataset, or a diagnostic on fetch/parse failure.
    fn resolve(&self, request: GraphResolveRequest<'_>) -> Result<Arc<RdfDataset>, RdfDiagnostic>;
}

/// Apply a parsed [`Update`] to `m` in request order.
///
/// Returns `Ok(())` on success and an [`UpdateAbort`] otherwise: a specific
/// [`RdfDiagnostic`] code on the boundary conditions (an unrecognized `VERSION`, `LOAD`
/// with no resolver, a bad re-key destination, an internal eval error), or the
/// [`TrippedGovernor`] that stopped the request. `resolver` supplies the `LOAD` host seam
/// (see [`GraphResolver`]); pass `None` to make any non-`SILENT` `LOAD` a hard error.
///
/// On **either** abort, `m` is left in whatever state the operations reached and is
/// expected to be dropped rather than frozen — that discard is the request's rollback, and
/// the module docs explain why it is the whole of it.
///
/// # This is the update-side `VERSION` admission chokepoint
///
/// [`admit_version`] runs FIRST, before `m` is touched at all (not even the blank-mint
/// counter is initialized yet) and before the operation loop starts: an update whose
/// prologue names an unrecognized `VERSION` is refused with no mutation applied, exactly
/// mirroring [`crate::eval::evaluate_query_evaluated`]'s admission of the same declaration
/// on the query side. This function is the single place every UPDATE entry point converges
/// on — [`crate::engine::NativeSparqlEngine::update_with_options`] (the ungoverned seam,
/// which [`crate::engine::NativeSparqlEngine::update`] and the `SparqlEngine::update` trait
/// impl delegate to) and
/// [`crate::engine::NativeSparqlEngine::update_governed`] both call it and nothing else —
/// so admission cannot be bypassed by choosing a different entry point, including the CLI,
/// the C ABI, wasm, and the Python bindings, all of which route through one of those two
/// seams.
// The in-crate callers are the engine UPDATE seams (`engine::update_with_options` and
// `NativeSparqlEngine::update_governed`).
pub(crate) fn eval_update(
    update: &Update,
    m: &mut MutableDataset,
    resolver: Option<&dyn GraphResolver>,
    cfg: &UpdateEvalConfig<'_>,
) -> Result<(), UpdateAbort> {
    admit_version(AdmittedRequest::Update(update)).map_err(|e| {
        RdfDiagnostic::error(
            crate::engine::eval_diagnostic_code(&e, "native-sparql-update-eval"),
            e.to_string(),
        )
    })?;
    // A single monotonic counter threaded across EVERY operation in this request, so
    // a synthetic blank label minted by operation N can never collide with one minted
    // by operation N+1 — even though each operation's own `_:b` → label map starts
    // empty. Per SPARQL 1.1 Update §4.1.1 / §19.6, a blank-node label is scoped to the
    // single operation (and, inside an INSERT/DELETE … WHERE, freshly per solution
    // row): `_:b` in one operation is a DIFFERENT blank node from `_:b` in another
    // operation of the same request. Resetting the counter per-operation (the
    // previous behaviour) let two operations mint the same synthetic label, so their
    // blanks silently unified in the shared store.
    let mut bnode_counter: u64 = 0;
    for op in &update.operations {
        apply_operation(op, m, resolver, cfg, &mut bnode_counter)?;
        // Close the single-operation terminal hole as well as the gap between operations.
        check_stop(cfg.governors)?;
    }
    Ok(())
}

/// Apply one update operation to `m`. `bnode_counter` is the request-wide monotonic
/// blank-mint counter (see [`eval_update`]) — never reset between operations.
///
/// The stop signal is polled here, before the operation starts, which is the granularity a
/// request has: a fuel ceiling stops an operation *within* itself (every mutating operation
/// charges — see [`charge_mutations`]), but a deadline or a cancellation is observed
/// between operations, exactly as the query evaluator observes one between operator
/// boundaries rather than between charge points. Without this poll a cancelled
/// thousand-operation request would run every one of them with a latched signal nobody
/// asked. Polling a latching signal is idempotent and costs an ungoverned request nothing
/// (`governors` is `None`, and the poll returns before touching anything).
fn apply_operation(
    op: &GraphUpdateOperation,
    m: &mut MutableDataset,
    resolver: Option<&dyn GraphResolver>,
    cfg: &UpdateEvalConfig<'_>,
    bnode_counter: &mut u64,
) -> Result<(), UpdateAbort> {
    check_stop(cfg.governors)?;
    match op {
        GraphUpdateOperation::InsertData { data } => {
            insert_data(data, m, bnode_counter, cfg.governors)
        }
        GraphUpdateOperation::DeleteData { data } => {
            delete_data(data, m, bnode_counter, cfg.governors)
        }
        GraphUpdateOperation::DeleteInsert {
            delete,
            insert,
            with,
            using,
            pattern,
        } => delete_insert(
            DeleteInsertSpec {
                delete,
                insert,
                with: with.as_ref(),
                using,
                pattern,
            },
            m,
            cfg,
            bnode_counter,
        ),
        GraphUpdateOperation::Load {
            silent,
            source,
            destination,
        } => load(
            *silent,
            source.as_str(),
            destination,
            m,
            resolver,
            cfg.governors,
        ),
        // CLEAR ≡ DROP in a quad store with implicit graph existence (see module docs).
        GraphUpdateOperation::Clear { target, .. } | GraphUpdateOperation::Drop { target, .. } => {
            clear_target(target, m, cfg.governors)
        }
        // Graph existence is implicit, so CREATE has nothing to register: no-op success.
        GraphUpdateOperation::Create { .. } => Ok(()),
        GraphUpdateOperation::Add {
            source,
            destination,
            ..
        } => graph_op_add(source, destination, m, cfg.governors),
        GraphUpdateOperation::Move {
            source,
            destination,
            ..
        } => graph_op_move(source, destination, m, cfg.governors),
        GraphUpdateOperation::Copy {
            source,
            destination,
            ..
        } => graph_op_copy(source, destination, m, cfg.governors),
    }
}

// ── INSERT DATA / DELETE DATA ────────────────────────────────────────────────

/// `INSERT DATA`: instantiate each quad (variable-free by parser invariant) with ONE
/// shared blank-map (blanks co-refer within the op) and insert the result.
///
/// DATA never queries the dataset, so it takes the snapshot-free ground path: no
/// `m.freeze()` (which would compact the whole base+delta for nothing) and no
/// `EvalCtx`. Blanks mint from the request-wide `counter` (see [`eval_update`]), so
/// this op's labels never collide with another operation's.
fn insert_data(
    data: &[QuadPattern],
    m: &mut MutableDataset,
    counter: &mut u64,
    governors: Option<&Arc<GovernorState>>,
) -> Result<(), UpdateAbort> {
    let mut blanks: DetHashMap<String, String> = DetHashMap::default();
    for qp in data {
        if let Some(q) = instantiate_ground_quad(qp, &mut blanks, counter) {
            // Charged per quad rather than per operation because an ill-formed template
            // quad is skipped rather than inserted (§16.2), and fuel counts what the store
            // actually did.
            charge_mutations(governors, 1)?;
            m.insert(q).map_err(|e| iri_abort(&e))?;
        }
    }
    Ok(())
}

/// `DELETE DATA`: instantiate each quad (variable-free AND blank-free — parser
/// guaranteed) and remove the result. Snapshot-free, like [`insert_data`].
fn delete_data(
    data: &[QuadPattern],
    m: &mut MutableDataset,
    counter: &mut u64,
    governors: Option<&Arc<GovernorState>>,
) -> Result<(), UpdateAbort> {
    let mut blanks: DetHashMap<String, String> = DetHashMap::default();
    for qp in data {
        if let Some(q) = instantiate_ground_quad(qp, &mut blanks, counter) {
            charge_mutations(governors, 1)?;
            m.remove(&q);
        }
    }
    Ok(())
}

// ── DELETE / INSERT … WHERE ──────────────────────────────────────────────────

/// The DELETE/INSERT/WITH/USING/WHERE fields of a `GraphUpdateOperation::DeleteInsert`,
/// bundled so [`delete_insert`] stays under clippy's argument-count ceiling.
#[derive(Clone, Copy)]
struct DeleteInsertSpec<'a> {
    delete: &'a [QuadPattern],
    insert: &'a [QuadPattern],
    with: Option<&'a purrdf_sparql_algebra::NamedNode>,
    using: &'a [UsingClause],
    pattern: &'a purrdf_sparql_algebra::GraphPattern,
}

/// `DELETE { ... } INSERT { ... } WHERE { ... }` and its shorthands.
fn delete_insert(
    spec: DeleteInsertSpec<'_>,
    m: &mut MutableDataset,
    cfg: &UpdateEvalConfig<'_>,
    bnode_counter: &mut u64,
) -> Result<(), UpdateAbort> {
    let DeleteInsertSpec {
        delete,
        insert,
        with,
        using,
        pattern,
    } = spec;
    // The WITH graph is the default target for delete/insert quads whose own
    // QuadPattern.graph is None (template target — independent of the WHERE dataset).
    let with_value = with.map(named_node_to_value);

    let snap = m.freeze()?;

    // The `prepare_for`-equivalent for an UPDATE's `WHERE`: an UPDATE `WHERE` is a
    // triple-pattern context exactly like a query's, so a registered relation's
    // predicate is admitted and feasibility-ordered here exactly as it would be in a
    // governed `SELECT` — before this operation's mutation, or its governor charges,
    // read a single cell of it. `Ok(None)` (no call node in this pattern — every
    // pattern on a host that has not configured the seam) leaves `pattern` as parsed.
    let planned = crate::property_fn_plan::plan_where_pattern(
        pattern,
        cfg.options.property_functions,
        cfg.options.aggregates,
    )
    .map_err(|e| RdfDiagnostic::error(e.diagnostic_code(), e.to_string()))?;
    let pattern: &purrdf_sparql_algebra::GraphPattern = planned.as_ref().unwrap_or(pattern);

    let ctx = EvalCtx::new(&*snap).with_order_cache(cfg.order_cache);
    // The property-function registry, the SHACL-AF function registry and the
    // blank-mint prefix, applied through the SAME seam a governed/ungoverned query
    // applies them — see `crate::engine::apply_query_options`. This is what lets a
    // call node reach evaluation at all: without it `ctx` carries no registry and
    // every call in this `WHERE` hard-errors "no property function is registered",
    // regardless of whether one was configured for the request.
    let mut ctx = apply_query_options(ctx, cfg.options)?;
    // The request's governors, so the `WHERE` charges and stops exactly as the same
    // pattern would inside a governed `SELECT`. Without this the ceilings a caller set
    // would bound their queries and silently not bound their mutations.
    if let Some(state) = cfg.governors {
        ctx = ctx.with_governors(Arc::clone(state));
    }
    if let Some(preds) = cfg.standpoint_predicates {
        ctx = ctx.with_standpoint_predicates(preds.clone());
    }
    // Seed the WHERE/template context from the request-wide counter (never reset
    // between operations — see `eval_update`) and hand it back below, so blanks
    // minted by this operation (template blanks, `BNODE()`, `rdf:List` cells) stay
    // disjoint from every other operation's in the same request.
    ctx.bnode_counter = *bnode_counter;

    // Scope the WHERE active dataset (§3.1.3): USING (if present) builds a custom
    // dataset and replaces WITH's effect on the WHERE; otherwise WITH scopes the WHERE
    // default graph; otherwise the dataset under mutation. An absent USING/WITH graph
    // contributes nothing (matches nothing) — never an error.
    ctx.active_dataset = if !using.is_empty() {
        ActiveDataset::from_using(using, &snap)
    } else if let Some(g) = &with_value {
        ActiveDataset::with_default_graph(&snap, g)
    } else {
        ActiveDataset::store_default()
    };

    admit_where(
        pattern,
        &snap,
        &ctx.active_dataset,
        cfg.governors,
        cfg.options.property_functions,
    )?;

    // A truncated `WHERE` must apply NO mutation: a half-applied UPDATE is not an
    // incomplete result, it is a corrupt store. Refusing the whole operation is the only
    // sound reading — and the certified partial rows the query path would hand back are
    // deliberately dropped here rather than reported, because there is no sound use for
    // them: instantiating a template from "some of the answers" is exactly the half-applied
    // mutation this refuses. What crosses is the typed governor, so the caller learns which
    // ceiling to raise.
    //
    // Nothing has been written to `m` at this point (the mutations below are collected
    // first), so this return needs no undo of its own.
    let seq = eval_evaluated(pattern, &mut ctx)
        .map_err(|e| {
            RdfDiagnostic::error(
                crate::engine::eval_diagnostic_code(&e, "native-sparql-update-eval"),
                e.to_string(),
            )
        })?
        .into_complete()
        .map_err(|truncation| UpdateAbort::Tripped(truncation.tripped()))?;
    let schema = seq.schema.clone();

    // Collect the mutations BEFORE touching `m`, so the snapshot stays valid for the
    // value resolution. DELETE before INSERT per row (SPARQL §3.1.3).
    let mut to_remove = Vec::new();
    let mut to_insert = Vec::new();
    // Blank-label maps are reset PER ROW (template blanks co-refer within a row, are
    // distinct across rows) but the allocation is hoisted: `.clear()` reuses the
    // capacity instead of allocating a fresh map for every solution.
    let mut del_blanks: DetHashMap<String, String> = DetHashMap::default();
    let mut ins_blanks: DetHashMap<String, String> = DetHashMap::default();
    for row in &seq.rows {
        del_blanks.clear();
        for qp in delete {
            if let Some(q) = instantiate_quad_with_default(
                qp,
                row,
                &schema,
                &mut del_blanks,
                &mut ctx,
                with_value.as_ref(),
            ) {
                observe_staged_mutation(
                    cfg.governors,
                    to_remove.len().saturating_add(to_insert.len()),
                )?;
                to_remove.push(q);
            }
        }
        ins_blanks.clear();
        for qp in insert {
            if let Some(q) = instantiate_quad_with_default(
                qp,
                row,
                &schema,
                &mut ins_blanks,
                &mut ctx,
                with_value.as_ref(),
            ) {
                observe_staged_mutation(
                    cfg.governors,
                    to_remove.len().saturating_add(to_insert.len()),
                )?;
                to_insert.push(q);
            }
        }
    }
    *bnode_counter = ctx.bnode_counter;
    drop(ctx);
    drop(snap);

    // The mutation this operation computed, charged as one batch before any of it lands.
    // The `WHERE` that produced it was charged as a query; this is the store's half of the
    // cost, which no ceiling on the `WHERE` bounds — one solution row can instantiate an
    // arbitrarily large template.
    charge_mutations(cfg.governors, to_remove.len() + to_insert.len())?;
    for q in &to_remove {
        m.remove(q);
    }
    for q in to_insert {
        m.insert(q).map_err(|e| iri_abort(&e))?;
    }
    Ok(())
}

/// Admit one more quad into an UPDATE's all-or-nothing mutation staging area before either
/// staging vector grows. A quad occupies four RDF positions (the graph position is a cell
/// even when it is the default graph), so the query-wide intermediate-cell ceiling also
/// bounds a template that expands one WHERE row into an arbitrarily large mutation batch.
fn observe_staged_mutation(
    governors: Option<&Arc<GovernorState>>,
    already_staged: usize,
) -> Result<(), UpdateAbort> {
    let Some(state) = governors else {
        return Ok(());
    };
    let dimension = ResourceDimension::IntermediateCells;
    if !state.is_engaged_in(dimension) {
        return Ok(());
    }
    let attempted = (already_staged as u64).saturating_add(1).saturating_mul(4);
    state
        .observe_peak(dimension, attempted)
        .map_err(UpdateAbort::Tripped)
}

/// Refuse an UPDATE's `WHERE` whose predicted peak intermediate bag already breaches the
/// caller's intermediate-cell ceiling.
///
/// The mutation-path twin of the query path's admission control
/// (`NativeSparqlEngine::admit`), and it exists for the identical reason: every other
/// governor is a meter, and a meter reports the cross product **after** it is in memory.
/// On `wasm32` that difference is total rather than uncomfortable — an allocation trap
/// aborts the module, so there is no execution left to return an outcome. An UPDATE's
/// `WHERE` builds its bag exactly the way a `SELECT` builds one, so leaving this out would
/// have made "a governed mutation" mean something strictly weaker than "a governed query"
/// on the one platform where the difference is fatal.
///
/// Priced against the operation's own snapshot and active dataset rather than against the
/// request's starting state, because that is the data this `WHERE` will actually read:
/// operation N+1 observes operation N's effects, so a shared pre-flight estimate would be
/// pricing a store that no longer exists.
///
/// The refusal is a [`TrippedGovernor::Refused`] carrying the estimate — never a
/// consumption it did not measure — latched through [`GovernorState::record_trip`] so a
/// stop signal that was already firing keeps precedence and the evidence names one governor
/// rather than two.
fn admit_where(
    pattern: &purrdf_sparql_algebra::GraphPattern,
    snap: &RdfDataset,
    active_dataset: &ActiveDataset<purrdf_core::TermId>,
    governors: Option<&Arc<GovernorState>>,
    relations: &crate::property_fn::PropertyFunctionRegistry,
) -> Result<(), UpdateAbort> {
    let Some(state) = governors else {
        return Ok(());
    };
    let dimension = ResourceDimension::IntermediateCells;
    if !state.is_engaged_in(dimension) {
        return Ok(());
    }
    let mut survey = crate::bgp::PlanSurvey::default();
    crate::bgp::survey_pattern_plans(
        snap,
        active_dataset,
        purrdf_core::GraphMatch::Default,
        pattern,
        // A call node CAN appear here: an UPDATE's `WHERE` is a triple-pattern
        // context exactly like a query's (`delete_insert` already feasibility-orders
        // `pattern` against this same registry through `plan_where_pattern` before
        // this survey runs), so the survey must price the call the same way a
        // governed `SELECT`'s admission does — `relations` is `cfg.options
        // .property_functions`, [`PropertyFunctionRegistry::EMPTY`] on a request
        // that configured no registry.
        relations,
        &mut survey,
    )
    .map_err(|e| {
        RdfDiagnostic::error(
            crate::engine::eval_diagnostic_code(&e, "native-sparql-update-eval"),
            e.to_string(),
        )
    })?;
    let estimate = survey.peak_cells();
    let limit = state.limits().get(dimension);
    if estimate <= limit {
        return Ok(());
    }
    Err(UpdateAbort::Tripped(state.record_trip(
        TrippedGovernor::Refused {
            dimension,
            limit,
            estimate,
        },
    )))
}

// ── LOAD ─────────────────────────────────────────────────────────────────────

/// `LOAD [SILENT] <iri> [INTO GRAPH <iri>]`.
///
/// # The stop signal is polled before the host is asked for anything
///
/// This is the one place a nominally governed engine hands control to the host and blocks
/// there: [`GraphResolver::resolve`] fetches and parses a document, and how long that takes
/// is not the evaluator's to bound. A signal only the evaluator polls is unpollable for
/// exactly as long as the evaluator is not running, which is precisely the window this call
/// occupies — so a cancelled or expired request must be stopped *before* the request is
/// issued, not noticed once it has returned. This is the same rule the `SERVICE` federation
/// seam follows.
///
/// The fetch is also charged as a host request before it is issued (see
/// [`charge_host_fetch`]), which is the only ceiling that can act on a `LOAD` in advance:
/// the ingest is charged per quad, and how many quads there are is the host's answer, not
/// something the request could have declared.
///
/// `SILENT` does **not** launder any of that into a no-op success. `SILENT` is a statement
/// about the *source* — an unreachable or unparseable document is not a request failure —
/// and it says nothing about the caller's budget. Swallowing a governor here would report a
/// request as fully applied when a ceiling had in fact stopped it, which is the one outcome
/// a governor exists to make impossible.
fn load(
    silent: bool,
    source: &str,
    destination: &GraphTarget,
    m: &mut MutableDataset,
    resolver: Option<&dyn GraphResolver>,
    governors: Option<&Arc<GovernorState>>,
) -> Result<(), UpdateAbort> {
    let Some(resolver) = resolver else {
        if silent {
            return Ok(());
        }
        return Err(UpdateAbort::Failed(RdfDiagnostic::error(
            "native-sparql-load-no-resolver",
            format!("LOAD <{source}> needs a GraphResolver host seam, none was provided"),
        )));
    };
    check_stop(governors)?;
    charge_host_fetch(governors)?;
    let stop = governors.and_then(|state| state.stop_signal().map(Arc::as_ref));
    let resolved = resolver.resolve(GraphResolveRequest { iri: source, stop });
    let post_return_trip = check_stop(governors).err();
    let loaded = match resolved {
        Ok(ds) => {
            if let Some(tripped) = post_return_trip {
                return Err(tripped);
            }
            ds
        }
        Err(e) => {
            if silent {
                if let Some(tripped) = post_return_trip {
                    return Err(tripped);
                }
                return Ok(());
            }
            return Err(UpdateAbort::Failed(e));
        }
    };

    // Re-key each loaded quad's graph to the destination (Default → None,
    // Named(g) → Some(g)). Enumerate the loaded dataset in value space.
    let dest = graph_target_value(destination)?;
    let view = MutableDataset::new(loaded);
    let quads = view.quads_for_pattern(None, None, None, GraphMatchValue::Any);
    // The document's size is the host's to choose, not the request's, so this is the one
    // mutation whose magnitude the caller could not have read off the request text. It is
    // charged before a single quad of it lands.
    charge_mutations(governors, quads.len())?;
    for q in quads {
        m.insert(rekey_graph(q, dest.as_ref()))
            .map_err(|e| iri_abort(&e))?;
    }
    Ok(())
}

// ── CLEAR / DROP ─────────────────────────────────────────────────────────────

/// Remove every quad of `target` from `m` (CLEAR ≡ DROP — see module docs).
///
/// `CLEAR ALL` is the cheapest sentence in SPARQL to write and the most expensive to
/// execute — its cost is the whole store — so it is charged per removed quad like every
/// other mutation.
fn clear_target(
    target: &GraphTarget,
    m: &mut MutableDataset,
    governors: Option<&Arc<GovernorState>>,
) -> Result<(), UpdateAbort> {
    let quads = quads_of_target(target, m);
    charge_mutations(governors, quads.len())?;
    for q in &quads {
        m.remove(q);
    }
    Ok(())
}

// ── ADD / MOVE / COPY ────────────────────────────────────────────────────────

/// `ADD <source> TO <dest>`: insert source quads re-keyed to dest; dest is NOT
/// cleared and source is NOT removed.
fn graph_op_add(
    source: &GraphTarget,
    destination: &GraphTarget,
    m: &mut MutableDataset,
    governors: Option<&Arc<GovernorState>>,
) -> Result<(), UpdateAbort> {
    // SPARQL §3.2.5: ADD where source ≡ destination is a no-op.
    if source == destination {
        return Ok(());
    }
    let src = quads_of_target(source, m);
    let dest = graph_target_value(destination)?;
    charge_mutations(governors, src.len())?;
    for q in src {
        m.insert(rekey_graph(q, dest.as_ref()))
            .map_err(|e| iri_abort(&e))?;
    }
    Ok(())
}

/// `COPY <source> TO <dest>`: clear dest, then insert source quads re-keyed to dest.
fn graph_op_copy(
    source: &GraphTarget,
    destination: &GraphTarget,
    m: &mut MutableDataset,
    governors: Option<&Arc<GovernorState>>,
) -> Result<(), UpdateAbort> {
    // SPARQL §3.2.4: COPY where source ≡ destination is a no-op.
    if source == destination {
        return Ok(());
    }
    let dest = graph_target_value(destination)?;
    let src = quads_of_target(source, m);
    // The destination clear charges its own removals; this charges the copy.
    clear_target(destination, m, governors)?;
    charge_mutations(governors, src.len())?;
    for q in src {
        m.insert(rekey_graph(q, dest.as_ref()))
            .map_err(|e| iri_abort(&e))?;
    }
    Ok(())
}

/// `MOVE <source> TO <dest>`: clear dest, insert source quads re-keyed to dest, then
/// remove the source quads.
fn graph_op_move(
    source: &GraphTarget,
    destination: &GraphTarget,
    m: &mut MutableDataset,
    governors: Option<&Arc<GovernorState>>,
) -> Result<(), UpdateAbort> {
    // SPARQL §3.2.6: MOVE where source ≡ destination is a no-op. This guard is also
    // a correctness requirement, not just an optimization: with source == dest the
    // trailing source-removal below would re-suppress the just-inserted quads and
    // empty the graph.
    if source == destination {
        return Ok(());
    }
    let dest = graph_target_value(destination)?;
    let src = quads_of_target(source, m);
    // The destination clear charges its own removals; a MOVE then touches every source
    // quad twice — once to write it at the destination, once to remove it from the source
    // — and is charged for both, because both are mutations the store performs.
    clear_target(destination, m, governors)?;
    charge_mutations(governors, src.len().saturating_mul(2))?;
    for q in &src {
        m.insert(rekey_graph(q.clone(), dest.as_ref()))
            .map_err(|e| iri_abort(&e))?;
    }
    for q in &src {
        m.remove(q);
    }
    Ok(())
}

// ── shared helpers ───────────────────────────────────────────────────────────

/// Instantiate a **variable-free** `DATA` quad (`INSERT DATA` / `DELETE DATA`) into a
/// concrete [`QuadValues`] with no dataset/snapshot. `None` if the triple is
/// positionally ill-formed (§16.2) or — a parser-invariant guard — any position holds
/// a variable. The graph slot is the explicit `GRAPH g { … }` wrapper, else the
/// default graph (DATA has no `WITH`). Blanks mint from the shared `counter`.
fn instantiate_ground_quad(
    qp: &QuadPattern,
    blanks: &mut DetHashMap<String, String>,
    counter: &mut u64,
) -> Option<QuadValues> {
    let s = instantiate_ground_term(&qp.triple.subject, blanks, counter)?;
    let p = match &qp.triple.predicate {
        NamedNodePattern::NamedNode(n) => named_node_to_value(n),
        NamedNodePattern::Variable(_) => return None,
    };
    let o = instantiate_ground_term(&qp.triple.object, blanks, counter)?;
    if positionally_ill_formed(&s, &p) {
        return None;
    }
    let g = match &qp.graph {
        Some(NamedNodePattern::NamedNode(n)) => Some(named_node_to_value(n)),
        Some(NamedNodePattern::Variable(_)) => return None,
        None => None,
    };
    Some(QuadValues { s, p, o, g })
}

/// Instantiate one solution-driven `QuadPattern` (subject/pred/object + optional
/// graph) into a concrete [`QuadValues`], with a `default_graph` (the WITH graph) used
/// when the pattern's own graph slot is `None`. `None` if any position holds an unbound
/// variable, or the result is positionally ill-formed (literal subject / non-IRI
/// predicate), or the graph slot is a variable bound to a non-IRI.
fn instantiate_quad_with_default(
    qp: &QuadPattern,
    row: &Solution,
    schema: &VarSchema,
    blanks: &mut DetHashMap<String, String>,
    ctx: &mut EvalCtx<'_>,
    default_graph: Option<&TermValue>,
) -> Option<QuadValues> {
    let s = instantiate_term(&qp.triple.subject, row, schema, blanks, ctx)?;
    let p = instantiate_predicate(&qp.triple.predicate, row, schema, ctx)?;
    let o = instantiate_term(&qp.triple.object, row, schema, blanks, ctx)?;

    // Positional validity (§16.2 / template rules): a literal subject or a non-IRI
    // predicate is ill-formed → skip the quad (do not error).
    if positionally_ill_formed(&s, &p) {
        return None;
    }

    // Graph slot: explicit pattern graph → else the WITH default → else None.
    let g = match &qp.graph {
        Some(NamedNodePattern::NamedNode(n)) => Some(named_node_to_value(n)),
        Some(NamedNodePattern::Variable(v)) => {
            let term = schema.index_of(v).and_then(|c| row[c])?;
            let value = ctx.scratch.value_of(ctx.dataset, term);
            // A graph name must be an IRI; a non-IRI binding makes the quad
            // ill-formed → skip.
            if !matches!(value, TermValue::Iri(_)) {
                return None;
            }
            Some(value)
        }
        None => default_graph.cloned(),
    };

    Some(QuadValues { s, p, o, g })
}

/// Re-key a quad's graph slot to `dest` (`None` = default graph).
fn rekey_graph(q: QuadValues, dest: Option<&TermValue>) -> QuadValues {
    QuadValues {
        s: q.s,
        p: q.p,
        o: q.o,
        g: dest.cloned(),
    }
}

/// The destination graph VALUE of a graph target (`Default` → `None`, `Named` → the
/// IRI value). Only `Default`/`Named` are valid as a re-key destination (LOAD's
/// destination and ADD/MOVE/COPY operands are `GraphOrDefault`); a `NamedGraphs`/`All`
/// target is meaningless as a single destination — the parser never produces it in
/// these positions, so reaching it is a hard error (no silent coercion to default).
fn graph_target_value(target: &GraphTarget) -> Result<Option<TermValue>, RdfDiagnostic> {
    match target {
        GraphTarget::Default => Ok(None),
        GraphTarget::Named(n) => Ok(Some(named_node_to_value(n))),
        GraphTarget::NamedGraphs | GraphTarget::All => Err(RdfDiagnostic::error(
            "native-sparql-update-bad-destination",
            "an ADD/MOVE/COPY/LOAD destination must be DEFAULT or a single named GRAPH, \
             not NAMED or ALL",
        )),
    }
}

/// Every effective quad of a graph target, as owned value-quads.
///
/// `Default` → the default graph; `Named(g)` → that one named graph; `NamedGraphs`
/// → all named graphs (every quad whose graph slot is `Some`); `All` → every quad.
fn quads_of_target(target: &GraphTarget, m: &MutableDataset) -> Vec<QuadValues> {
    match target {
        GraphTarget::Default => m.quads_for_pattern(None, None, None, GraphMatchValue::Default),
        GraphTarget::Named(n) => {
            let g = named_node_to_value(n);
            m.quads_for_pattern(None, None, None, GraphMatchValue::Named(&g))
        }
        GraphTarget::All => m.quads_for_pattern(None, None, None, GraphMatchValue::Any),
        GraphTarget::NamedGraphs => m
            .quads_for_pattern(None, None, None, GraphMatchValue::Any)
            .into_iter()
            .filter(|q| q.g.is_some())
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use purrdf_core::{RdfDatasetBuilder, RdfLiteral};
    use purrdf_sparql_algebra::SparqlParser;

    const EX: &str = "http://ex/";

    fn iri(local: &str) -> TermValue {
        TermValue::Iri(format!("{EX}{local}"))
    }

    fn parse(text: &str) -> Update {
        SparqlParser::new()
            .parse_update(&format!("PREFIX ex: <{EX}>\n{text}"))
            .expect("update parses")
    }

    /// A fresh mutable dataset over the given default-graph (s,p,o) IRI triples.
    fn mut_with(triples: &[(&str, &str, &str)]) -> MutableDataset {
        let mut b = RdfDatasetBuilder::new();
        for (s, p, o) in triples {
            let s = b.intern_iri(&format!("{EX}{s}"));
            let p = b.intern_iri(&format!("{EX}{p}"));
            let o = b.intern_iri(&format!("{EX}{o}"));
            b.push_quad(s, p, o, None);
        }
        MutableDataset::new(b.freeze().expect("freeze base"))
    }

    /// The effective quads as a comparable set of value tuples.
    fn quad_set(m: &MutableDataset) -> std::collections::BTreeSet<String> {
        m.quads_for_pattern(None, None, None, GraphMatchValue::Any)
            .iter()
            .map(|q| format!("{:?}|{:?}|{:?}|{:?}", q.s, q.p, q.o, q.g))
            .collect()
    }

    /// The ungoverned WHERE-evaluation config every test in this module runs under: the
    /// governed surface is exercised from the public API (`tests/governed_update.rs`),
    /// because that is the only vantage a consumer has on it.
    fn ungoverned(order_cache: &BgpOrderCache) -> UpdateEvalConfig<'_> {
        UpdateEvalConfig {
            standpoint_predicates: None,
            order_cache,
            governors: None,
            options: QueryOptions::EMPTY,
        }
    }

    /// The diagnostic code of a failed request.
    ///
    /// A governor trip has no code — it is a [`TrippedGovernor`], not a diagnostic — so
    /// reaching that arm from an ungoverned request is itself the failure.
    fn failure_code(abort: UpdateAbort) -> String {
        match abort {
            UpdateAbort::Failed(diagnostic) => diagnostic.code,
            UpdateAbort::Tripped(tripped) => panic!(
                "an ungoverned request cannot trip, yet it reported {}",
                tripped.label()
            ),
        }
    }

    fn run(text: &str, m: &mut MutableDataset) {
        let cache = BgpOrderCache::default();
        let cfg = ungoverned(&cache);
        eval_update(&parse(text), m, None, &cfg).expect("update applies");
    }

    #[test]
    fn insert_data_adds_quad() {
        let mut m = mut_with(&[]);
        run("INSERT DATA { ex:a ex:p ex:b }", &mut m);
        let frozen = m.freeze().expect("freeze");
        assert_eq!(frozen.quad_count(), 1);
        assert!(frozen.term_id_by_value(&iri("a")).is_some());
    }

    #[test]
    fn insert_data_blank_node_mints_a_blank() {
        let mut m = mut_with(&[]);
        run("INSERT DATA { _:x ex:p ex:b . _:x ex:q ex:c }", &mut m);
        let frozen = m.freeze().expect("freeze");
        // Two quads, sharing ONE minted blank subject (co-reference within the op).
        assert_eq!(frozen.quad_count(), 2);
        let mut blanks = std::collections::BTreeSet::new();
        for q in frozen.quads() {
            if let purrdf_core::TermRef::Blank { label, .. } = frozen.resolve(q.s) {
                blanks.insert(label.to_owned());
            }
        }
        assert_eq!(blanks.len(), 1, "the two quads share one minted blank");
    }

    #[test]
    fn delete_data_removes_quad() {
        let mut m = mut_with(&[("a", "p", "b"), ("a", "p", "c")]);
        run("DELETE DATA { ex:a ex:p ex:b }", &mut m);
        let set = quad_set(&m);
        assert_eq!(set.len(), 1);
        assert!(!m.contains(&QuadValues::triple(iri("a"), iri("p"), iri("b"))));
        assert!(m.contains(&QuadValues::triple(iri("a"), iri("p"), iri("c"))));
    }

    #[test]
    fn delete_where_removes_matches() {
        let mut m = mut_with(&[("a", "p", "b"), ("a", "p", "c"), ("a", "q", "d")]);
        run("DELETE WHERE { ?s ex:p ?o }", &mut m);
        // Only the two ex:p quads go; the ex:q quad survives.
        assert_eq!(quad_set(&m).len(), 1);
        assert!(m.contains(&QuadValues::triple(iri("a"), iri("q"), iri("d"))));
    }

    #[test]
    fn delete_insert_modify_round_trips_a_where_bound_value() {
        // The inserted quad's OBJECT is a WHERE-bound value (?o). It must survive the
        // snapshot → mutable round-trip (value-space resolution).
        let mut m = mut_with(&[("a", "p", "b")]);
        run(
            "DELETE { ?s ex:p ?o } INSERT { ?s ex:q ?o } WHERE { ?s ex:p ?o }",
            &mut m,
        );
        // (a,p,b) gone, (a,q,b) present — and b is the WHERE-bound object value.
        assert!(!m.contains(&QuadValues::triple(iri("a"), iri("p"), iri("b"))));
        assert!(m.contains(&QuadValues::triple(iri("a"), iri("q"), iri("b"))));
    }

    #[test]
    fn insert_only_modify_keeps_source() {
        let mut m = mut_with(&[("a", "p", "b")]);
        run("INSERT { ?s ex:q ?o } WHERE { ?s ex:p ?o }", &mut m);
        assert!(m.contains(&QuadValues::triple(iri("a"), iri("p"), iri("b"))));
        assert!(m.contains(&QuadValues::triple(iri("a"), iri("q"), iri("b"))));
    }

    #[test]
    fn clear_default_empties_default_graph() {
        let mut m = mut_with(&[("a", "p", "b"), ("a", "p", "c")]);
        run("CLEAR DEFAULT", &mut m);
        assert!(quad_set(&m).is_empty());
    }

    #[test]
    fn drop_named_graph_removes_its_quads() {
        // A base with a default-graph quad and a named-graph quad.
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri(&format!("{EX}a"));
        let p = b.intern_iri(&format!("{EX}p"));
        let o = b.intern_iri(&format!("{EX}b"));
        let g = b.intern_iri(&format!("{EX}g"));
        b.push_quad(s, p, o, None);
        b.push_quad(s, p, o, Some(g));
        let mut m = MutableDataset::new(b.freeze().expect("freeze"));

        run("DROP GRAPH ex:g", &mut m);
        // The named-graph quad is gone; the default-graph quad survives.
        let remaining = m.quads_for_pattern(None, None, None, GraphMatchValue::Any);
        assert_eq!(remaining.len(), 1);
        assert!(remaining[0].g.is_none());
    }

    #[test]
    fn create_graph_is_a_noop() {
        let mut m = mut_with(&[("a", "p", "b")]);
        run("CREATE GRAPH ex:g", &mut m);
        // No change — graph existence is implicit.
        assert_eq!(quad_set(&m).len(), 1);
    }

    #[test]
    fn add_copies_source_into_dest_keeping_both() {
        // ADD default TO GRAPH ex:g — default-graph quads are copied into ex:g and
        // the default graph is untouched.
        let mut m = mut_with(&[("a", "p", "b")]);
        run("ADD DEFAULT TO GRAPH ex:g", &mut m);
        let all = m.quads_for_pattern(None, None, None, GraphMatchValue::Any);
        assert_eq!(all.len(), 2, "default kept + named copy added");
        assert_eq!(all.iter().filter(|q| q.g.is_none()).count(), 1);
        assert_eq!(all.iter().filter(|q| q.g == Some(iri("g"))).count(), 1);
    }

    #[test]
    fn move_clears_source_after_copy() {
        let mut m = mut_with(&[("a", "p", "b")]);
        run("MOVE DEFAULT TO GRAPH ex:g", &mut m);
        let all = m.quads_for_pattern(None, None, None, GraphMatchValue::Any);
        assert_eq!(all.len(), 1, "source emptied, dest has the one quad");
        assert_eq!(all[0].g, Some(iri("g")));
    }

    #[test]
    fn copy_replaces_dest_then_fills_it() {
        // Seed ex:g with a stale quad; COPY default → ex:g must clear ex:g first.
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri(&format!("{EX}a"));
        let p = b.intern_iri(&format!("{EX}p"));
        let o = b.intern_iri(&format!("{EX}b"));
        let stale = b.intern_iri(&format!("{EX}stale"));
        let g = b.intern_iri(&format!("{EX}g"));
        b.push_quad(s, p, o, None); // default (a,p,b)
        b.push_quad(stale, p, o, Some(g)); // ex:g (stale,p,b)
        let mut m = MutableDataset::new(b.freeze().expect("freeze"));

        run("COPY DEFAULT TO GRAPH ex:g", &mut m);
        let in_g: Vec<_> = m
            .quads_for_pattern(None, None, None, GraphMatchValue::Named(&iri("g")))
            .into_iter()
            .collect();
        assert_eq!(in_g.len(), 1, "dest cleared then filled from source");
        assert_eq!(in_g[0].s, iri("a"), "stale quad gone, source quad present");
    }

    #[test]
    fn move_self_to_self_preserves_the_graph() {
        // MOVE GRAPH ex:g TO GRAPH ex:g is a no-op (SPARQL §3.2.6). Without the
        // same-graph guard the suppression-delta double-remove would empty ex:g.
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri(&format!("{EX}a"));
        let p = b.intern_iri(&format!("{EX}p"));
        let o = b.intern_iri(&format!("{EX}b"));
        let g = b.intern_iri(&format!("{EX}g"));
        b.push_quad(s, p, o, Some(g));
        let mut m = MutableDataset::new(b.freeze().expect("freeze"));

        run("MOVE GRAPH ex:g TO GRAPH ex:g", &mut m);
        let in_g = m.quads_for_pattern(None, None, None, GraphMatchValue::Named(&iri("g")));
        assert_eq!(in_g.len(), 1, "self-MOVE preserves the graph's quad");
        assert_eq!(in_g[0].s, iri("a"));

        // The same guard makes self-COPY and self-ADD no-ops too.
        run("COPY GRAPH ex:g TO GRAPH ex:g", &mut m);
        run("ADD GRAPH ex:g TO GRAPH ex:g", &mut m);
        let still = m.quads_for_pattern(None, None, None, GraphMatchValue::Named(&iri("g")));
        assert_eq!(still.len(), 1, "self COPY/ADD leave the graph unchanged");
    }

    #[test]
    fn graph_op_to_named_or_all_destination_is_a_hard_error() {
        // The parser only ever produces DEFAULT/GRAPH destinations, but if a NAMED/ALL
        // destination ever reaches a single-graph re-key it is a hard error, not a
        // silent coercion to the default graph.
        let mut m = mut_with(&[("a", "p", "b")]);
        let upd = Update {
            operations: vec![GraphUpdateOperation::Move {
                silent: false,
                source: GraphTarget::Default,
                destination: GraphTarget::All,
            }],
            base_iri: None,
            version: None,
        };
        let cache = BgpOrderCache::default();
        let cfg = ungoverned(&cache);
        let code = failure_code(eval_update(&upd, &mut m, None, &cfg).unwrap_err());
        assert_eq!(code, "native-sparql-update-bad-destination");
        // The base is untouched (the error aborts before any mutation lands here, and
        // the engine seam's branch/freeze guarantees atomicity at the request level).
        assert_eq!(quad_set(&m).len(), 1);
    }

    // ── VERSION admission ───────────────────────────────────────────────────

    #[test]
    fn unrecognized_version_refused_and_leaves_dataset_unchanged() {
        let mut m = mut_with(&[("a", "p", "b")]);
        let before = quad_set(&m);
        let cache = BgpOrderCache::default();
        let cfg = ungoverned(&cache);
        let code = failure_code(
            eval_update(
                &parse("VERSION \"9.9\" INSERT DATA { ex:x ex:y ex:z }"),
                &mut m,
                None,
                &cfg,
            )
            .unwrap_err(),
        );
        assert_eq!(code, "native-sparql-update-eval");
        // The chokepoint runs before the blank-mint counter is even initialized, let
        // alone before any operation applies — the store must be exactly what it was.
        assert_eq!(quad_set(&m), before, "no mutation applied");
    }

    #[test]
    fn recognized_versions_still_apply() {
        for version in ["1.2", "1.2-basic"] {
            let mut m = mut_with(&[]);
            let cache = BgpOrderCache::default();
            let cfg = ungoverned(&cache);
            eval_update(
                &parse(&format!(
                    "VERSION \"{version}\" INSERT DATA {{ ex:x ex:y ex:z }}"
                )),
                &mut m,
                None,
                &cfg,
            )
            .unwrap_or_else(|_| panic!("VERSION {version:?} must apply"));
            assert_eq!(quad_set(&m).len(), 1);
        }
    }

    #[test]
    fn basic_profile_gate_refuses_triple_term_and_leaves_dataset_unchanged() {
        let mut m = mut_with(&[("a", "p", "b")]);
        let before = quad_set(&m);
        let cache = BgpOrderCache::default();
        let cfg = ungoverned(&cache);
        let code = failure_code(
            eval_update(
                &parse(
                    "VERSION \"1.2-basic\" INSERT DATA { ex:x ex:reifies <<( ex:a ex:p ex:b )>> }",
                ),
                &mut m,
                None,
                &cfg,
            )
            .unwrap_err(),
        );
        assert_eq!(code, "native-sparql-update-eval");
        // Same admission chokepoint as the unrecognized-VERSION case: it runs before any
        // operation applies, so the store is exactly what it was.
        assert_eq!(quad_set(&m), before, "no mutation applied");
    }

    #[test]
    fn basic_profile_gate_admits_a_within_profile_update() {
        let mut m = mut_with(&[]);
        let cache = BgpOrderCache::default();
        let cfg = ungoverned(&cache);
        eval_update(
            &parse("VERSION \"1.2-basic\" INSERT DATA { ex:x ex:y ex:z }"),
            &mut m,
            None,
            &cfg,
        )
        .expect("a within-profile update must still apply");
        assert_eq!(quad_set(&m).len(), 1);
    }

    // ── LOAD ─────────────────────────────────────────────────────────────────

    struct TestResolver {
        ds: Arc<RdfDataset>,
    }
    impl GraphResolver for TestResolver {
        fn resolve(
            &self,
            _request: GraphResolveRequest<'_>,
        ) -> Result<Arc<RdfDataset>, RdfDiagnostic> {
            Ok(self.ds.clone())
        }
    }

    fn loadable() -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri(&format!("{EX}loaded"));
        let p = b.intern_iri(&format!("{EX}p"));
        let o = b.intern_literal(RdfLiteral::simple("v"));
        b.push_quad(s, p, o, None);
        b.freeze().expect("freeze loadable")
    }

    #[test]
    fn load_with_resolver_imports_into_default_graph() {
        let mut m = mut_with(&[]);
        let resolver = TestResolver { ds: loadable() };
        let cache = BgpOrderCache::default();
        let cfg = ungoverned(&cache);
        eval_update(&parse("LOAD ex:doc"), &mut m, Some(&resolver), &cfg).expect("load");
        let frozen = m.freeze().expect("freeze");
        assert_eq!(frozen.quad_count(), 1);
        assert!(frozen.term_id_by_value(&iri("loaded")).is_some());
    }

    #[test]
    fn load_into_named_graph_rekeys_to_destination() {
        let mut m = mut_with(&[]);
        let resolver = TestResolver { ds: loadable() };
        let cache = BgpOrderCache::default();
        let cfg = ungoverned(&cache);
        eval_update(
            &parse("LOAD ex:doc INTO GRAPH ex:g"),
            &mut m,
            Some(&resolver),
            &cfg,
        )
        .expect("load into");
        let all = m.quads_for_pattern(None, None, None, GraphMatchValue::Any);
        assert_eq!(all.len(), 1);
        assert_eq!(
            all[0].g,
            Some(iri("g")),
            "re-keyed to the destination graph"
        );
    }

    #[test]
    fn load_without_resolver_is_a_hard_error() {
        let mut m = mut_with(&[]);
        let cache = BgpOrderCache::default();
        let cfg = ungoverned(&cache);
        let code =
            failure_code(eval_update(&parse("LOAD ex:doc"), &mut m, None, &cfg).unwrap_err());
        assert_eq!(code, "native-sparql-load-no-resolver");
    }

    #[test]
    fn load_silent_without_resolver_is_a_noop_ok() {
        let mut m = mut_with(&[("a", "p", "b")]);
        let cache = BgpOrderCache::default();
        let cfg = ungoverned(&cache);
        eval_update(&parse("LOAD SILENT ex:doc"), &mut m, None, &cfg).expect("silent load no-ops");
        assert_eq!(quad_set(&m).len(), 1, "unchanged");
    }

    // ── USING ─────────────────────────────────────────────────────────────────

    /// A base with the same (a,p,b) triple in the default graph and in ex:g, plus a
    /// decoy (a,p,c) only in ex:g.
    fn base_default_and_named() -> MutableDataset {
        let mut b = RdfDatasetBuilder::new();
        let a = b.intern_iri(&format!("{EX}a"));
        let p = b.intern_iri(&format!("{EX}p"));
        let bb = b.intern_iri(&format!("{EX}b"));
        let cc = b.intern_iri(&format!("{EX}c"));
        let g = b.intern_iri(&format!("{EX}g"));
        b.push_quad(a, p, bb, None); // default (a,p,b)
        b.push_quad(a, p, bb, Some(g)); // ex:g (a,p,b)
        b.push_quad(a, p, cc, Some(g)); // ex:g (a,p,c)
        MutableDataset::new(b.freeze().expect("freeze"))
    }

    #[test]
    fn using_scopes_where_to_the_named_graph() {
        // USING ex:g folds ex:g into the WHERE default graph: the DELETE template
        // (default-graph target) removes whatever the WHERE bound from ex:g. The WHERE
        // sees ex:g's (a,p,b)+(a,p,c), so the default-graph (a,p,b) is deleted but the
        // default graph's other triples (none here) and ex:g itself are not the target.
        let mut m = base_default_and_named();
        // DELETE the default-graph quad whose object the WHERE bound from ex:g.
        run(
            "DELETE { ex:a ex:p ?o } USING ex:g WHERE { ex:a ex:p ?o }",
            &mut m,
        );
        // The WHERE matched ?o ∈ {b, c} in ex:g; the DELETE removed (a,p,b) and (a,p,c)
        // from the DEFAULT graph. Default had only (a,p,b) → gone; (a,p,c) wasn't there.
        assert!(!m.contains(&QuadValues::triple(iri("a"), iri("p"), iri("b"))));
        // ex:g is untouched (USING only scopes the WHERE, not the delete target).
        let in_g = m.quads_for_pattern(None, None, None, GraphMatchValue::Named(&iri("g")));
        assert_eq!(
            in_g.len(),
            2,
            "ex:g is the WHERE source, not the delete target"
        );
    }

    #[test]
    fn using_named_restricts_graph_var_in_where() {
        // USING NAMED ex:g makes ex:g (and only ex:g) addressable by GRAPH ?g in the
        // WHERE; the default graph of the WHERE is empty (no plain USING).
        let mut m = base_default_and_named();
        run(
            "INSERT { ex:hit ex:in ?g } USING NAMED ex:g WHERE { GRAPH ?g { ex:a ex:p ex:b } }",
            &mut m,
        );
        // ?g bound to ex:g (the only named graph in the USING NAMED set) → one insert.
        assert!(m.contains(&QuadValues::triple(iri("hit"), iri("in"), iri("g"))));
    }

    #[test]
    fn using_nonexistent_graph_matches_nothing() {
        // USING <absent> → the WHERE default graph is empty → no solutions → no-op,
        // not an error.
        let mut m = mut_with(&[("a", "p", "b")]);
        run(
            "DELETE { ex:a ex:p ?o } USING ex:absent WHERE { ex:a ex:p ?o }",
            &mut m,
        );
        // Nothing matched in the empty WHERE dataset → the base is unchanged.
        assert!(m.contains(&QuadValues::triple(iri("a"), iri("p"), iri("b"))));
    }

    #[test]
    fn with_scopes_where_and_targets_the_named_graph() {
        // WITH ex:g: the WHERE matches in ex:g, and the delete/insert quads (no
        // explicit graph) target ex:g too. Seed a quad in ex:g and a decoy in the
        // default graph with the same s/p/o; only the ex:g one is rewritten.
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri(&format!("{EX}a"));
        let p = b.intern_iri(&format!("{EX}p"));
        let o = b.intern_iri(&format!("{EX}b"));
        let g = b.intern_iri(&format!("{EX}g"));
        b.push_quad(s, p, o, None); // default-graph decoy (a,p,b)
        b.push_quad(s, p, o, Some(g)); // ex:g (a,p,b)
        let mut m = MutableDataset::new(b.freeze().expect("freeze"));

        run(
            "WITH ex:g DELETE { ?s ex:p ?o } INSERT { ?s ex:q ?o } WHERE { ?s ex:p ?o }",
            &mut m,
        );

        // The default-graph decoy is untouched (WITH scoped the WHERE to ex:g).
        assert!(m.contains(&QuadValues::triple(iri("a"), iri("p"), iri("b"))));
        // In ex:g: (a,p,b) gone, (a,q,b) present, both keyed to ex:g (the WITH graph).
        assert!(m.contains(&QuadValues::quad(iri("a"), iri("q"), iri("b"), iri("g"))));
        let in_g = m.quads_for_pattern(None, None, None, GraphMatchValue::Named(&iri("g")));
        assert_eq!(in_g.len(), 1);
        assert_eq!(in_g[0].p, iri("q"));
    }

    #[test]
    fn later_operation_sees_earlier_effect() {
        // INSERT then DELETE WHERE in one request: the DELETE must see the inserted
        // quad (operations apply in order over the shared `m`).
        let mut m = mut_with(&[]);
        run(
            "INSERT DATA { ex:a ex:p ex:b } ; DELETE WHERE { ?s ex:p ?o }",
            &mut m,
        );
        assert!(
            quad_set(&m).is_empty(),
            "the second op saw the first's insert"
        );
    }

    // ── property-function / custom-aggregate admission (an UPDATE's WHERE is a
    //    triple-pattern context exactly like a query's — see `delete_insert`'s
    //    `plan_where_pattern` call) ────────────────────────────────────────────

    /// An unregistered `AGG(<iri>, …)` reached through an UPDATE's `WHERE` (nested
    /// inside a sub-`SELECT`'s `GROUP BY`) must be refused at PREPARE time —
    /// before `m.freeze()`'s snapshot is even evaluated — under the AGGREGATE
    /// diagnostic code, and it must spend
    /// EXACTLY ZERO of a governed request's budget: every dimension `QueryGovernors
    /// ::METERED` engages is asserted at zero, not merely "the request failed",
    /// because a coarse assertion would not catch an admission failure that ran
    /// after a charge had already landed.
    #[test]
    fn unregistered_aggregate_in_update_where_is_refused_with_the_aggregate_code_and_zero_charge() {
        let mut m = mut_with(&[]);
        let before = quad_set(&m);
        let cache = BgpOrderCache::default();
        let state = Arc::new(GovernorState::new(
            &crate::governor::QueryGovernors::METERED,
        ));
        let cfg = UpdateEvalConfig {
            standpoint_predicates: None,
            order_cache: &cache,
            governors: Some(&state),
            options: QueryOptions::EMPTY,
        };
        let upd = parse(
            "INSERT { ex:x ex:p ?a } WHERE { \
                 SELECT (AGG(<http://example.org/agg#nope>, ?v) AS ?a) \
                 WHERE { ex:s ex:val ?v } GROUP BY ?v \
             }",
        );
        let code = failure_code(eval_update(&upd, &mut m, None, &cfg).unwrap_err());
        assert_eq!(code, "native-sparql-aggregate-function");
        // Nothing applied.
        assert_eq!(
            quad_set(&m),
            before,
            "an admission refusal must apply nothing"
        );
        // And nothing was CHARGED: a refusal that ran after any charge point would
        // still leave the store untouched (mutation is applied only at the very
        // end), so the zero-mutation assertion above cannot by itself distinguish
        // "refused before evaluation" from "refused after the WHERE was fully
        // evaluated, metered, and then discarded". The governor ledger can.
        for dimension in ResourceDimension::ALL {
            assert_eq!(
                state.consumed_in(dimension),
                0,
                "an admission refusal consumed {dimension:?}, so evaluation ran after all"
            );
        }
    }

    /// The regression control for the test above: an unregistered PROPERTY-FUNCTION
    /// call reached through the identical UPDATE `WHERE` position must still report
    /// the property-function code, not the aggregate code the fix above introduces.
    #[test]
    fn unregistered_property_function_in_update_where_still_reports_the_property_function_code() {
        let mut m = mut_with(&[]);
        let cache = BgpOrderCache::default();
        // A namespace-declared relation with NOTHING registered under it: the
        // specific IRI still parses as a call node (the namespace claims it), and
        // is refused as unregistered — see
        // `property_fn_eval::an_unregistered_iri_under_a_configured_namespace_is_refused_before_evaluation`
        // for the query-path twin this mirrors.
        let registry = crate::property_fn::PropertyFunctionRegistry::new();
        let options = QueryOptions {
            property_functions: &registry,
            ..QueryOptions::EMPTY
        };
        let cfg = UpdateEvalConfig {
            standpoint_predicates: None,
            order_cache: &cache,
            governors: None,
            options,
        };
        let parser_options = purrdf_sparql_algebra::ParserOptions {
            extension_fn_namespaces: vec![],
            property_fn_namespaces: vec!["http://ex/pf/".to_owned()],
            property_fn_iris: Vec::new(),
        };
        let upd = SparqlParser::new()
            .parse_update_with(
                "PREFIX ex: <http://ex/>\n\
                 INSERT { ex:x ex:p ?a } WHERE { ex:s <http://ex/pf/split> ?a }",
                &parser_options,
            )
            .expect("update parses: the namespace claims the predicate as a call node");
        let code = failure_code(eval_update(&upd, &mut m, None, &cfg).unwrap_err());
        assert_eq!(code, "native-sparql-property-function");
    }

    /// A custom aggregate that IS registered, called with the wrong argument count,
    /// through an UPDATE's `WHERE` — the arity-mismatch admission failure must
    /// carry the aggregate code too, not just the unregistered-IRI case.
    #[test]
    fn custom_aggregate_arity_mismatch_in_update_where_carries_the_aggregate_code() {
        let mut m = mut_with(&[]);
        let cache = BgpOrderCache::default();
        let mut registry = crate::agg_fn::AggregateRegistry::new();
        registry.register_statistical_aggregates("http://ex/agg#");
        let options = QueryOptions {
            aggregates: &registry,
            ..QueryOptions::EMPTY
        };
        let cfg = UpdateEvalConfig {
            standpoint_predicates: None,
            order_cache: &cache,
            governors: None,
            options,
        };
        // MEDIAN is declared `Arity::Exact(1)`; supplying two positional arguments
        // is an arity mismatch, not an unregistered IRI.
        let upd = parse(
            "INSERT { ex:x ex:p ?a } WHERE { \
                 SELECT (AGG(<http://ex/agg#MEDIAN>, ?v, ?w) AS ?a) \
                 WHERE { ex:s ex:val ?v . ex:s ex:val2 ?w } GROUP BY ?v ?w \
             }",
        );
        let code = failure_code(eval_update(&upd, &mut m, None, &cfg).unwrap_err());
        assert_eq!(code, "native-sparql-aggregate-function");
    }
}
