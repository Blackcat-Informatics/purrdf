// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The executor: each admitted unit runs independently through the evaluator.
//!
//! [`execute`] is the only stage that runs a **ranked read**, and the only stage
//! that compiles a query at all — including the one query it does not itself run,
//! the per-candidate exclusion lookup it prepares and hands back inside the
//! stream, which executes when the fusion stage asks (see
//! [`RankedStream::exclusion`](crate::RankedStream::exclusion) and the borrow
//! note on [`RankedStreamImpl`]). It runs one unit per stratum
//! through `purrdf-sparql-eval` against **the caller's dataset**, with the
//! caller's registry injected, and the query text is exactly the admitted
//! emission. A unit that fails to parse or evaluate becomes that stratum's
//! [`ProducerStatus::ExecutionFailed`] while every other stratum streams on — the
//! isolation requirement a single monolithic query could not meet.
//!
//! # The dataset is the caller's, and it is read
//!
//! A unit is ordinary SPARQL: its ranked relations are reached from predicate
//! position, but every other pattern in it is matched against the stored data
//! like any other query. The dataset is therefore a parameter, taken by reference
//! and never built here — a retrieval layer that searched a graph of its own
//! choosing would answer about data nobody asked about. `execute` is generic over
//! [`DatasetView`] rather than taking a trait object because the view has
//! associated types and is not object-safe; the evaluator entry point it calls is
//! generic for the same reason.
//!
//! # Ranks are the evaluator's emission order
//!
//! Each successful unit's solution rows become candidates in emission order, with
//! **1-based** ranks. The evaluator preserves a relation's declared emission
//! order, so the candidate set and per-stratum ranks are a pure function of the
//! plan, the registry and the dataset — a pinned plan replays identically against
//! the same data.
//!
//! That the rank is the *relation's* order, and not an artifact of how the unit
//! was assembled, rests on one stratum carrying one producer — refused at
//! registration by
//! [`register_ranked`](purrdf_sparql_eval::PropertyFunctionRegistry::register_ranked)
//! and again at the admission waist. A unit with two branches would emit their
//! concatenation, so the second relation's rank-1 row would arrive at stratum
//! rank `n+1` and decay here as though it had lost to `n` rows it never competed
//! with. With one branch there is nothing to concatenate: the stratum's rank
//! order **is** its producer's emission order, row for row.
//!
//! # A raw stream, not yet a fusion stream
//!
//! [`RankedStreamImpl`] carries `(rank, candidate, block)` only. A fusion contribution
//! depends on the fusion profile's weights and smoothing constant, which are
//! deliberately not a plan input, so the contribution is attached at `fuse` time.
//! Keeping the executor profile-free is what lets the unfused rung be consumed
//! with no fusion law in the path at all. It is not what makes it cheap: under
//! [`execute`] a stratum's rows are materialized by the evaluator before the first
//! one is read, so that rung costs what the stratum's own result costs. What makes
//! a read cheap is reading it on demand — see the last section of this header.
//!
//! Attaching it is not left to the caller to re-derive: a stream here is carried
//! into the fusion protocol by
//! [`RankedStreamAdapter`](crate::RankedStreamAdapter), the same exported bridge
//! [`search`](crate::search) composes with. Stopping here and resuming later is
//! one supported path, not a private one.
//!
//! # The witness rides the governed receipt, and one unit attests once
//!
//! A unit is run through the **governed** entry
//! (`query_prepared_governed_view`) under
//! [`QueryGovernors::UNBOUNDED`], not because anything here wants a ceiling but
//! because that is the lane whose receipt carries a
//! [`RelationWitness`](purrdf_sparql_eval::RelationWitness). A witness is
//! evidence *about* an outcome rather than an outcome, and it already travels
//! beside `GovernorEvidence` on that receipt, so asking for a second, witnessed
//! entry point would be asking the evaluator to grow a second spelling of a
//! channel it already has. `UNBOUNDED` is that engine's own documented way to
//! decline every caller-settable ceiling, so nothing a unit could do before is
//! bounded now; and the governors are built per call and dropped with it,
//! because the entry point's own contract is that governors are per call and
//! never per engine.
//!
//! What comes back is read under the **sole-witness rule**. A compiled unit
//! binds exactly one producer — the registry refuses a second producer for a
//! stratum and the admission waist re-checks it — and every argument the unit
//! places is a constant, so a conforming run invokes exactly one relation,
//! against one pinned index generation, with at most one thing to say about
//! that index's wholeness. A witness that says otherwise did not observe a
//! different stratum; it observed that the snapshot moved under the query or
//! that the registry was not the one the unit was compiled against. That
//! invalidates the **run**, so it is [`ExecutionError::InconsistentWitness`]
//! and not a per-stratum status.
//!
//! It is deliberately not [`ProducerStatus::ExecutionFailed`]. That status
//! means the producer could not run, it carries no rows, and the fusion stage
//! refuses it outright once rows have been emitted
//! ([`ProtocolError::ErrorAfterRows`]) — but the unit here *did* run and *did*
//! return rows. Reporting it that way would either throw those rows away under
//! a false label or hand fusion a receipt that contradicts what it just pulled.
//!
//! The rule is keyed on the declaration **sets** and the relation count, and
//! deliberately **not** on the invocation count. How many times a relation
//! enters host code is a property of the schedule: one evaluator lane forks a
//! child per chunk of driving rows and a `FILTER EXISTS` re-enters pattern
//! evaluation inside each, so the same query over the same data can count
//! differently without anything having changed about the index. What was
//! *declared* does not vary with the chunking, which is exactly why the witness
//! unions the declarations and merely sums the counts. A rule that tightened to
//! include the count would be a flake waiting for a bigger dataset.
//!
//! What *is* attached here is the producer's own ranked-stream contract —
//! its duplicate handling and the blocks of the candidate universe it may name
//! — carried through from the compiled unit into [`StratumStream::contract`]. That is not a fusion input
//! the way a weight is: it is the producer's declaration about its own rows, and
//! the same one-stratum-one-producer rule the ranks rest on makes it
//! unambiguous, so a stratum's contract is simply its producer's. It travels
//! with the rows rather than being looked up again at the end, for the reason
//! the plan identity does. The attestation read off the witness travels the
//! same way, in [`StratumStream::attestation`].
//!
//! # Each row says which block it was drawn from
//!
//! A restricted declaration is a promise about every row, and a consumer can only
//! hold it to the rows it reads if a row says where it came from. So every row
//! carries its block ([`RowBlock`]), from exactly one of two places and never from
//! a guess:
//!
//! * the unit's own `?block` column, where the producer's declaration named the
//!   argument position to read it from. `compile` projects that column only for
//!   such a producer, and it is found here by name;
//! * the **declaration itself**, where it names exactly one block. A producer that
//!   promised all its candidates lie in one block has already answered per row, so
//!   nothing is invented and no host repeats itself — see [`entailed_block`].
//!
//! Anything else is [`RowBlock::Undeclared`], which is the honest report and not a
//! gap: an unrestricted producer owes no block, and a producer restricted to
//! several blocks with no column to distinguish them has declared something this
//! stage cannot back. Fusion is where that is answered — it refuses a restriction
//! no row backs rather than certifying an order on it — because fusion is the
//! stage that would otherwise have used it.
//!
//! # The unit is read one row deeper than the stratum is
//!
//! [`compile`](crate::compile) emits `LIMIT depth + 1`, so a unit whose producer
//! still had rows past the planned depth hands back one more row than the
//! stratum may contribute. That row is a **probe**: it is never emitted onto the
//! stream, never ranked, and counted in exactly one place —
//! [`RankedStreamImpl::rows_materialised`], which answers what the read cost
//! rather than what the answer is made of, and which would understate the read by
//! exactly this row if it left it out. What it decides is the
//! stream's ending — [`StreamEnding::DepthReached`] when it arrived,
//! [`StreamEnding::Exhausted`] when it did not. Without it an executor could
//! only ever say `Exhausted`, which is the one ending that names no
//! stopper, uttered about a read the plan itself cut short.
//!
//! The unit's own bound leaves that slot open at every depth a plan can carry. It
//! can, because the one depth whose slot would not fit a 32-bit `LIMIT` cannot reach
//! this stage: the admission waist refuses it
//! ([`AdmissionError::DepthWithoutProbe`](crate::AdmissionError)) and the planner
//! records a derived bound past that ceiling *at* the ceiling rather than past it. A
//! read whose ending nobody could have observed must not arrive here to be reported
//! as an exhaustion.
//!
//! Below the declaration, the extra row means the *depth* stopped the read, which
//! is `DepthReached`. Past the declaration, it means the producer yielded a row it
//! promised did not exist, and [`bound_to_depth`] refuses the whole run
//! ([`ExecutionError::RowBoundBreached`]) rather than truncating to the depth
//! and calling the result exhausted. When the declaration is honest the slot
//! comes back empty and costs nothing, and `Exhausted` is then verified against
//! a read that was allowed to go one row further rather than believed on the
//! strength of a bound.
//!
//! # One class of producer cannot be asked for the probe row, and says so
//!
//! A producer that declared a depth *placement* is handed the number instead of
//! being bounded by it, and a request is not a ceiling: asking such a producer for
//! one row more than its registration allows asks it to contradict that
//! registration, which a well-built relation refuses — the shipped
//! nearest-neighbour relation refuses exactly that against its configured guard. So
//! [`compile`](crate::compile) caps the *argument* at the declaration while leaving
//! the unit's own bound one row deeper.
//!
//! At a depth that already sits on the declaration those two cap at the same place:
//! the producer is asked for exactly `depth` rows, returns at most `depth` rows, and
//! the slot past the depth can never be filled however many rows its index holds. The
//! ending is then **genuinely unobservable**, and neither of the two endings above is
//! true of it — `DepthReached` would claim the planned depth cut a read it did not,
//! and `Exhausted` would claim rows ran out when nobody could know. So it is reported
//! as its own ending, [`StreamEnding::RowBoundReached`], which names the stopper it
//! actually had: the producer's own declared row bound. It is reported only where
//! that is true — where the read returned every row it was allowed and no further row
//! could have been asked for. A producer planned below its declaration is still asked
//! for the probe and still reports `DepthReached` or `Exhausted`; one that returned
//! fewer rows than it was allowed is `Exhausted`, verified, because it stopped before
//! anything stopped it.
//!
//! A declared **zero** is read rather than obeyed, and the emitted bound is what
//! reads it: the floored depth of one is emitted one row deeper like every other
//! depth, so an empty index reports `Exhausted { rows_emitted: 0 }` as a verified
//! claim and an index that turns out to hold rows breaches the declaration
//! ([`ExecutionError::RowBoundBreached`]) exactly as a wrong declaration of any other
//! size does.
//!
//! # A unit running a caller's own text is read, and is never certified
//!
//! Everything above rests on the bound being this layer's: the probe row is evidence
//! only because [`compile`](crate::compile) rendered a bound one row past the depth
//! and nothing else could have cut the read first. A unit built through
//! [`StratumUnit::new`](crate::StratumUnit::new) runs a text a caller wrote, and this
//! layer bounds only its outside — a `LIMIT` on a sub-`SELECT` inside it decides the
//! read before the outer bound is ever consulted, and nothing this layer reads of that
//! text rules it out: the one parse it runs
//! ([`StratumUnit::new`](crate::StratumUnit::new)'s) locates the caller's prologue and
//! interprets no bound.
//!
//! So such a read is executed exactly as any other, its rows are ranked exactly as
//! any other, a row past the depth still means something further existed
//! ([`StreamEnding::DepthReached`]), and a producer that beat its own declaration is
//! still refused by name. What it never reports is [`StreamEnding::Exhausted`]: a read
//! that came back inside the unit's bound ends
//! [`StreamEnding::SuppliedQueryEnded`], which names the caller's text as the stopper
//! it actually had. Refusing to run the text instead would have been the
//! over-refusal — the seam exists so that a host can drive this executor over a query
//! of its own, and most such queries are perfectly good; what cannot be done honestly
//! is certify a completeness claim from one.
//!
//! # A stratum may be read on demand, and its receipt is taken when the reading stops
//!
//! [`execute_within`] takes a [`ReadSchedule`]. [`ReadSchedule::Materialised`] is the
//! read described so far, and it is what [`execute`] runs. Under
//! [`ReadSchedule::OnDemand`] — the schedule [`search`](crate::search) runs — every
//! unit whose prepared text is one property-function call under row-for-row
//! operators — projections, `OFFSET`-free `LIMIT`s, renaming `BIND`s and `FILTER`s
//! evaluated row by row
//! ([`PreparedQuery::is_call_read`](purrdf_sparql_eval::PreparedQuery::is_call_read)),
//! which is every unit this layer rendered, is opened as **one invocation held open**
//! ([`NativeSparqlEngine::open_call_cursor`]): the same text, the same planned depth,
//! the same bound one probe row past it, and the same arguments, admission, width
//! check and unification the governed lane gives it. Its stream then produces a row
//! each time it is pulled and none before. A fusion that certifies at its sixth rank
//! has produced six rows of a four-hundred-row plan; one that needs the four
//! hundredth goes on reading the invocation it opened — never a second invocation,
//! never a row twice — and reaches the probe row only if it asks for it. The ending
//! is decided by exactly the observations [`bound_to_depth`] makes, one row at a
//! time: the probe row's arrival, the producer running out, the producer's own
//! declared bound.
//!
//! The receipt is where a materialised read and an on-demand one differ, and it is
//! the reason the difference is sound. A materialised read's witness describes a run
//! that finished before its first row was readable. An on-demand read's invocation
//! finishes when its consumer stops, so its evidence is taken then: the stream
//! announces what the invocation attested **the instant it opened** — the generation
//! it pinned and the service level it reported — before its first row, exactly where
//! a materialised stream announces its own; and when the consumer stops, the stream
//! settles ([`RankedStream::settle`](crate::RankedStream::settle)), reading the
//! invocation's witness as it stands then under the same sole-witness rule a
//! materialised run is read under. An index that moved under the read shows as the
//! two generations it is and is refused; a service level that changed between the
//! announcement and the stop is refused too
//! ([`ProtocolError::AttestationMoved`](crate::ProtocolError::AttestationMoved)),
//! because every row was certified under the announcement. The receipt covers
//! exactly the rows the stream handed out, and the consumer checks that count
//! against the rows it pulled.
//!
//! # A producer that takes its depth is opened at the planned depth, and pays nothing for it
//!
//! A rendered unit hands a self-bounding producer — a nearest-neighbour search — the
//! planned depth (one probe row past it, within its declaration) as its `k`, and the
//! on-demand read opens it there even where the fusion will provably stop far
//! shallower. That is not a missed saving, because neither nearest-neighbour producer
//! in this workspace does work that depends on `k`. The exact scan computes one
//! distance per row of its space for any `k` of at least one — the nearest row is
//! not known until every row has been measured — and `k` only sizes the selection
//! it keeps. The graph search walks a beam of the artifact's declared `ef_search`,
//! which is part of the index identity and is never narrowed or widened to fit a
//! request, and `k` only truncates the beam's sorted result. So the search an
//! invocation performs is the same search at `k = 6` as at `k = 401`, both producers'
//! results at a smaller `k` are exactly the prefix of their results at a larger one
//! (the order is total: distance, then row), and the rows past the fusion's stop are
//! never produced because the cursor builds a row only when it is pulled. Opening at
//! a shallower `k` and continuing with a second invocation where the fusion read
//! past it would repeat that whole search for the second read — twice the work of
//! the one invocation this layer takes — and would buy the first read nothing.
//!
//! A unit whose text is anything else — a caller's own join, `ORDER BY` or dataset
//! clause — is materialised under either schedule: it is not one invocation this
//! layer can hold open. A caller's own text that *is* one call under row-for-row
//! operators is read on demand exactly as a rendered one, and ends
//! [`StreamEnding::SuppliedQueryEnded`] rather than `Exhausted` when it runs out,
//! for the reason its materialised read does. A `FILTER` it wrote over the call is
//! applied as each row is pulled, by the engine's own evaluator: a row it drops is
//! read and takes no rank, so the stream's ranks count only the rows it kept — as
//! the materialised read of the same text numbers its answer — and no `LIMIT` above
//! the `FILTER` is ever offered to the producer as a ceiling
//! ([`CallCursor`](purrdf_sparql_eval::CallCursor)).
//!
//! A failure is isolated exactly as far as it still can be — one before the
//! first row is that stratum's [`ProducerStatus::ExecutionFailed`], while one after
//! rows were merged fails the request by name, because those rows are already in the
//! answer.

use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::rc::Rc;
use std::sync::Arc;

use purrdf_core::{DatasetView, SparqlResult, TermValue};
use purrdf_sparql_eval::{
    CallCursor, CandidateDomains, DomainTag, ExtensionEnv, GovernedOutcome, InternedOutcome,
    NativeSparqlEngine, PfAttestation, PreparedExecution, PropertyFunctionRegistry, QueryGovernors,
    QueryOptions, RegistryId, RelationWitness, ServiceLevel,
};

use crate::admission::BoundMode;
use crate::compile::{
    BLOCK_NAME, CANDIDATE_NAME, CompiledRetrieval, EXCLUSION_LIMIT, ReadReach, ReadSchedule,
    StratumUnit,
};
use crate::fuse::TopK;
use crate::fusion_stream::ProducerStatus;
use crate::id::PlanId;
use crate::iri::{Iri, Term};
use crate::ranked_stream::{
    ExclusionVerdict, ProducerReceipt, ProtocolError, RowBlock, StreamContract,
};
use crate::render::{RenderError, candidate_lexical, decode_term};

/// One stratum's ranked rows, tagged with the pinned plan they descend from and
/// the contract its producer declared them under.
#[derive(Debug)]
pub struct StratumStream<'d> {
    /// The stratum the stream's ranks are within.
    pub stratum: Iri,
    /// The admitted plan the stream was compiled from.
    pub plan_id: PlanId,
    /// The row bound this stream's depth was derived for, carried from
    /// [`CompiledRetrieval::fused_bound`](crate::CompiledRetrieval).
    ///
    /// It travels with the rows for the reason [`Self::plan_id`] does, and it
    /// closes the same class of defect one stage later: the depth behind these
    /// rows is honest for one bound, and a fusion run at another would serve an
    /// answer out of a read that was never taken for it.
    /// [`fuse`](crate::fuse) reads it back through
    /// [`RankedStream::fused_bound`](crate::RankedStream::fused_bound) and refuses
    /// the mismatch by name. A caller that stops here and fuses later hands it to
    /// [`RankedStreamAdapter::with_fused_bound`](crate::RankedStreamAdapter::with_fused_bound),
    /// beside the plan identity it already hands over.
    pub fused_bound: TopK,
    /// The duplicate handling and the candidate domains the stratum's producer
    /// declared, carried from [`StratumUnit::contract`](crate::StratumUnit).
    ///
    /// It travels with the rows for the reason [`Self::plan_id`] does: the
    /// fusion stage is the consumer those two declarations were written for, and
    /// a consumer that re-fetched them from a registry at the end would be
    /// reading a fact about that registry rather than about the stream in its
    /// hand. A caller that stops here and fuses later hands this to
    /// [`RankedStreamAdapter::new`](crate::RankedStreamAdapter::new) alongside
    /// the stream, which is why it is a field of the same value.
    pub contract: StreamContract,
    /// What the index behind these rows attested: which generation answered,
    /// and whether that generation admitted to being short.
    ///
    /// Read off the governed receipt's relation witness for the run that
    /// produced these very rows, and carried here for the reason
    /// [`Self::contract`] and [`Self::plan_id`] are carried: the consumer is
    /// three stages downstream, and a consumer that asked the index again at the
    /// end would be told about the index *then* rather than about the one that
    /// answered. A generation is pinned when an index opens, so the version that
    /// served rank one is the version that served the last rank — which is only
    /// a useful fact if it travels with the rows it is true of.
    ///
    /// For a stratum read on demand it is what the invocation attested the
    /// instant it opened, before any row existed — the announcement the stream is
    /// held to when it settles ([`RankedStream::settle`](crate::RankedStream::settle)),
    /// so a read that ends under a different attestation is refused rather than
    /// reported under this one.
    ///
    /// [`PfAttestation::UNDECLARED`] is the honest answer for a unit whose
    /// relation declared neither fact, and it stays an absence all the way out:
    /// silence is never a certificate that the index was current or whole.
    pub attestation: PfAttestation,
    /// The evaluator's rows, in rank order.
    pub stream: RankedStreamImpl<'d>,
}

/// The result of running every compiled unit: the surviving streams and every
/// stratum's status.
///
/// A failed stratum appears in `statuses` as
/// [`ProducerStatus::ExecutionFailed`] and has no entry in `streams`; a surviving
/// stratum read materialised appears in both. A stratum read on demand
/// ([`ReadSchedule::OnDemand`]) appears in `streams` only, because how its read
/// ends is decided by how far it is read, and its stream's receipt is its status.
/// Nothing reduces the statuses to one flag.
#[derive(Debug)]
pub struct ExecutionResult<'d> {
    /// One stream per stratum that ran, ordered as compiled.
    pub streams: Vec<StratumStream<'d>>,
    /// Every stratum's own terminal status.
    pub statuses: HashMap<Iri, ProducerStatus>,
}

/// A whole-execution failure, as distinct from a per-stratum one.
///
/// A per-stratum failure is *data* — a [`ProducerStatus::ExecutionFailed`] in
/// [`ExecutionResult::statuses`] — because the remaining strata must still run.
/// Only a defect that invalidates the run itself is an error here.
///
/// # Why there are exactly two variants
///
/// Every failure a *unit* can have is attributable to the stratum that unit was
/// compiled for, and the registry identity is checked before any unit runs, so
/// there is no window in which an evaluation failure exists without a stratum
/// to hang it on. That left the registry mismatch as the only condition known
/// to invalidate the whole run — and the enum was left `#[non_exhaustive]`
/// precisely so a second whole-run condition could be added without a breaking
/// change if one were ever found. [`Self::InconsistentWitness`] is that second
/// condition, and it is added on the terms that argument set rather than
/// against them: a variant no path can construct is worse than no variant, and
/// this one is reached by a real observation the executor now makes — an index
/// that moved between two of one run's invocations, executed end to end in
/// `tests/execute_dataset.rs`.
///
/// What makes it whole-run rather than per-stratum is *what it observes*. A
/// witness that does not describe exactly one relation, one index generation
/// and at most one incompleteness reason is not a report about one stratum's
/// producer behaving badly; it is the evidence channel disagreeing with the
/// admission waist about what was registered, or an index moving underneath the
/// query. Neither fact is confined to the stratum that noticed it, and the
/// remaining strata's answers rest on the same two assumptions.
///
/// # The witness rule is the reachable construction; the budget arm is not
///
/// [`execute`] raises [`Self::InconsistentWitness`] from two places, and they
/// are not equal. The witness rule is the observation just described. The other
/// is the `GovernedOutcome::BudgetExhausted` arm, and **nothing a compiled unit
/// can do reaches it today** — which is worth writing down plainly rather than
/// leaving a reader to infer that the executor has seen one.
///
/// It is not removable, and that is the honest resolution rather than an excuse.
/// `GovernedOutcome` is deliberately *not* `#[non_exhaustive]`: its two variants
/// are the whole taxonomy a governor can produce, so the compiler requires both
/// to be handled and the only open question is what this stage does with the
/// second. The two alternatives are worse than a refusal nothing reaches. Taking
/// the partial rows as an answer would report a truncated read as a stratum that
/// exhausted — the silent drop, certified. Panicking would turn a condition this
/// crate cannot rule out *by type* into a crash in a library.
///
/// What rules it out is the lane's configuration, which is a fact about two
/// values rather than about this enum: [`QueryGovernors::UNBOUNDED`] engages no
/// caller-settable ceiling and carries no stop signal, leaving only the
/// evaluator's fixed recursion guard on user-function depth engaged; and the
/// options built below inject no user-function registry, so a unit cannot enter
/// a user function at all, let alone nest one past a build constant. Both halves
/// are executed in `tests/execute_dataset.rs` beside the witness test, so a
/// later change to either — a ceiling added to the lane, a registry injected —
/// reddens a test rather than quietly making a documented impossibility
/// possible.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ExecutionError {
    /// The compiled units were built against a different live registry instance.
    #[error("compiled units name registry instance {expected:?}, but execution holds {got:?}")]
    RegistryMismatch {
        /// The instance the compiled bundle records.
        expected: RegistryId,
        /// The instance execution was handed.
        got: RegistryId,
    },

    /// The relation witness for one unit's run cannot be the witness of a
    /// conforming compiled unit.
    ///
    /// A compiled unit binds one producer and passes it constants, so its run
    /// invokes one relation over one pinned index generation with at most one
    /// thing to say about that index's wholeness. More than one of any of those
    /// — or none at all from a unit that returned rows — means the registry or
    /// the snapshot was not what the unit was compiled against, which is a fact
    /// about the run and not about the stratum that happened to expose it. It
    /// is **not** [`ProducerStatus::ExecutionFailed`]: that status says the
    /// producer could not run and carries no rows, and this unit ran and
    /// returned rows.
    ///
    /// The invocation count is deliberately not part of the rule; see this
    /// module's header for why tightening it to include the count would be a
    /// flake rather than a check.
    ///
    /// One other condition is reported here, for want of a whole-run refusal
    /// that would mean anything different: a governed outcome that tripped on a
    /// lane which declined every ceiling. It is the same shape of fact — the run
    /// did not happen under the assumptions it was compiled against — and it is
    /// unreachable through this lane's configuration, per this type's own docs.
    #[error("stratum {stratum}: the relation witness is not a compiled unit's: {reason}")]
    InconsistentWitness {
        /// The stratum whose unit produced the witness. Boxed because an
        /// [`Iri`] is much wider than the other variant's two ids, and a large
        /// `Err` is paid for on every call that returns `Ok`.
        stratum: Box<Iri>,
        /// What about the witness could not be a compiled unit's, naming the
        /// count that was wrong.
        reason: String,
    },

    /// A stratum's producer yielded more rows than the registry declared it
    /// could yield per invocation.
    ///
    /// The declared row bound is what
    /// [`compile`](crate::compile) sizes the emitted `LIMIT` against and what
    /// admission holds a recorded depth to, so a producer that beats it has
    /// invalidated both decisions for this run. The condition is observable
    /// only because the emitted bound carries a probe slot one row past the
    /// declaration: the read is allowed to reach for a row the registry said
    /// does not exist, precisely so that its arrival can be reported.
    ///
    /// It is a **whole-run refusal**, not a
    /// [`ProducerStatus::ExecutionFailed`] entry, for two reasons. That status
    /// says the producer could not run and carries no rows, and this producer
    /// ran and returned rows — the same distinction
    /// [`Self::InconsistentWitness`] draws. And the broken number is not
    /// confined to the stratum that exposed it: the same declaration ordered
    /// this call against the other operators of its group and admitted every
    /// depth in the plan, so the remaining strata's answers rest on it too.
    ///
    /// The alternative to refusing is what this layer did before the slot
    /// reached this depth: truncate to the depth and report the stratum
    /// [`ProducerStatus::Exhausted`] — the one ending that names no stopper —
    /// minted for a read that demonstrably had more rows behind it.
    ///
    /// Both numbers are carried because either alone is unactionable. `declared`
    /// is the promise a host has to go and fix in its producer, and `pulled` is
    /// the evidence that it is false.
    ///
    /// `mode` says *which* promise, and it is not decoration for the producers this
    /// layer is built for. A declared row count is a function of the access mode, so an
    /// index-backed producer has several registered, and the one that bounds a read is
    /// the tightest among the declared modes that serve it — routinely a coarser mode
    /// than the call was made under. Named without its mode, the figure sent an author
    /// to a declaration the call was not made at.
    #[error(
        "stratum {stratum}: the registry declares at most {declared} rows per invocation{mode}, and the read returned {pulled}"
    )]
    RowBoundBreached {
        /// The stratum whose producer beat its own declaration. Boxed for the
        /// reason [`Self::InconsistentWitness`] boxes its own: an [`Iri`] is
        /// much wider than the two counts beside it, and a large `Err` is paid
        /// for on every call that returns `Ok`.
        stratum: Box<Iri>,
        /// The row bound the registry declared for this stratum, as
        /// [`StratumUnit::declared_rows()`](crate::StratumUnit::declared_rows) carries it.
        declared: u64,
        /// How many rows the read actually returned, which is past `declared` and
        /// usually one past it: the probe slot is the only row past the declaration
        /// the emitted bound asks for, except at a declared zero, where the depth is
        /// read at the floor of one and the bound therefore asks for two.
        pulled: u64,
        /// Which declared access mode `declared` was read at, relative to the mode this
        /// read was invoked under. [`BoundMode::Undeclared`] for a unit a caller
        /// assembled itself, which records the count and no mode.
        mode: BoundMode,
    },

    /// The bundle's units are not the units it was assembled from.
    ///
    /// Every later step of a run is keyed by the stratum a unit carries — the status
    /// map, the tag on the stream, the per-stratum weight a fusion profile applies —
    /// so the tag above a unit decides *whose* read the evidence describes. It was a
    /// public field on a public `Vec` above a type whose own numbers and text had
    /// already been sealed, and three edits to it each produced a served answer with a
    /// real plan identity on it: a stratum renamed onto its neighbour's lost one
    /// producer's evidence entirely, a swap crossed two producers' statuses, and
    /// dropping a unit answered from one stratum fewer at `ScoreExactness::Exact`.
    ///
    /// The bundle records its attribution when it is assembled
    /// ([`CompiledRetrieval::new`](crate::CompiledRetrieval::new)), so this refusal is
    /// about a bundle that *changed*, never about a caller assembling its own. It is
    /// the compile-to-execute half of the implication the admission waist already
    /// enforces from a plan into `compile`: a stratum missing from the set is a
    /// producer missing from the answer, and that must be a refusal rather than a
    /// silently narrower read.
    #[error("the compiled bundle for plan {plan} is not the bundle it was assembled as: {reason}")]
    UnitsNotAsAssembled {
        /// The plan identity the bundle names, so a report can say which bundle
        /// moved. Carried by value: it is a fixed 32-byte digest, not a growable
        /// field like the [`Iri`]s the other variants box.
        plan: PlanId,
        /// What moved, named at the position it moved in.
        reason: String,
    },
    /// A registered relation's declaration methods panicked while the execution
    /// environment was being derived from the caller's registry.
    ///
    /// Derivation reads every registered relation's declaration — that is how the
    /// parser learns which predicate IRIs are calls — so a relation whose `describe`
    /// panics makes the environment underivable. It is a refusal rather than a
    /// fallback to an empty environment, because an empty environment would lower
    /// every one of this bundle's relation calls to an ordinary triple pattern and
    /// answer over the base graph: a silently narrower read, which is exactly the
    /// outcome the rest of this type exists to prevent.
    #[error("the execution environment could not be derived from the registry: {reason}")]
    EnvironmentNotDerivable {
        /// The registry's own failure, propagated unchanged.
        reason: String,
    },
}

/// A concrete ranked stream of `(rank, candidate, block)` rows.
///
/// The rows are either materialized by the evaluator before the stream exists and
/// drained in order, or — under [`ReadSchedule::OnDemand`] — produced one per
/// [`next`](Self::next) from an invocation held open. Either way `next` never
/// pends, so the stream is usable under any executor. A caller that wants to fuse
/// the rows wraps them with the fusion profile at `fuse` time.
///
/// The third element is the block of the candidate universe the row was drawn
/// from ([`RowBlock`]) — read from the unit's own `?block` column where the
/// producer declared a position for it, entailed from a single-block declaration
/// where it did not, and [`RowBlock::Undeclared`] for a producer that restricted
/// nothing. It is carried here rather than attached at `fuse` time for the reason
/// the contract and the attestation are carried: the producer is the only party
/// that knows it, and a consumer three stages downstream is the party that checks
/// it.
///
/// There is deliberately no bulk accessor beside [`next`](Self::next): reading
/// the rows one at a time and then taking the [`receipt`](Self::receipt) *is*
/// the unfused rung, and a second way to get at the same rows would be a second
/// protocol — one with no receipt at the end of it, and so no way for a caller
/// to tell a stratum that ended from a stratum it stopped reading.
#[derive(Debug)]
pub struct RankedStreamImpl<'d> {
    /// Where the rows come from: a read already materialised, or one produced as
    /// this stream is pulled.
    source: RowSource<'d>,
    pulled: u64,
    exhausted: bool,
    /// The prepared exclusion lookup for this stratum, where its producer
    /// declared a basis for one.
    ///
    /// `None` is the whole of "this stratum answers no exclusion lookup", and it
    /// is what every stream of every producer that declared
    /// [`ExclusionBasis::Unavailable`](purrdf_sparql_eval::ExclusionBasis)
    /// carries.
    ///
    /// # Why the stream borrows the dataset, and why that is the honest shape
    ///
    /// The rows may be materialized before this type exists; an exclusion
    /// verdict is not, and cannot be. Which candidates a fusion needs verdicts for is
    /// decided by the fusion, from a frontier that does not exist until the rows
    /// are being merged — so the alternative to holding the dataset is
    /// pre-computing verdicts for candidates nobody will ask about, which is the
    /// drain this whole mechanism removes. The borrow is what makes the lifetime
    /// parameter on this type unavoidable, and it is stated rather than worked
    /// around because a lookup that could not reach the dataset would have to
    /// answer from something else.
    exclusion: Option<Box<dyn ExclusionLookup + 'd>>,
    /// How many rows the read that produced this stream really returned.
    ///
    /// Not `rows.len()` in general, and that is the point. The rows this stream
    /// holds are what survived [`bound_to_depth`] — the depth's worth, with the
    /// probe row taken off — while this counts what the evaluator handed back,
    /// which is what the read actually cost. The two differ by the probe row on
    /// every unit that got one, and by a great deal more on a unit whose
    /// producer beat its depth.
    ///
    /// Set by [`Self::new`] to the rows it was handed, which is the honest
    /// answer for a caller-assembled stream: those rows are the whole of the
    /// read behind it. [`execute`] overrides it through
    /// [`Self::with_materialised_rows`] with the number only it can know. A read
    /// produced on demand counts its own, as it goes, and this field is not read
    /// for it.
    materialised: u64,
    /// Whether the fault [`Self::next`] last refused with invalidates the whole run —
    /// the producer beating its declared row bound — rather than only this stratum.
    ///
    /// Both reach a consumer as [`ProtocolError::ReadFailed`], because a fused stream
    /// fails the request either way. A stream nothing fuses is the one consumer that
    /// tells them apart: an isolated fault is that stratum's status, exactly as it is
    /// for a materialised read, while a breach is refused for every stratum, weighted
    /// or not, exactly as [`ExecutionError::RowBoundBreached`] is.
    run_invalidated: bool,
}

/// Where a [`RankedStreamImpl`]'s rows come from.
enum RowSource<'d> {
    /// A read that finished before this stream existed: its rows, already cut to the
    /// depth, and the ending it had.
    Materialised {
        rows: VecDeque<(u64, Term, RowBlock)>,
        ending: StreamEnding,
        /// What the read's witness attested when it finished, for a read this
        /// executor took; `None` for rows a caller handed [`RankedStreamImpl::new`],
        /// which came from no read this crate observed.
        attested: Option<PfAttestation>,
    },
    /// One invocation held open and read a row per pull, and — once it has ended —
    /// how it ended.
    OnDemand {
        read: Box<dyn OnDemandRead + 'd>,
        ended: Option<OnDemandEnd>,
    },
}

impl fmt::Debug for RowSource<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Materialised {
                rows,
                ending,
                attested,
            } => f
                .debug_struct("Materialised")
                .field("rows_left", &rows.len())
                .field("ending", ending)
                .field("attested", attested)
                .finish(),
            Self::OnDemand { read, ended } => f
                .debug_struct("OnDemand")
                .field("read", read)
                .field("ended", ended)
                .finish(),
        }
    }
}

/// How an on-demand read ended: an ending [`bound_to_depth`] would have reported for
/// the same rows, or a failure before its first row, which is that stratum's status
/// exactly as a materialised read's failure is.
#[derive(Clone, Debug)]
enum OnDemandEnd {
    Ended(StreamEnding),
    Failed(String),
}

/// One stratum's invocation, held open and read as its stream is pulled.
///
/// Object-safe for the reason [`ExclusionLookup`] is: the dataset's type stays out
/// of the stream's.
trait OnDemandRead: fmt::Debug {
    /// The stratum this read is for, naming its refusals.
    fn stratum(&self) -> &str;
    /// The next ranked row, or how the read ended.
    fn pull(&mut self) -> Result<ReadStep, ReadFault>;
    /// The rows the invocation has returned so far, the probe row included once it
    /// has been read.
    fn materialised(&self) -> u64;
    /// The attestation the invocation stands behind now, read off its witness under
    /// the sole-witness rule, or the rule it broke.
    fn settle(&self) -> Result<PfAttestation, String>;
}

/// One step of an on-demand read.
enum ReadStep {
    /// The next row, ranked.
    Row((u64, Term, RowBlock)),
    /// The read has ended, this way.
    Ended(StreamEnding),
}

/// Why an on-demand read could not go on.
struct ReadFault {
    /// The refusal, rendered as a materialised read would render it.
    reason: String,
    /// Whether the failure invalidates the whole run whenever it is observed — the
    /// producer beating its declared row bound — rather than only this stratum.
    whole_run: bool,
}

/// One stratum's prepared *do you hold this one* question, asked per candidate.
///
/// Object-safe and synchronous: the evaluator this crate reads through is
/// synchronous, and the `async` shape of
/// [`RankedStream::exclusion`](crate::RankedStream::exclusion) is the fusion
/// stage's protocol rather than a pending future. Boxed behind this trait rather
/// than named concretely so the dataset's own type stays out of every type
/// between here and the fusion engine — a fusion is over strata, and which
/// concrete store answered is not a fact its type should carry.
pub(crate) trait ExclusionLookup: fmt::Debug {
    /// Ask this stratum's producer whether `candidate` is out of its reach.
    ///
    /// # Errors
    ///
    /// [`ProtocolError::ExclusionLookupFailed`] for every way the lookup can
    /// fail — the term did not decode, the dataset read was refused, the
    /// producer answered with more rows than its declared bound admits — and
    /// [`ProtocolError::ExclusionAttestationMoved`] when the index generation that
    /// answered is not the one the stratum's ranked read pinned. None of them
    /// degrades to a verdict.
    ///
    /// `&mut self` because a lookup is a prepared execution that is bound and run
    /// once per candidate, and a prepared execution is bound and run through a
    /// unique borrow — see [`PreparedExecution`].
    fn look_up(&mut self, candidate: &Term) -> Result<ExclusionVerdict, ProtocolError>;
}

/// How a candidate this execution read is bound into an exclusion lookup, decided
/// once at the ranking read that named it rather than once per lookup.
///
/// The fusion stage asks for verdicts by [`Term`], the canonical lexical the
/// frontier keys candidates by. Turning that text back into a term on every lookup
/// would re-parse, per candidate, a term the ranking read already held as a value;
/// and binding it by value would then re-ground it. The ranking read is where the
/// term is in hand, so that is where this is decided: the dataset's own id for it
/// where the dataset holds the term — bound through
/// [`PreparedExecution::bind_id`], which resolves it against the same view the
/// lookup runs over — and the value itself where the dataset does not, which is the
/// honest case for a producer whose index names terms the graph never mentions.
#[derive(Clone, Debug)]
enum CandidateBinding<I> {
    /// The dataset holds the term, at this id.
    Id(I),
    /// The dataset does not hold the term; it is bound by value.
    Value(TermValue),
}

/// Every candidate the ranking reads of one [`execute_within`] call named, keyed by
/// the [`Term`] the fusion stage will ask about. Shared by that call's lookups.
type CandidateIndex<I> = HashMap<Term, CandidateBinding<I>>;

/// One stratum's prepared lookups, held until every ranking read of the call is
/// open and they can be attached to its stream.
struct PendingLookup {
    /// The stream's position in the call's streams.
    position: usize,
    /// The stratum the lookups answer for.
    stratum: Iri,
    /// The lookups, each prepared once with the candidate as its one parameter,
    /// beside that parameter's slot — see [`DatasetExclusion::lookups`].
    lookups: Vec<(PreparedExecution, usize)>,
    /// What the stratum's ranked read pinned, which every lookup is held to.
    pinned: PfAttestation,
}

/// The exclusion lookup [`execute`] compiles: one prepared execution per qualifying
/// call of the stratum's text, run against the caller's dataset once per candidate.
///
/// Prepared **once** per stratum and bound per candidate, which is the whole
/// reason this is a value rather than a closure over a query string: a text
/// re-parsed per candidate would pay a parse and a feasibility pass for a question
/// whose answer is a row count. The candidate is bound into each handle's one
/// declared parameter, by the dataset id the ranking read already resolved where
/// there is one (see [`CandidateBinding`]), and the substitution pushes it into the
/// producer's call, so the producer is invoked with the candidate position bound
/// and answers the one question asked — a point read, not a scan of its index.
///
/// # Several calls, one verdict
///
/// A rendered unit has one call, and so one lookup. A caller's text may draw its
/// `?candidate` column from several calls — a join of calls on it — and every one of
/// them that the registry qualifies is a proof of absence on its own
/// ([`StratumUnit::exclusion_sparql`](crate::StratumUnit)): the column holds only
/// values each of them emitted. So the lookups are asked in order and the first that
/// finds no row answers `Excluded`; `Possible` needs every one of them to find its
/// candidate. Each is held, before its row count is read at all, to the one
/// attestation the stratum's ranked read pinned — the sole-witness rule makes that one
/// attestation cover every call the text made, since every call of a conforming unit
/// invokes the one relation at one generation — so a verdict from any lookup answered
/// by another index generation is refused, whichever lookup it was.
struct DatasetExclusion<'d, D: DatasetView + Sync> {
    /// The stratum this lookup answers for, so its refusals name a producer
    /// rather than a query.
    stratum: Iri,
    /// The engine the execution was prepared on, shared by every lookup the same
    /// [`execute_within`] call compiled. Shared rather than owned per stratum
    /// because the stream outlives that call, and an engine is not a fact about
    /// any one stratum.
    engine: Rc<NativeSparqlEngine>,
    /// The extension environment the execution was prepared against, shared with
    /// the run that produced the rows. An execution prepared against one registry
    /// and run under another is refused by the evaluator, so the same value must
    /// reach both calls — see [`execute_within`]'s own single-environment note.
    env: Arc<ExtensionEnv>,
    /// The lookups themselves, one per qualifying call in the order
    /// [`StratumUnit::exclusion_sparql`](crate::StratumUnit) derived them, each
    /// prepared once with the candidate as its one parameter and beside that
    /// parameter's slot, resolved once.
    lookups: Vec<(PreparedExecution, usize)>,
    /// How each candidate this execution read is bound, shared by its lookups and
    /// written by the reads that name candidates — on demand, as each row is pulled.
    candidates: Rc<RefCell<CandidateIndex<D::Id>>>,
    /// The caller's dataset, read exactly as the ranking read it.
    dataset: &'d D,
    /// What the ranked read of this stratum pinned: the attestation its stream
    /// announced before its first row. Every lookup is held to it — see
    /// [`ProtocolError::ExclusionAttestationMoved`].
    pinned: PfAttestation,
}

impl<D: DatasetView + Sync> fmt::Debug for DatasetExclusion<'_, D> {
    /// Names the stratum and nothing else. A prepared execution has no `Debug` a
    /// reader could act on, and a dataset's is unbounded — printing either would
    /// make a stream's `{:?}` a dump of the store it was read from.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DatasetExclusion")
            .field("stratum", &self.stratum.as_str())
            .finish_non_exhaustive()
    }
}

impl<D: DatasetView + Sync> ExclusionLookup for DatasetExclusion<'_, D> {
    fn look_up(&mut self, candidate: &Term) -> Result<ExclusionVerdict, ProtocolError> {
        let binding = self.candidates.borrow().get(candidate).cloned();
        for index in 0..self.lookups.len() {
            if self.ask(index, candidate, binding.as_ref())? == ExclusionVerdict::Excluded {
                return Ok(ExclusionVerdict::Excluded);
            }
        }
        Ok(ExclusionVerdict::Possible)
    }
}

impl<D: DatasetView + Sync> DatasetExclusion<'_, D> {
    /// Ask the `index`-th lookup whether `candidate`, bound as `binding` says, is out
    /// of its call's reach.
    fn ask(
        &mut self,
        index: usize,
        candidate: &Term,
        binding: Option<&CandidateBinding<D::Id>>,
    ) -> Result<ExclusionVerdict, ProtocolError> {
        let stratum = &self.stratum;
        let failed = |reason: String| ProtocolError::ExclusionLookupFailed {
            stratum: stratum.as_str().to_owned(),
            reason,
        };
        let (execution, slot) = &mut self.lookups[index];
        let slot = *slot;
        let bound = match binding {
            Some(CandidateBinding::Id(id)) => execution.bind_id(slot, self.dataset, *id),
            Some(CandidateBinding::Value(value)) => execution.bind(slot, value.clone()),
            // A candidate no ranking read of this execution named: one from a
            // stream assembled elsewhere and fused beside these. Its canonical
            // lexical is the only form it arrives in, so it is read back through the
            // crate's own decoder. A term this layer wrote and cannot read back is a
            // defect in one of the two halves, and it is reported as the failed
            // lookup it is rather than answered as `Possible`.
            None => {
                let value = decode_term(candidate.as_str()).map_err(|reason| {
                    failed(format!("candidate {candidate} did not decode: {reason}"))
                })?;
                execution.bind(slot, value)
            }
        };
        bound.map_err(|diagnostic| failed(diagnostic.to_string()))?;
        let options = QueryOptions {
            env: &self.env,
            ..QueryOptions::EMPTY
        };
        // Ungoverned, and the governed ranking read's reasoning is why that is no
        // loss: that lane declines every caller-settable ceiling and is taken for
        // its receipt. A lookup's receipt is two facts — its row count, which the
        // visitor reads here, and the witness of the index generation that
        // answered, which the witnessed door hands back beside it.
        let (rows, witness) = self
            .engine
            .execute_witnessed(execution, self.dataset, options, |outcome| match outcome {
                InternedOutcome::Solutions(solutions) => Some(solutions.len()),
                InternedOutcome::Boolean(_) | InternedOutcome::Graph(_) => None,
            })
            .map_err(|diagnostic| failed(diagnostic.to_string()))?;
        let rows =
            rows.ok_or_else(|| failed("the exclusion lookup did not return solutions".to_owned()))?;
        // Held to the read before its verdict is read at all. A verdict is only a
        // statement about the rows the ranked read would have gone on to name if it
        // came from the index generation that read pinned; one from anywhere else
        // is refused whatever it says, because a verdict that happens to agree has
        // still certified rows on the word of an index the answer's evidence does
        // not name. The lookup is one call with constant arguments, so its witness
        // is read under the rule a compiled unit's is — one relation, one
        // generation — and a witness that breaks it is the same refusal.
        let moved = |reason: String| ProtocolError::ExclusionAttestationMoved {
            stratum: stratum.as_str().to_owned(),
            reason,
        };
        let answered = sole_attestation(&witness).map_err(moved)?;
        if answered != self.pinned {
            return Err(moved(format!(
                "the read pinned {:?}, the lookup of {candidate} was answered under {:?}",
                self.pinned, answered
            )));
        }
        let pulled = u64::try_from(rows).unwrap_or(u64::MAX);
        // The point bound a declared exclusion basis rests on, derived from the
        // ceiling the lookup is read under rather than written twice: the text
        // asks for one row past the bound precisely so a producer that beats it
        // is observed instead of truncated into looking conforming.
        const POINT_BOUND: u64 = EXCLUSION_LIMIT - 1;
        if pulled > POINT_BOUND {
            // A producer that declared an exclusion basis declared, through the
            // registry, a candidate-bound mode whose row bound is a point bound.
            // More than one row back is that declaration broken, and it is
            // reported through the refusal that already exists for exactly that
            // — named here rather than given a family of its own, because a
            // producer that beats its own row bound is one fault however the
            // call that caught it was shaped.
            let breach = ExecutionError::RowBoundBreached {
                stratum: Box::new(self.stratum.clone()),
                declared: POINT_BOUND,
                pulled,
                // No declared mode stands behind this number *here*. The point
                // bound is the condition the registry admitted the basis under,
                // read at registration over the relation's own modes; this stage
                // holds the bound and not the mode it was read at, and naming
                // one would attribute the figure to a declaration this stage
                // never looked at — the same reason a caller-assembled unit
                // records `Undeclared`.
                mode: BoundMode::Undeclared,
            };
            return Err(failed(breach.to_string()));
        }
        Ok(if pulled == 0 {
            ExclusionVerdict::Excluded
        } else {
            ExclusionVerdict::Possible
        })
    }
}

/// How a stream's read ended.
///
/// The endings answer "what stopped this read", and only the producer's own
/// side of the seam can tell them apart: an empty cursor looks identical whether
/// the rows ran out or the bound did. [`execute`] can tell, because
/// [`compile`](crate::compile) emits a bound one row deeper than the stratum
/// reads, so the arrival of that extra row *is* the distinction — see this
/// module's header.
///
/// A materialised read's ending is known before its first row is pulled, and is
/// fixed when its stream is built. A read produced on demand learns its ending
/// only when it gets there — the probe row, or the producer running out — so its
/// ending is decided at that pull. That is not a status that can say something
/// different to a caller that stopped early, because it says nothing to such a
/// caller at all: [`RankedStreamImpl::receipt`] refuses to answer before the
/// stream has returned its last row ([`ProtocolError::NeverEndingSource`]), so an
/// ending is only ever observed about a read that reached it, and the same rows
/// reach the same ending under either schedule.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StreamEnding {
    /// Every row the unit could yield is on the stream.
    Exhausted,
    /// The planned depth stopped the read, and a further row existed.
    DepthReached {
        /// The last 1-based rank the stream carries, which is the planned depth
        /// and also the number of rows the stream holds.
        rank: u64,
    },
    /// The producer's own declared row bound stopped the read, so whether a further
    /// row existed could not be observed.
    ///
    /// Written for the one shape whose probe row cannot be asked for: a producer
    /// that takes its depth as an argument, planned at a depth that already sits on
    /// its declared row count, which therefore returned every row it was allowed and
    /// could not be asked for one more. See this module's header, and
    /// [`ProducerReceipt::RowBoundReached`] for why neither neighbour is true of it.
    RowBoundReached {
        /// The last 1-based rank the stream carries, which is both the planned depth
        /// and the declared row bound the read stopped at.
        rank: u64,
    },
    /// The query text was a caller's, and it is the stopper the read actually had, so
    /// whether a further row existed could not be observed.
    ///
    /// Written for every read of a unit built through
    /// [`StratumUnit::new`](crate::StratumUnit::new) that came back inside the unit's
    /// own bound. The layer bounds such a text only on the outside; what the text
    /// bounds *inside* itself — a `LIMIT` on a sub-`SELECT`, a pattern matching less
    /// than the producer holds — is no part of what this layer reads, so the absence of
    /// the probe row is no evidence. `Exhausted` would be the one ending that names
    /// no stopper, minted from a text whose bounds it never read, which is the
    /// defect this vocabulary exists to prevent; see
    /// [`ProducerReceipt::SuppliedQueryEnded`].
    ///
    /// A row arriving *past* the depth is still an observation, so such a read still
    /// ends [`Self::DepthReached`]: something further existed whatever the text
    /// bounded.
    SuppliedQueryEnded {
        /// The last 1-based rank the stream carries, which is the number of rows it
        /// holds. Zero for a read that returned nothing — which is not a claim that
        /// there was nothing to return.
        rank: u64,
    },
}

impl<'d> RankedStreamImpl<'d> {
    /// Build a stream over pre-ranked `(rank, candidate, block)` rows that ends the way
    /// `ending` says.
    ///
    /// The ending is a parameter rather than something inferred from `rows`,
    /// because it cannot be inferred from `rows`: the row count is the same
    /// either way, and the whole point of the distinction is that only the
    /// reader of the underlying answer knows which one happened.
    ///
    /// The stream answers no exclusion lookup. That is the honest state for a
    /// caller-built stream: an exclusion verdict is a measurement against a
    /// dataset, and this constructor is handed rows rather than a dataset to
    /// take one from. `attach_exclusion` is where
    /// [`execute`] attaches the prepared lookup it compiled.
    #[must_use]
    pub fn new(rows: Vec<(u64, Term, RowBlock)>, ending: StreamEnding) -> Self {
        // The rows in hand are the whole of the read behind a caller-assembled
        // stream, so they are the honest read-work figure for one. A `usize`
        // that does not fit a `u64` is a row count no machine produced; it is
        // saturated rather than truncated, because a wrapped count would report
        // an enormous read as a tiny one.
        let materialised = u64::try_from(rows.len()).unwrap_or(u64::MAX);
        Self {
            source: RowSource::Materialised {
                rows: rows.into(),
                ending,
                attested: None,
            },
            pulled: 0,
            exhausted: false,
            exclusion: None,
            materialised,
            run_invalidated: false,
        }
    }

    /// A stream over one invocation held open, producing a row per pull.
    ///
    /// Not public: an on-demand read is one this executor opened, against a dataset
    /// and a registry it was handed, and its ending is decided by observations only
    /// the executor's read makes.
    const fn on_demand(read: Box<dyn OnDemandRead + 'd>) -> Self {
        Self {
            source: RowSource::OnDemand { read, ended: None },
            pulled: 0,
            exhausted: false,
            exclusion: None,
            materialised: 0,
            run_invalidated: false,
        }
    }

    /// Record how many rows the read that produced these rows really returned.
    ///
    /// A builder step rather than a parameter of [`new`](Self::new) for the
    /// reason [`attach_exclusion`](Self::attach_exclusion) is: only the party that
    /// ran the read knows the number, and the rows reach this type after the
    /// depth has already been applied to them. A caller-assembled stream
    /// attaches nothing and reports the rows it was handed.
    #[must_use]
    pub(crate) const fn with_materialised_rows(mut self, rows: u64) -> Self {
        self.materialised = rows;
        self
    }

    /// Record what the witness of the read that produced these rows attested.
    ///
    /// A builder step for the reason [`with_materialised_rows`](Self::with_materialised_rows)
    /// is: only the party that ran the read holds its witness. A caller-assembled
    /// stream records nothing, and settles to nothing.
    #[must_use]
    pub(crate) fn with_attested(mut self, attestation: PfAttestation) -> Self {
        if let RowSource::Materialised { attested, .. } = &mut self.source {
            *attested = Some(attestation);
        }
        self
    }

    /// How many rows the read behind this stream returned.
    ///
    /// The number [`RankedStream::rows_materialised`](crate::RankedStream::rows_materialised)
    /// carries into the fused trailer; see there for what it is for and why it
    /// is not the ranks a fusion pulled.
    ///
    /// For a stream read on demand, the rows its invocation has returned **so far**:
    /// the rows pulled, and the probe row once a pull past the planned depth has read
    /// it. It grows exactly as far as the stream's consumer reads and no further.
    #[must_use]
    pub fn rows_materialised(&self) -> u64 {
        match &self.source {
            RowSource::Materialised { .. } => self.materialised,
            RowSource::OnDemand { read, .. } => read.materialised(),
        }
    }

    /// How many rows this stream has handed out through [`Self::next`].
    #[must_use]
    pub const fn rows_emitted(&self) -> u64 {
        self.pulled
    }

    /// What the read behind this stream stands behind now, if this crate observed
    /// the read.
    ///
    /// For a read this executor materialised, the attestation its witness held,
    /// read under the sole-witness rule when its run finished and before its first
    /// row was readable — the one [`StratumStream::attestation`] carries. It has
    /// nothing left to learn, and it is still handed back rather than withheld: a
    /// consumer holds whatever a caller announced for the stream to this, exactly
    /// as it holds an on-demand read's announcement to its settled witness, so the
    /// two schedules refuse a mis-announced attestation alike and the evidence an
    /// answer names is the evidence its read and its exclusion lookups stood
    /// behind. For a read produced on demand, the invocation's witness as it stands
    /// at this instant, read under the same rule. `None` for rows a caller handed
    /// [`Self::new`]: no read of this crate's stands behind them.
    ///
    /// # Errors
    ///
    /// [`ProtocolError::AttestationMoved`] when that witness cannot be one
    /// attestation — the index the read served from moved under it — naming the
    /// count the rule refused.
    // Synchronous for the reason `next` is.
    #[allow(clippy::unused_async, clippy::unused_async_trait_impl)]
    #[expect(
        clippy::future_not_send,
        reason = "an on-demand read shares its candidate index with the stratum's \
                  exclusion lookups through `Rc<RefCell<_>>` and is awaited in the one \
                  task that fuses it, so the future is not `Send` by construction"
    )]
    pub async fn settle(&mut self) -> Result<Option<PfAttestation>, ProtocolError> {
        match &self.source {
            RowSource::Materialised { attested, .. } => Ok(attested.clone()),
            RowSource::OnDemand { read, .. } => {
                read.settle()
                    .map(Some)
                    .map_err(|reason| ProtocolError::AttestationMoved {
                        stratum: read.stratum().to_owned(),
                        reason,
                    })
            }
        }
    }

    /// Whether the fault [`Self::next`] refused with invalidates the whole run
    /// rather than only this stratum: the producer returned more rows than its
    /// registry declared it could.
    pub(crate) const fn run_invalidated(&self) -> bool {
        self.run_invalidated
    }

    /// Attach the prepared exclusion lookup this stratum answers through.
    ///
    /// A builder step rather than a parameter of [`new`](Self::new), for the
    /// reason the plan identity is one on the adapter above: a lookup exists
    /// only where the producer declared a basis and only where this layer
    /// rendered the query, and a stream without one is not a stream missing
    /// anything.
    ///
    /// On a stream already in place rather than as a consuming builder step:
    /// [`execute_within`] attaches its lookups after every stratum has been read,
    /// because a lookup binds candidates any stratum of the call may have named,
    /// and those are only all known once the last ranking read is in.
    pub(crate) fn attach_exclusion(&mut self, lookup: Box<dyn ExclusionLookup + 'd>) {
        self.exclusion = Some(lookup);
    }

    /// Ask this stratum's producer whether `candidate` is out of its reach.
    ///
    /// # Errors
    ///
    /// [`ProtocolError::ExclusionUnavailable`] when no lookup was compiled for
    /// this stratum, and whatever the lookup itself refuses with — its failure
    /// ([`ProtocolError::ExclusionLookupFailed`]), or an answer given under an
    /// attestation other than the one this stream's read pinned
    /// ([`ProtocolError::ExclusionAttestationMoved`]). None is
    /// answered as [`ExclusionVerdict::Possible`]: see
    /// [`RankedStream::exclusion`](crate::RankedStream::exclusion) for why a
    /// failed measurement must not wear that costume.
    // Synchronous for the reason `next` is: the evaluator behind it is
    // synchronous, and the `async` shape is the fusion stage's protocol.
    #[allow(clippy::unused_async, clippy::unused_async_trait_impl)]
    #[expect(
        clippy::future_not_send,
        reason = "an on-demand read shares its candidate index with the stratum's \
                  exclusion lookups through `Rc<RefCell<_>>` and is awaited in the one \
                  task that fuses it, so the future is not `Send` by construction"
    )]
    pub async fn exclusion(&mut self, candidate: &Term) -> Result<ExclusionVerdict, ProtocolError> {
        match self.exclusion.as_mut() {
            Some(lookup) => lookup.look_up(candidate),
            None => Err(ProtocolError::ExclusionUnavailable),
        }
    }

    /// The next row, or `None` when the stream is exhausted.
    ///
    /// # Errors
    ///
    /// A [`ProtocolError`] if the stream cannot describe its own rows; this
    /// materialized stream never raises one.
    // The body is synchronous because the evaluator materializes its rows before
    // returning; the `async` shape is the ranked-stream contract the fusion stage
    // consumes, and a caller may compose it with genuinely asynchronous streams.
    #[allow(clippy::unused_async, clippy::unused_async_trait_impl)]
    #[expect(
        clippy::future_not_send,
        reason = "an on-demand read shares its candidate index with the stratum's \
                  exclusion lookups through `Rc<RefCell<_>>` and is awaited in the one \
                  task that fuses it, so the future is not `Send` by construction"
    )]
    pub async fn next(&mut self) -> Result<Option<(u64, Term, RowBlock)>, ProtocolError> {
        let row = match &mut self.source {
            RowSource::Materialised { rows, .. } => rows.pop_front(),
            RowSource::OnDemand { read, ended } => {
                if ended.is_some() {
                    None
                } else {
                    match read.pull() {
                        Ok(ReadStep::Row(row)) => Some(row),
                        Ok(ReadStep::Ended(ending)) => {
                            *ended = Some(OnDemandEnd::Ended(ending));
                            None
                        }
                        // Before the first row, a failure is this stratum's alone,
                        // reported exactly as a materialised read's is: through the
                        // receipt, as `ExecutionFailed`, with every other stratum
                        // still answering.
                        Err(ReadFault {
                            reason,
                            whole_run: false,
                        }) if self.pulled == 0 => {
                            *ended = Some(OnDemandEnd::Failed(reason));
                            None
                        }
                        // After rows were handed out — or at any point, for a
                        // breach of the declared row bound — it fails the request by
                        // name, because those rows may already be in the answer.
                        Err(ReadFault { reason, whole_run }) => {
                            self.run_invalidated = whole_run;
                            return Err(ProtocolError::ReadFailed {
                                stratum: read.stratum().to_owned(),
                                rows_before: self.pulled,
                                reason,
                            });
                        }
                    }
                }
            }
        };
        match row {
            Some(row) => {
                self.pulled += 1;
                Ok(Some(row))
            }
            None => {
                self.exhausted = true;
                Ok(None)
            }
        }
    }

    /// How the stream ended. Call only after [`Self::next`] returned `None`.
    ///
    /// A [`StreamEnding::Exhausted`] stream reports the rows it actually
    /// emitted; a [`StreamEnding::DepthReached`] stream reports the rank the
    /// depth stopped it at, and a [`StreamEnding::RowBoundReached`] stream the rank
    /// its producer's own declaration stopped it at. Fusion measures every one of
    /// those numbers against the rows it pulled and refuses a disagreement
    /// ([`ProtocolError::ForgedReceipt`]), which is why the rank is the depth
    /// the rows were truncated to and not the number of rows the unit returned.
    ///
    /// # Errors
    ///
    /// [`ProtocolError::NeverEndingSource`] when called before the stream was
    /// drained, because a completeness claim from a partially read stream is
    /// exactly the falsifiable status the protocol forbids.
    #[allow(clippy::unused_async, clippy::unused_async_trait_impl)]
    #[expect(
        clippy::future_not_send,
        reason = "an on-demand read shares its candidate index with the stratum's \
                  exclusion lookups through `Rc<RefCell<_>>` and is awaited in the one \
                  task that fuses it, so the future is not `Send` by construction"
    )]
    pub async fn receipt(&mut self) -> Result<ProducerReceipt, ProtocolError> {
        if !self.exhausted {
            return Err(ProtocolError::NeverEndingSource);
        }
        let ending = match &self.source {
            RowSource::Materialised { ending, .. } => *ending,
            RowSource::OnDemand {
                ended: Some(OnDemandEnd::Ended(ending)),
                ..
            } => *ending,
            RowSource::OnDemand {
                ended: Some(OnDemandEnd::Failed(reason)),
                ..
            } => {
                return Ok(ProducerReceipt::ExecutionFailed {
                    reason: reason.clone(),
                });
            }
            // `exhausted` is set only on the pull that recorded an end, so an
            // exhausted on-demand stream always has one; reporting this rather than
            // inventing an ending keeps the impossibility a refusal.
            RowSource::OnDemand { ended: None, .. } => {
                return Err(ProtocolError::NeverEndingSource);
            }
        };
        Ok(match ending {
            StreamEnding::Exhausted => ProducerReceipt::Exhausted {
                rows_emitted: self.pulled,
            },
            StreamEnding::DepthReached { rank } => ProducerReceipt::DepthReached { rank },
            StreamEnding::RowBoundReached { rank } => ProducerReceipt::RowBoundReached { rank },
            StreamEnding::SuppliedQueryEnded { rank } => {
                ProducerReceipt::SuppliedQueryEnded { rank }
            }
        })
    }
}

/// Run every compiled unit independently through `purrdf-sparql-eval` against
/// `dataset`.
///
/// Each unit is evaluated over the caller's `dataset` with `registry` injected as
/// the property-function registry. A unit that fails is recorded as that
/// stratum's [`ProducerStatus::ExecutionFailed`] and contributes no stream, while
/// every other unit runs to completion.
///
/// # Errors
///
/// [`ExecutionError::RegistryMismatch`] when `compiled` was built against a
/// different live registry instance than `registry`, and
/// [`ExecutionError::InconsistentWitness`] when a unit's run attested something
/// no compiled unit's run can attest. Per-stratum failures are not errors; they
/// are reported in [`ExecutionResult::statuses`].
// The executor drives a synchronous evaluator and returns materialized streams;
// the `async` shape is its stage contract, not a pending future. A caller composes
// it with the asynchronous fusion stage.
//
// `future_not_send`: the dataset is a caller-chosen type parameter, so the
// returned future's `Send`-ness is the caller's to establish. Requiring it here
// would force every caller's view — and its statistics and environment, through
// `search` — to be `Send` for a future that is awaited in one task and never
// crosses a thread boundary. `search` carries the same reasoning.
#[allow(
    clippy::unused_async,
    clippy::unused_async_trait_impl,
    clippy::future_not_send
)]
pub async fn execute<'d, D: DatasetView + Sync>(
    compiled: &CompiledRetrieval,
    registry: &PropertyFunctionRegistry,
    dataset: &'d D,
) -> Result<ExecutionResult<'d>, ExecutionError> {
    execute_within(compiled, registry, dataset, ReadSchedule::Materialised).await
}

/// Run every compiled unit as [`execute`] does, producing each stratum's rows on
/// the `schedule` given.
///
/// [`execute`] is this function at [`ReadSchedule::Materialised`]. Under
/// [`ReadSchedule::OnDemand`] every unit whose prepared text is one property-function
/// call under row-for-row operators — every unit this layer rendered, and a caller's
/// own text of that shape, a row-by-row `FILTER` included — is opened at its planned
/// depth as one invocation held
/// open, and its stream produces a row each time it is pulled — see this module's
/// header. The two schedules run the same text and read
/// the same rows in the same order; what differs is how many of them a consumer that
/// stops early ever causes to be produced.
///
/// Two consequences a caller of the on-demand schedule should know:
///
/// * an on-demand stratum has **no entry in [`ExecutionResult::statuses`]** until its
///   stream is read, because how it ends is decided by how far it is read. Its
///   stream's receipt is its status, and a caller that wants the status of a stream
///   it will not fuse reads the stream to its end;
/// * a unit whose text is not one call under row-for-row operators — a caller's own
///   join, `ORDER BY`, dataset clause, or a `FILTER` the read cannot evaluate row by
///   row — is materialised under this schedule too, and does get an entry.
///
/// # Errors
///
/// Exactly [`execute`]'s: [`ExecutionError::RegistryMismatch`],
/// [`ExecutionError::UnitsNotAsAssembled`],
/// [`ExecutionError::EnvironmentNotDerivable`],
/// [`ExecutionError::InconsistentWitness`] and
/// [`ExecutionError::RowBoundBreached`] — the last two only for a materialised read;
/// an on-demand read reports the same two facts when it reaches them, through its
/// stream ([`ProtocolError::AttestationMoved`], [`ProtocolError::ReadFailed`]).
// The same three allowances [`execute`] carries, for the same reasons; this is
// the body that function delegates to.
#[allow(
    clippy::unused_async,
    clippy::unused_async_trait_impl,
    clippy::future_not_send
)]
pub async fn execute_within<'d, D: DatasetView + Sync>(
    compiled: &CompiledRetrieval,
    registry: &PropertyFunctionRegistry,
    dataset: &'d D,
    schedule: ReadSchedule,
) -> Result<ExecutionResult<'d>, ExecutionError> {
    if compiled.registry_id != registry.instance_id() {
        return Err(ExecutionError::RegistryMismatch {
            expected: compiled.registry_id,
            got: registry.instance_id(),
        });
    }
    // Which producers this bundle answers for, before any of them is asked anything.
    // Checked here rather than per unit because the condition is about the SET: a
    // missing unit is only visible against the set the bundle was assembled with, and
    // running the units that remain would report a narrowed answer under a real plan
    // identity before the check could be reached.
    compiled.tagged_as_assembled()?;

    // One engine for the whole call, shared with the exclusion lookups it
    // compiles: they run after this call returns, on executions prepared here.
    let engine = Rc::new(NativeSparqlEngine::new());
    let mut streams = Vec::with_capacity(compiled.units.len());
    // The lookups this call compiled, attached once every ranking read is open —
    // see `CandidateIndex` — beside the index of each candidate those reads named.
    // The index is filled only when some stratum compiled a lookup, because it is
    // read by lookups and by nothing else; a read produced on demand fills it as
    // each row is pulled, which is before any lookup can ask about that row's
    // candidate, because the fusion asks only about candidates it has pulled.
    let mut lookups: Vec<PendingLookup> = Vec::new();
    let candidates: Rc<RefCell<CandidateIndex<D::Id>>> = Rc::new(RefCell::new(HashMap::new()));
    let index_candidates = compiled.units.iter().any(StratumUnit::declares_exclusion);
    let mut statuses = HashMap::with_capacity(compiled.units.len());
    // The registry is named identically at prepare and at evaluation: the
    // evaluator refuses a plan prepared against a different registry than the
    // one it is run under, because a plan prepared without one has already
    // lowered every relation's predicate to an ordinary triple pattern. One
    // spelling, called twice, so the two sites cannot drift apart.
    // The environment is built ONCE, here, and both the prepare and the evaluation
    // borrow it: an environment derives the parse configuration and the registry
    // fingerprints the plan cache keys on, and rebuilding it per unit would pay that
    // derivation once per unit for a value that cannot change inside one execution.
    //
    // Shared rather than owned outright because a capable stratum's exclusion
    // lookup outlives this call: the lookup runs per candidate while the fusion
    // is merging, against a plan prepared here, and the evaluator refuses a plan
    // run under a different registry than it was prepared against. One value,
    // one refcount per capable stratum, and no way for the two calls to name
    // different registries.
    let env = Arc::new(ExtensionEnv::over_relations(registry.clone()).map_err(|e| {
        ExecutionError::EnvironmentNotDerivable {
            reason: e.to_string(),
        }
    })?);
    let options = || QueryOptions {
        env: env.as_ref(),
        ..QueryOptions::EMPTY
    };

    for unit in &compiled.units {
        // There is no empty-text arm here, and there is nothing left for one to catch.
        // An empty supplied text is not a query, so `StratumUnit::new` refuses it
        // outright (`UnitError::NotAQuery`), and a rendered text is assembled from a
        // producer's own call. A status with no reachable cause is a status a caller
        // can be told about and never observe, which is why this arm was deleted rather
        // than kept as a guard: the condition it guarded cannot reach a bundle.
        //
        // Rendered once and run once, under either schedule: the depth this loop
        // reads the ending against is the depth that wrote the bound in this text,
        // taken from the unit once, here, so the text, the reach and the rank the
        // ending reports cannot be three readings of two different numbers.
        let depth = unit.probed_depth();
        let sparql = unit.sparql();
        let prepared = match engine.prepare_query_with_options(&sparql, None, options()) {
            Ok(prepared) => prepared,
            Err(diagnostic) => {
                statuses.insert(
                    unit.stratum.clone(),
                    ProducerStatus::ExecutionFailed {
                        reason: diagnostic.to_string(),
                    },
                );
                continue;
            }
        };
        // Prepared before the ranking read runs, and once for the whole stratum —
        // but after the ranking text is prepared, because that prepared plan is what
        // a caller's own text has its lookups derived from (see
        // `StratumUnit::exclusion_sparql`). Each is a second query over the same
        // producer — one of the text's calls with the candidate bound — and the
        // fusion stage runs it per candidate it needs a verdict for, so parsing and
        // feasibility-ordering it here is
        // the difference between a prepared lookup and a re-parse per
        // candidate. A stratum whose lookup will not prepare is a stratum whose
        // declared basis cannot be honoured, and that is this stratum's own
        // failure exactly as a ranking text that will not prepare is: reported
        // here and contributing no stream, rather than carried to a lookup that
        // fails once per candidate at read time.
        //
        // The candidate is DECLARED to the prepare as the execution's parameter, not
        // merely substituted at run time. A lookup is one prepared execution run
        // once per candidate, so the admission pass sees `?candidate` as a free
        // variable unless it is told otherwise — and for a producer that takes a
        // depth, a free candidate beside a free depth is a call no mode serves at
        // all, while a free candidate beside a BOUND depth is the ranked question
        // whose absences are not exclusions. Declaring it is what puts the call in
        // the candidate-bound mode a membership basis is admitted against, and the
        // execution refuses to run with it unbound, so it cannot be run free.
        let exclusion = match unit.exclusion_sparql(&prepared, registry) {
            None => None,
            Some(Err(reason)) => {
                statuses.insert(
                    unit.stratum.clone(),
                    ProducerStatus::ExecutionFailed {
                        reason: format!(
                            "stratum {}: the exclusion lookup its contract declared could not be \
                             derived from its query: {reason}",
                            unit.stratum
                        ),
                    },
                );
                continue;
            }
            Some(Ok(texts)) => {
                let prepared_lookups: Result<Vec<(PreparedExecution, usize)>, _> = texts
                    .iter()
                    .map(|text| {
                        engine
                            .prepare_execution(text, None, &[CANDIDATE_NAME], options())
                            .map(|execution| {
                                let slot = execution
                                    .slot(CANDIDATE_NAME)
                                    .expect("the one parameter this execution was prepared with");
                                (execution, slot)
                            })
                    })
                    .collect();
                match prepared_lookups {
                    Ok(lookups) => Some(lookups),
                    Err(diagnostic) => {
                        statuses.insert(
                            unit.stratum.clone(),
                            ProducerStatus::ExecutionFailed {
                                reason: format!(
                                    "stratum {}: the exclusion lookup its producer declared \
                                     could not be prepared: {diagnostic}",
                                    unit.stratum
                                ),
                            },
                        );
                        continue;
                    }
                }
            }
        };
        // On demand: the same prepared text, opened as one invocation held open, and
        // read as its stream is pulled. No status is recorded, because how this read
        // ends is decided by how far its consumer reads it; see `execute_within`.
        //
        // Decided by the prepared plan's shape, never by who wrote the text: a
        // caller's own text that is one call under row-for-row operators is exactly
        // one invocation, and one that is anything else is read materialised below.
        if schedule == ReadSchedule::OnDemand && prepared.is_call_read() {
            let opened = engine
                .open_call_cursor(&prepared, options())
                .map_err(|diagnostic| diagnostic.to_string())
                .and_then(|cursor| {
                    CandidateColumns::locate(cursor.variables(), &unit.contract.domains)
                        .map(|columns| (cursor, columns))
                        .map_err(|reason| format!("stratum {}: {reason}", unit.stratum))
                });
            match opened {
                Ok((cursor, columns)) => {
                    let attestation = cursor.opened().clone();
                    let read = CallRead {
                        stratum: unit.stratum.clone(),
                        cursor,
                        columns,
                        depth: depth.get(),
                        declared_rows: unit.declared_rows(),
                        declared_mode: unit.declared_mode(),
                        reach: unit.reach(),
                        materialised: 0,
                        candidates: index_candidates.then(|| Rc::clone(&candidates)),
                        dataset,
                    };
                    if let Some(prepared_lookups) = exclusion {
                        lookups.push(PendingLookup {
                            position: streams.len(),
                            stratum: unit.stratum.clone(),
                            lookups: prepared_lookups,
                            pinned: attestation.clone(),
                        });
                    }
                    streams.push(StratumStream {
                        stratum: unit.stratum.clone(),
                        plan_id: compiled.plan_id,
                        fused_bound: compiled.fused_bound,
                        contract: unit.contract.clone(),
                        attestation,
                        stream: RankedStreamImpl::on_demand(Box::new(read)),
                    });
                }
                Err(reason) => {
                    statuses.insert(
                        unit.stratum.clone(),
                        ProducerStatus::ExecutionFailed { reason },
                    );
                }
            }
            continue;
        }
        // Governors are per call, never per engine: the value is built here,
        // used once, and dropped with the call. `UNBOUNDED` declines every
        // caller-settable ceiling, so this is the governed lane for its receipt
        // and for nothing else — see this module's header.
        let outcome = engine.query_prepared_governed_view(
            dataset,
            &prepared,
            &[],
            options(),
            &QueryGovernors::UNBOUNDED,
        );
        match outcome {
            Ok(GovernedOutcome::Complete {
                result:
                    SparqlResult::Solutions {
                        variables, rows, ..
                    },
                relations,
                ..
            }) => {
                // Read before ranking. The witness describes the run that just
                // happened, so an inconsistency in it invalidates that run
                // whether or not this stratum's rows could also be ranked; the
                // alternative would let a per-stratum ranking failure mask the
                // whole-run condition that caused it.
                let attestation =
                    read_attestation(&relations.witness, rows.is_empty()).map_err(|reason| {
                        ExecutionError::InconsistentWitness {
                            stratum: Box::new(unit.stratum.clone()),
                            reason,
                        }
                    })?;
                match rank_candidates(&variables, &rows, &unit.contract.domains) {
                    Ok(ranked) => {
                        if index_candidates {
                            index_candidates_into(
                                &mut candidates.borrow_mut(),
                                &variables,
                                &rows,
                                &ranked,
                                dataset,
                            );
                        }
                        // `?`, not a per-stratum status: a producer that beat
                        // its own declaration broke the number every other
                        // stratum's admission and ordering rested on, and the
                        // only ending left for this one is a completeness claim
                        // the extra row has already falsified.
                        let (ranked, ending, status) = bound_to_depth(
                            ranked,
                            depth.get(),
                            unit.declared_rows(),
                            unit.declared_mode(),
                            unit.reach(),
                            &unit.stratum,
                        )?;
                        // The read's own cost, taken from the evaluator's
                        // answer rather than from the rows that survived the
                        // depth: `bound_to_depth` has just taken the probe row
                        // off, and a figure read after it would report every
                        // bounded read as one row cheaper than it was.
                        let materialised = u64::try_from(rows.len()).unwrap_or(u64::MAX);
                        let stream = RankedStreamImpl::new(ranked, ending)
                            .with_materialised_rows(materialised)
                            .with_attested(attestation.clone());
                        if let Some(prepared_lookups) = exclusion {
                            lookups.push(PendingLookup {
                                position: streams.len(),
                                stratum: unit.stratum.clone(),
                                lookups: prepared_lookups,
                                pinned: attestation.clone(),
                            });
                        }
                        streams.push(StratumStream {
                            stratum: unit.stratum.clone(),
                            plan_id: compiled.plan_id,
                            fused_bound: compiled.fused_bound,
                            contract: unit.contract.clone(),
                            attestation,
                            stream,
                        });
                        statuses.insert(unit.stratum.clone(), status);
                    }
                    Err(reason) => {
                        statuses.insert(
                            unit.stratum.clone(),
                            ProducerStatus::ExecutionFailed {
                                reason: format!("stratum {}: {reason}", unit.stratum),
                            },
                        );
                    }
                }
            }
            Ok(GovernedOutcome::Complete { .. }) => {
                statuses.insert(
                    unit.stratum.clone(),
                    ProducerStatus::ExecutionFailed {
                        reason: "compiled unit did not return solutions".to_owned(),
                    },
                );
            }
            Ok(GovernedOutcome::BudgetExhausted(exhausted)) => {
                // A required arm over an outcome no unit on this lane can reach
                // — the enum is exhaustive by design, so this case is handled
                // here or nowhere. `UNBOUNDED` declines every caller-settable
                // ceiling and carries no stop signal, and these options inject
                // no user-function registry, so the one ceiling that remains has
                // no charge site a unit can enter; see `ExecutionError` for the
                // full argument and the tests that pin both halves.
                //
                // What it must not do is take the partial rows: a trip means
                // something other than this call's governors stopped the run,
                // and reporting its rows as a stratum's answer would certify a
                // truncated read as an exhausted one. So it is the whole-run
                // refusal it would be, naming the governor that did it.
                return Err(ExecutionError::InconsistentWitness {
                    stratum: Box::new(unit.stratum.clone()),
                    reason: format!(
                        "the run declined every ceiling, yet {} stopped it",
                        exhausted.tripped
                    ),
                });
            }
            Err(diagnostic) => {
                statuses.insert(
                    unit.stratum.clone(),
                    ProducerStatus::ExecutionFailed {
                        reason: diagnostic.to_string(),
                    },
                );
            }
        }
    }

    for PendingLookup {
        position,
        stratum,
        lookups,
        pinned,
    } in lookups
    {
        streams[position]
            .stream
            .attach_exclusion(Box::new(DatasetExclusion {
                stratum,
                engine: Rc::clone(&engine),
                env: Arc::clone(&env),
                lookups,
                candidates: Rc::clone(&candidates),
                dataset,
                pinned,
            }));
    }

    Ok(ExecutionResult { streams, statuses })
}

/// Record how each of `ranked`'s candidates is bound into an exclusion lookup —
/// see [`CandidateBinding`].
///
/// `ranked` is [`rank_candidates`]' answer over `rows`, one entry per row in row
/// order, so the two are zipped rather than the lexical being rendered a second
/// time. A candidate several strata named is recorded once: the dataset's id for a
/// term does not depend on which read found it.
fn index_candidates_into<D: DatasetView>(
    index: &mut CandidateIndex<D::Id>,
    variables: &[String],
    rows: &[Vec<Option<TermValue>>],
    ranked: &[(u64, Term, RowBlock)],
    dataset: &D,
) {
    let Some(column) = variables.iter().position(|name| name == CANDIDATE_NAME) else {
        return;
    };
    for ((_, term, _), row) in ranked.iter().zip(rows) {
        if let Some(value) = row.get(column).and_then(Option::as_ref) {
            index_candidate(index, term, value, dataset);
        }
    }
}

/// Record how one candidate, named `term` and read as `value`, is bound into an
/// exclusion lookup — the dataset's own id where the dataset holds it, the value
/// where it does not. A candidate already recorded is left as it is: the dataset's
/// id for a term does not depend on which read found it.
fn index_candidate<D: DatasetView>(
    index: &mut CandidateIndex<D::Id>,
    term: &Term,
    value: &TermValue,
    dataset: &D,
) {
    if index.contains_key(term) {
        return;
    }
    let binding = dataset.term_id_by_value(value).map_or_else(
        || CandidateBinding::Value(value.clone()),
        CandidateBinding::Id,
    );
    index.insert(term.clone(), binding);
}

/// What one stratum's read came to: the rows that reach the stream, how the read
/// ended, and the status that mirrors that ending.
///
/// The three are produced together by [`bound_to_depth`] and must stay together:
/// the ending is a claim about the rows beside it, and a caller holding one
/// without the others could report an exhaustion that the dropped probe row had
/// already falsified.
type BoundedRead = (Vec<(u64, Term, RowBlock)>, StreamEnding, ProducerStatus);

/// Cut `ranked` down to the stratum's `depth`, and say which ending that was.
///
/// The unit was emitted one row deeper than `depth`, so a `depth + 1`-th row
/// here means the read still had a row when the bound ran out. That row is
/// dropped — it is a probe and never a value — and its only effect on the
/// *answer* is the ending. It is still a row the read paid for, so it survives
/// in [`RankedStreamImpl::rows_materialised`], which this function's caller sets
/// from the rows the evaluator returned rather than from the rows it kept. Every
/// other row keeps the rank [`rank_candidates`] gave it, so nothing is
/// renumbered.
///
/// The status returned is the mirror of the ending, so the terminal report and
/// the stream's own receipt cannot say different things about the same read.
///
/// # The three endings, and which observation each one rests on
///
/// * `depth + 1` rows came back — the read had more, and the planned depth is what
///   stopped it: `DepthReached`.
/// * fewer rows than the read was allowed came back — the producer stopped before
///   anything stopped it: `Exhausted`, verified.
/// * exactly `depth` rows came back and the read was allowed no more than `depth`
///   ([`ReadReach::AtDepth`]) — the producer's own declared row bound is the
///   stopper, and whether a further row exists was not observable:
///   `RowBoundReached`. `reach` is the only thing that distinguishes this from the
///   second case, which is why it travels on the unit rather than being guessed
///   from the row count.
/// * the read came back inside the bound at all and the text was a *caller's*
///   ([`ReadReach::Unknown`]) — the stopper is that text, and this layer can see
///   neither what it bounded nor therefore what it left unread:
///   `SuppliedQueryEnded`. It is decided after the row past the depth is looked for,
///   because that row's arrival is an observation no text can take away, and before
///   either of the other two, because neither of them is knowable once the query is
///   not this layer's.
///
/// # Errors
///
/// [`ExecutionError::RowBoundBreached`] where the extra row is past
/// `declared_rows` rather than merely past `depth`. Those are different facts
/// and `declared_rows` is what tells them apart: a row past the depth and below
/// the declaration is the bound the plan recorded doing its job, while a row
/// past the declaration is the producer contradicting the registry — and
/// reporting the second as the first would truncate a read that had more rows
/// behind it and certify the remainder as exhaustion.
///
/// A stratum whose registry declared no bound carries no promise for a row to
/// break, so its probe is always the ordinary `DepthReached`.
///
/// The comparison is made against the declaration before the depth is consulted,
/// because a declared **zero** is the one declaration the depth is read *wider*
/// than: the floor of one is what lets such a producer report its own emptiness,
/// so the emitted bound asks for two rows and the first one back is already more
/// than the declaration allows. Compared only from inside the depth arm, that row
/// counted as within the depth and earned a certified exhaustion.
///
/// The `depth + 1`-th row can arrive at every depth this function can be called
/// with: the compiler emits a bound strictly deeper than the depth, which it can
/// do for every depth because the waist refuses the one depth whose probe row a
/// 32-bit bound cannot express. Without that refusal this comparison would be
/// unsatisfiable at exactly that depth, and every such read — cut or not — would
/// leave here as `Exhausted`.
fn bound_to_depth(
    mut ranked: Vec<(u64, Term, RowBlock)>,
    depth: u32,
    declared_rows: Option<u64>,
    declared_mode: BoundMode,
    reach: ReadReach,
    stratum: &Iri,
) -> Result<BoundedRead, ExecutionError> {
    let ceiling = usize::try_from(depth).unwrap_or(usize::MAX);
    let pulled = u64::try_from(ranked.len()).unwrap_or(u64::MAX);
    // Consulted against the declaration alone, before the depth is looked at,
    // because the declaration and the depth are not the same ceiling. They coincide
    // at every declaration the waist admits a depth *inside*, where the only row
    // that can breach is the probe past the depth — which is why this used to sit
    // inside the arm below. At a declared **zero** they do not coincide: the depth
    // is read at the floor of one, so the emitted bound asks for two and the FIRST
    // row already returns more than the producer promised. Judged from inside the
    // depth arm, that row was within the depth and so never compared, and a
    // producer that declared an empty index and then named one candidate was
    // certified `Exhausted { rows_emitted: 1 }` — a completeness claim over a read
    // whose declaration it had already broken.
    //
    // Hoisting it refuses nothing that was admitted before: the waist holds every
    // recorded depth at or below the declaration, so a read inside its depth can
    // exceed the declaration only where the floor widened one, and the floored row
    // is the probe that lets an honestly empty producer report `Exhausted { 0 }`
    // — which it still does, because zero is not above zero.
    if let Some(declared) = declared_rows
        && pulled > declared
    {
        return Err(ExecutionError::RowBoundBreached {
            stratum: Box::new(stratum.clone()),
            declared,
            pulled,
            // Read off the unit rather than re-derived: the mode the number was taken
            // at is the waist's fact about the declaration the unit was compiled
            // against, and the invocation it belongs to is three stages upstream of
            // here.
            mode: declared_mode,
        });
    }
    if ranked.len() > ceiling {
        ranked.truncate(ceiling);
        let rank = u64::from(depth);
        return Ok((
            ranked,
            StreamEnding::DepthReached { rank },
            ProducerStatus::DepthReached { rank },
        ));
    }
    // A read of a text this layer did not write has an ending nobody observed either,
    // and the honest report names that text. The probe slot may never have existed:
    // the bound this layer renders is only the outermost one, and a `LIMIT` inside a
    // caller's sub-`SELECT` decides the read before it is reached.
    if reach == ReadReach::Unknown {
        let rank = u64::try_from(ranked.len()).unwrap_or(u64::MAX);
        return Ok((
            ranked,
            StreamEnding::SuppliedQueryEnded { rank },
            ProducerStatus::SuppliedQueryEnded { rank },
        ));
    }
    // A read that filled a depth it could not be taken past has an ending nobody
    // observed, and the honest report names the bound that made the row past it
    // unaskable. `Exhausted` here would be the one ending that names no
    // stopper, minted from the producer's registration rather than from a read.
    if reach == ReadReach::AtDepth && ranked.len() == ceiling {
        let rank = u64::from(depth);
        return Ok((
            ranked,
            StreamEnding::RowBoundReached { rank },
            ProducerStatus::RowBoundReached { rank },
        ));
    }
    let rows_emitted = u64::try_from(ranked.len()).unwrap_or(u64::MAX);
    Ok((
        ranked,
        StreamEnding::Exhausted,
        ProducerStatus::Exhausted { rows_emitted },
    ))
}

/// The attestation a compiled unit's run left on the governed receipt.
///
/// `empty_answer` is whether the unit returned no solution row at all, and it
/// decides exactly one thing: whether an empty witness is a refusal. A unit that
/// returned rows must have reached its relation to obtain them, so a witness
/// naming nothing contradicts the rows in hand. A unit that returned nothing has
/// nothing for an attestation to be *about*, and refusing it would fail the whole
/// run over a stratum that legitimately answered with nothing — the over-refusal
/// mirror of the silent drop. [`PfAttestation::UNDECLARED`] is the true report
/// there: nobody said anything.
///
/// Everything else is [`sole_attestation`]'s rule, unchanged.
fn read_attestation(
    witness: &RelationWitness,
    empty_answer: bool,
) -> Result<PfAttestation, String> {
    if empty_answer && witness.is_empty() {
        return Ok(PfAttestation::UNDECLARED);
    }
    sole_attestation(witness)
}

/// The one attestation a conforming compiled unit's witness holds, or why it
/// cannot be one.
///
/// The rule, and the reason it is the rule, is in this module's header. In
/// short: one unit, one producer, constant arguments — therefore one relation,
/// one pinned generation, at most one incompleteness reason. Each refusal names
/// the count that was wrong, because the count is what a caller needs in order
/// to tell "the registry grew a second producer for this stratum" from "the
/// index rebuilt under the query".
///
/// The invocation **count** is read and deliberately ignored. It is a function
/// of how the evaluator chunked its driving rows, not of what was attested, and
/// a rule keyed on it would fail on an input size rather than on a defect.
fn sole_attestation(witness: &RelationWitness) -> Result<PfAttestation, String> {
    let mut entries = witness.iter();
    let (relation, attested) = match (entries.next(), entries.next()) {
        (None, _) => {
            return Err(
                "0 relations attested, but a compiled unit calls exactly one registered \
                 relation, so its run must attest exactly 1"
                    .to_owned(),
            );
        }
        (Some(_), Some(_)) => {
            return Err(format!(
                "{} relations attested, but a compiled unit binds exactly 1 producer",
                witness.len()
            ));
        }
        (Some(sole), None) => sole,
    };
    if attested.generations.len() > 1 {
        return Err(format!(
            "relation {relation} attested {} distinct index generations, but one invocation \
             pins exactly 1: the index moved under the query",
            attested.generations.len()
        ));
    }
    let Some(generation) = attested.generations.first() else {
        return Err(format!(
            "relation {relation} attested 0 index generations, but every recorded invocation \
             contributes exactly 1, even when it declares nothing"
        ));
    };
    if attested.incompleteness.len() > 1 {
        return Err(format!(
            "relation {relation} attested {} distinct incompleteness reasons, but one \
             invocation can give at most 1",
            attested.incompleteness.len()
        ));
    }
    let service = attested
        .incompleteness
        .first()
        .map_or(ServiceLevel::Undeclared, |reason| {
            ServiceLevel::Incomplete {
                reason: reason.clone(),
            }
        });
    Ok(PfAttestation {
        generation: generation.clone(),
        service,
    })
}

/// Read a unit's projected candidate and block columns into ranked
/// `(rank, candidate, block)` rows.
///
/// # The column is found by name
///
/// A unit projects exactly one variable, `?candidate`, so the column is located
/// by that name. Reading a row's first *bound* cell instead would make the
/// candidate depend on projection order, and a unit that projected anything
/// alongside the candidate would silently rank the wrong term.
///
/// # No row is ever dropped
///
/// A rank is a position in the stratum's answer, so discarding a row renumbers
/// every row after it: the stratum would report a shorter, differently-ranked
/// list that still looked complete. An unbound cell and an unrenderable value are
/// therefore both refusals of the whole unit, returned as the reason the caller
/// records as this stratum's [`ProducerStatus::ExecutionFailed`].
fn rank_candidates(
    variables: &[String],
    rows: &[Vec<Option<TermValue>>],
    domains: &CandidateDomains,
) -> Result<Vec<(u64, Term, RowBlock)>, String> {
    let columns = CandidateColumns::locate(variables, domains)?;
    let mut ranked = Vec::with_capacity(rows.len());
    for (index, row) in rows.iter().enumerate() {
        let rank = u64::try_from(index + 1).unwrap_or(u64::MAX);
        ranked.push(columns.rank(row, rank)?);
    }
    Ok(ranked)
}

/// Where a unit's solutions carry the candidate and the block, found once per read
/// and applied to every row of it — materialised or produced on demand alike, so the
/// two schedules rank a row by one reading of it.
#[derive(Debug)]
struct CandidateColumns {
    /// The `?candidate` column.
    candidate: usize,
    /// The `?block` column, present exactly when the producer declared a position to
    /// read a block out of, because that is the only case `compile` projects it.
    block: Option<usize>,
    /// The block every row carries when the unit projects no block column: a
    /// derivation rather than a default, see [`entailed_block`].
    entailed: RowBlock,
}

impl CandidateColumns {
    /// Find the columns by name among `variables`, for a producer that declared
    /// `domains`.
    ///
    /// Found by name rather than by position, because reading a position would make
    /// the candidate and the block depend on projection order.
    fn locate(variables: &[String], domains: &CandidateDomains) -> Result<Self, String> {
        let candidate = variables
            .iter()
            .position(|name| name == CANDIDATE_NAME)
            .ok_or_else(|| {
                format!("the unit's solutions project no ?{CANDIDATE_NAME} column: {variables:?}")
            })?;
        Ok(Self {
            candidate,
            block: variables.iter().position(|name| name == BLOCK_NAME),
            entailed: entailed_block(domains),
        })
    }

    /// The ranked `(rank, candidate, block)` reading of one solution row at `rank`.
    fn rank(&self, row: &[Option<TermValue>], rank: u64) -> Result<(u64, Term, RowBlock), String> {
        let value = row
            .get(self.candidate)
            .and_then(Option::as_ref)
            .ok_or_else(|| {
                format!("the projected ?{CANDIDATE_NAME} column is unbound in row {rank}")
            })?;
        let block = match self.block {
            None => self.entailed.clone(),
            Some(column) => row_block(row.get(column).and_then(Option::as_ref), rank)?,
        };
        let candidate = term_candidate(value).map_err(|error| {
            format!("the projected ?{CANDIDATE_NAME} column in row {rank} names no term: {error}")
        })?;
        Ok((rank, candidate, block))
    }

    /// The candidate cell of `row`, when it is bound.
    fn candidate_of<'r>(&self, row: &'r [Option<TermValue>]) -> Option<&'r TermValue> {
        row.get(self.candidate).and_then(Option::as_ref)
    }
}

/// One stratum's invocation held open by [`execute_within`] under
/// [`ReadSchedule::OnDemand`], read a row per pull.
///
/// Every observation [`bound_to_depth`] makes over a materialised read is made here,
/// in the same order, one row at a time: the declared row bound is checked against
/// every row the invocation returns before anything else is decided about it; a row
/// past the planned depth is the probe, never emitted, and ends the read
/// `DepthReached`; a producer that runs out ends it `Exhausted`, or
/// `RowBoundReached` where it filled a depth it could not be asked past.
struct CallRead<'d, D: DatasetView + Sync> {
    /// The stratum this read is for.
    stratum: Iri,
    /// The open invocation.
    cursor: CallCursor,
    /// Where the invocation's solutions carry the candidate and the block.
    columns: CandidateColumns,
    /// The planned depth: the most rows this stratum may contribute.
    depth: u32,
    /// The registry's declared row bound for the producer, held against every row.
    declared_rows: Option<u64>,
    /// Which declared mode `declared_rows` was read at, for the refusal naming it.
    declared_mode: BoundMode,
    /// How far past `depth` this read can reach.
    reach: ReadReach,
    /// The rows the invocation has returned, the probe row included once read.
    materialised: u64,
    /// The index an exclusion lookup binds candidates through, when any stratum of
    /// this execution compiled one.
    candidates: Option<Rc<RefCell<CandidateIndex<D::Id>>>>,
    /// The caller's dataset, which a candidate is resolved against for that index.
    dataset: &'d D,
}

impl<D: DatasetView + Sync> fmt::Debug for CallRead<'_, D> {
    /// Names the stratum and the read's position; a dataset's `Debug` is unbounded.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CallRead")
            .field("stratum", &self.stratum.as_str())
            .field("depth", &self.depth)
            .field("materialised", &self.materialised)
            .finish_non_exhaustive()
    }
}

impl<D: DatasetView + Sync> OnDemandRead for CallRead<'_, D> {
    fn stratum(&self) -> &str {
        self.stratum.as_str()
    }

    fn pull(&mut self) -> Result<ReadStep, ReadFault> {
        let isolated = |reason: String| ReadFault {
            reason,
            whole_run: false,
        };
        let Some(row) = self
            .cursor
            .next_row(self.dataset)
            .map_err(|error| isolated(error.to_string()))?
        else {
            // Ran out. A caller's own text is the stopper nobody observed past, and
            // the ending names it, exactly as `bound_to_depth` does: a `LIMIT` inside
            // that text may have cut the read before this layer's bound was reached.
            // Filling a depth the read could not be taken past is the producer's own
            // bound stopping it, which nobody could observe past either; anything
            // else is a verified exhaustion.
            let ending = match self.reach {
                ReadReach::Unknown => StreamEnding::SuppliedQueryEnded {
                    rank: self.materialised,
                },
                ReadReach::AtDepth if self.materialised == u64::from(self.depth) => {
                    StreamEnding::RowBoundReached {
                        rank: u64::from(self.depth),
                    }
                }
                ReadReach::AtDepth | ReadReach::PastDepth => StreamEnding::Exhausted,
            };
            return Ok(ReadStep::Ended(ending));
        };
        self.materialised += 1;
        if let Some(declared) = self.declared_rows
            && self.materialised > declared
        {
            return Err(ReadFault {
                reason: ExecutionError::RowBoundBreached {
                    stratum: Box::new(self.stratum.clone()),
                    declared,
                    pulled: self.materialised,
                    mode: self.declared_mode,
                }
                .to_string(),
                whole_run: true,
            });
        }
        if self.materialised > u64::from(self.depth) {
            // The probe row: never emitted, and its arrival is the ending.
            return Ok(ReadStep::Ended(StreamEnding::DepthReached {
                rank: u64::from(self.depth),
            }));
        }
        let ranked = self
            .columns
            .rank(&row, self.materialised)
            .map_err(|reason| isolated(format!("stratum {}: {reason}", self.stratum)))?;
        if let Some(index) = &self.candidates
            && let Some(value) = self.columns.candidate_of(&row)
        {
            index_candidate(&mut index.borrow_mut(), &ranked.1, value, self.dataset);
        }
        Ok(ReadStep::Row(ranked))
    }

    fn materialised(&self) -> u64 {
        self.materialised
    }

    fn settle(&self) -> Result<PfAttestation, String> {
        let witness = self.cursor.settle().map_err(|error| error.to_string())?;
        sole_attestation(&witness)
    }
}

/// The block every row of a producer that names none itself lies in, read off the
/// declaration.
///
/// This is an **entailment**, not a default. A producer declaring exactly one
/// block has already said that every candidate it names lies in that block, so
/// naming it per row states nothing the declaration did not, and a host is not
/// asked to repeat itself on every row of every stratum. That is the common
/// configuration and the one the whole declared-domain mechanism exists for — one
/// producer per block, blocks that do not overlap.
///
/// Everything else is [`RowBlock::Undeclared`], because nothing else is derivable:
///
/// * [`CandidateDomains::Unrestricted`] restricts nothing, so there is no block to
///   entail and none is owed;
/// * a restriction naming **several** blocks has not said which of them a given
///   row is in, and this layer may not choose — a guess would place a candidate in
///   a block the producer never claimed and could refuse a perfectly good corpus
///   as self-contradictory. The consequence is the honest one: fusion refuses a
///   restriction no row backs
///   ([`ProtocolError::UnbackedDomainDeclaration`](crate::ProtocolError)), and the
///   host's exits are to declare a block column, to register one producer per
///   block, or to declare `Unrestricted`.
fn entailed_block(domains: &CandidateDomains) -> RowBlock {
    match domains.tags() {
        Some(tags) if tags.len() == 1 => tags
            .iter()
            .next()
            .map_or(RowBlock::Undeclared, |tag| RowBlock::Declared(tag.clone())),
        Some(_) | None => RowBlock::Undeclared,
    }
}

/// Read one row's projected block column as the block that row was drawn from.
///
/// A block is a [`DomainTag`], which is an IRI, so the cell must be a bound IRI
/// and nothing else. Both failures refuse the **whole unit** rather than the row,
/// for the reason an unbound candidate does: a rank is a position in the stratum's
/// answer, so dropping one row renumbers every row after it and the stratum would
/// report a shorter, differently-ranked list that still looked complete. A
/// producer that cannot name a block for one of its rows has not declared a
/// narrower domain — it has declared one it cannot back, and that is refused where
/// it is observed.
fn row_block(value: Option<&TermValue>, rank: u64) -> Result<RowBlock, String> {
    let value = value.ok_or_else(|| {
        format!(
            "the projected ?{BLOCK_NAME} column is unbound in row {rank}, so that row names \
                 no block of the candidate universe; a producer that declares a block column owes \
                 a block on every row"
        )
    })?;
    match value {
        // Parsed, never trusted: the tag reaches a consumer that compares it
        // against a host's declared blocks, and a cell that is not an IRI cannot
        // be one of those. `DomainTag::parse` is the same validation the registry
        // applied to the declaration, so the two sides are compared after one
        // rule rather than two.
        TermValue::Iri(text) => DomainTag::parse(text)
            .map(RowBlock::Declared)
            .map_err(|error| {
                format!(
                    "the projected ?{BLOCK_NAME} column in row {rank} is <{text}>, which is not a \
                 valid IRI and so names no block: {error}"
                )
            }),
        other => Err(format!(
            "the projected ?{BLOCK_NAME} column in row {rank} is {other:?}, and a block of the \
             candidate universe is named by an IRI"
        )),
    }
}

/// A candidate's canonical term lexical — exactly the spelling a caller uses for
/// a seed.
///
/// [`candidate_lexical`] writes `<http://example.org/doc>`, `"lex"@en` or the RDF 1.2
/// triple term `<<( s p o )>>`: deterministic, injective, target-independent, and
/// invertible by the decoder that lives beside it. A candidate is therefore
/// spelled exactly as [`RequestTerm::EntitySeed`](crate::RequestTerm::EntitySeed)
/// spells a seed, so a row this layer returns can be fed straight back in as the
/// seed of a follow-up request.
///
/// A blank node is named `_:label` rather than refused. Its label is
/// dataset-local, so it cannot seed a *later* request — but that is refused at
/// placement, where a caller actually tries it, and the decoder reads `_:label`
/// back either way. Refusing here would discard every other row in the stratum
/// over one answer the layer merely declined to write down.
///
/// A literal no concrete syntax can spell — a base direction with no language tag,
/// or a tag that is not a `LANGTAG` — is refused: it is not well-formed RDF, and
/// dropping the tag or direction to write it would name the plain literal of the
/// same lexical form instead, a different term.
fn term_candidate(value: &TermValue) -> Result<Term, RenderError> {
    candidate_lexical(value).map(Term::new)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use purrdf_core::{RdfTextDirection, TermValue};
    use purrdf_sparql_eval::{
        BindingPattern, CandidateDomains, DomainTag, IndexGeneration, PfAttestation,
        RelationWitness, ServiceLevel,
    };

    use super::{
        BoundMode, ExecutionError, ProducerStatus, RowBlock, StreamEnding, bound_to_depth,
        entailed_block, rank_candidates, read_attestation, sole_attestation, term_candidate,
    };
    use crate::compile::{BLOCK_NAME, CANDIDATE_NAME, ReadReach};
    use crate::iri::{Iri, Term};
    use crate::render::decode_term;

    fn variables() -> Vec<String> {
        vec![CANDIDATE_NAME.to_owned()]
    }

    #[test]
    fn a_candidate_is_the_canonical_lexical_and_decodes_back_to_its_value() {
        for value in [
            TermValue::iri("http://example.org/doc"),
            TermValue::simple_literal("quick brown fox"),
            TermValue::typed_literal("3", "http://www.w3.org/2001/XMLSchema#integer"),
            TermValue::Triple {
                s: Box::new(TermValue::iri("http://example.org/s")),
                p: Box::new(TermValue::iri("http://example.org/p")),
                o: Box::new(TermValue::simple_literal("o")),
            },
        ] {
            let candidate = term_candidate(&value).expect("the fixture value is well-formed");
            assert_eq!(
                decode_term(candidate.as_str()),
                Ok(value),
                "the candidate {} reads back as the value it names",
                candidate.as_str()
            );
        }
        assert_eq!(
            term_candidate(&TermValue::iri("http://example.org/doc"))
                .expect("an IRI is well-formed")
                .as_str(),
            "<http://example.org/doc>",
            "an IRI candidate is legible, not an encoding of one"
        );
    }

    /// A blank node is a perfectly ordinary answer. It cannot seed a *later*
    /// request, because its label is dataset-local — but that is refused at
    /// placement, where a caller actually tries it. Refusing it here would throw
    /// away every other row in the stratum over one answer this layer merely
    /// declined to write down, which is the silent-drop bug wearing strictness.
    #[test]
    fn a_blank_candidate_is_named_and_does_not_take_its_stratum_down() {
        let ranked = rank_candidates(
            &variables(),
            &[
                vec![Some(TermValue::iri("http://example.org/doc"))],
                vec![Some(TermValue::blank("b0"))],
                vec![Some(TermValue::iri("http://example.org/other"))],
            ],
            &CandidateDomains::Unrestricted,
        )
        .expect("a blank node among the answers is still an answer");

        assert_eq!(
            ranked
                .iter()
                .map(|(_, term, _)| term.as_str())
                .collect::<Vec<_>>(),
            vec![
                "<http://example.org/doc>",
                "_:b0",
                "<http://example.org/other>"
            ],
            "the blank node is named in place, and its neighbours keep their ranks"
        );
        assert_eq!(
            ranked.iter().map(|(rank, ..)| *rank).collect::<Vec<_>>(),
            vec![1, 2, 3],
            "ranks stay 1-based and contiguous, so no row was dropped"
        );
        assert_eq!(
            decode_term("_:b0"),
            Ok(TermValue::blank("b0")),
            "and the decoder reads the label back, so nothing is lost"
        );
    }

    /// A literal no concrete syntax can spell is refused rather than written without
    /// its tag, which would name the plain literal of the same lexical form — a
    /// different term. Its well-formed neighbours, a tagged and a directional
    /// literal, and a blank node nested in a triple term, are all named and read
    /// back to the value they came from.
    #[test]
    fn an_unspellable_literal_is_refused_and_its_well_formed_neighbours_are_named() {
        let malformed_tag = TermValue::Literal {
            lexical_form: "chat".to_owned(),
            datatype: "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString".to_owned(),
            language: Some("en_us".to_owned()),
            direction: None,
        };
        let untagged_direction = TermValue::Literal {
            lexical_form: "chat".to_owned(),
            datatype: "http://www.w3.org/1999/02/22-rdf-syntax-ns#dirLangString".to_owned(),
            language: None,
            direction: Some(RdfTextDirection::Rtl),
        };
        for (value, nested) in [
            (&malformed_tag, false),
            (&untagged_direction, false),
            (&malformed_tag, true),
        ] {
            let value = if nested {
                TermValue::Triple {
                    s: Box::new(TermValue::blank("b0")),
                    p: Box::new(TermValue::iri("http://example.org/p")),
                    o: Box::new(value.clone()),
                }
            } else {
                value.clone()
            };
            let refused = rank_candidates(
                &variables(),
                &[vec![Some(value)]],
                &CandidateDomains::Unrestricted,
            )
            .expect_err("an unspellable literal is refused, not misnamed");
            assert!(
                refused.contains("row 1 names no term"),
                "the refusal names the row: {refused}"
            );
        }

        let tagged = TermValue::lang_literal("chat", "en-us");
        let directional = TermValue::Literal {
            lexical_form: "chat".to_owned(),
            datatype: "http://www.w3.org/1999/02/22-rdf-syntax-ns#dirLangString".to_owned(),
            language: Some("ar".to_owned()),
            direction: Some(RdfTextDirection::Rtl),
        };
        let nested_blank = TermValue::Triple {
            s: Box::new(TermValue::blank("b0")),
            p: Box::new(TermValue::iri("http://example.org/p")),
            o: Box::new(tagged.clone()),
        };
        let ranked = rank_candidates(
            &variables(),
            &[
                vec![Some(tagged.clone())],
                vec![Some(directional.clone())],
                vec![Some(nested_blank.clone())],
            ],
            &CandidateDomains::Unrestricted,
        )
        .expect("well-formed literals and a nested blank node are named");
        let named: Vec<&str> = ranked.iter().map(|(_, term, _)| term.as_str()).collect();
        assert_eq!(
            named,
            vec![
                "\"chat\"@en-us",
                "\"chat\"@ar--rtl",
                "<<( _:b0 <http://example.org/p> \"chat\"@en-us )>>",
            ],
            "each well-formed value keeps its tag, its direction and its nested blank"
        );
        for (text, value) in named.iter().zip([tagged, directional, nested_blank]) {
            assert_eq!(
                decode_term(text),
                Ok(value),
                "{text} reads back to its value"
            );
        }
    }

    #[test]
    fn the_candidate_column_is_read_by_name_not_by_first_binding() {
        // A row whose earlier column is bound and whose candidate column is not
        // is a refusal, never the earlier column's term promoted into the rank.
        let variables = vec!["other".to_owned(), CANDIDATE_NAME.to_owned()];
        let rows = vec![vec![Some(TermValue::iri("http://example.org/other")), None]];
        let reason = rank_candidates(&variables, &rows, &CandidateDomains::Unrestricted)
            .expect_err("an unbound candidate column is a refusal");
        assert!(reason.contains("unbound"), "{reason}");

        // Bound in the candidate column, it is that column that ranks.
        let rows = vec![vec![
            Some(TermValue::iri("http://example.org/other")),
            Some(TermValue::iri("http://example.org/doc")),
        ]];
        assert_eq!(
            rank_candidates(&variables, &rows, &CandidateDomains::Unrestricted)
                .expect("the candidate column ranks"),
            vec![(
                1,
                Term::new("<http://example.org/doc>".to_owned()),
                RowBlock::Undeclared
            )]
        );
    }

    #[test]
    fn a_unit_that_projects_no_candidate_column_is_refused() {
        let reason = rank_candidates(
            &["other".to_owned()],
            &[vec![None]],
            &CandidateDomains::Unrestricted,
        )
        .expect_err("a unit with no candidate column cannot be ranked");
        assert!(reason.contains(CANDIDATE_NAME), "{reason}");
    }

    // -----------------------------------------------------------------------
    // Each row's block: entailed from the declaration, or read from the column
    // -----------------------------------------------------------------------

    fn block_tag(suffix: &str) -> DomainTag {
        DomainTag::parse(&format!("http://example.org/domain/{suffix}"))
            .expect("fixture domain tags are valid IRIs")
    }

    /// A declaration naming exactly one block answers per row by itself, and no
    /// other declaration answers at all. The negative halves are the point: a
    /// guess for a several-block declaration would place a candidate in a block
    /// its producer never claimed.
    #[test]
    fn a_single_block_declaration_entails_every_rows_block_and_nothing_else_does() {
        assert_eq!(
            entailed_block(&CandidateDomains::within([block_tag("docs")])),
            RowBlock::Declared(block_tag("docs")),
            "one declared block IS the block every row of this producer lies in"
        );
        assert_eq!(
            entailed_block(&CandidateDomains::within([
                block_tag("docs"),
                block_tag("people")
            ])),
            RowBlock::Undeclared,
            "two blocks entail nothing about any one row, and this layer does not choose"
        );
        assert_eq!(
            entailed_block(&CandidateDomains::Unrestricted),
            RowBlock::Undeclared,
            "an unrestricted producer owes no block, so there is none to entail"
        );
    }

    /// The column, when the producer declared one. An IRI is a block; an unbound
    /// cell and a non-IRI are refusals of the whole unit rather than of the row,
    /// because dropping a row renumbers every rank after it. The valid case is
    /// executed beside both refusals.
    #[test]
    fn the_block_column_is_read_as_an_iri_or_the_unit_is_refused() {
        let variables = vec![CANDIDATE_NAME.to_owned(), BLOCK_NAME.to_owned()];
        let declared = CandidateDomains::within([block_tag("docs"), block_tag("people")]);

        // Valid: two rows, each naming its own block out of the declared pair.
        let ranked = rank_candidates(
            &variables,
            &[
                vec![
                    Some(TermValue::iri("http://example.org/doc")),
                    Some(TermValue::iri("http://example.org/domain/docs")),
                ],
                vec![
                    Some(TermValue::iri("http://example.org/person")),
                    Some(TermValue::iri("http://example.org/domain/people")),
                ],
            ],
            &declared,
        )
        .expect("a bound IRI in the block column is a block");
        assert_eq!(
            ranked
                .iter()
                .map(|(_, _, block)| block.clone())
                .collect::<Vec<_>>(),
            vec![
                RowBlock::Declared(block_tag("docs")),
                RowBlock::Declared(block_tag("people")),
            ],
            "each row carries the block it was drawn from, not the declaration's set"
        );

        // Unbound: the producer declared a column and then named no block in it.
        let unbound = rank_candidates(
            &variables,
            &[vec![Some(TermValue::iri("http://example.org/doc")), None]],
            &declared,
        )
        .expect_err("a declared block column that binds nothing backs nothing");
        assert!(
            unbound.contains(BLOCK_NAME) && unbound.contains("row 1"),
            "the refusal names the column and the row: {unbound}"
        );

        // Not an IRI: a block is named by an IRI, and a literal is not one.
        let literal = rank_candidates(
            &variables,
            &[vec![
                Some(TermValue::iri("http://example.org/doc")),
                Some(TermValue::simple_literal("docs")),
            ]],
            &declared,
        )
        .expect_err("a literal names no block of the candidate universe");
        assert!(
            literal.contains(BLOCK_NAME) && literal.contains("IRI"),
            "the refusal says what a block is: {literal}"
        );
    }

    /// A unit with no block column falls back to the entailment, which is how a
    /// single-block producer backs its declaration without any host writing a
    /// column. The neighbouring case — the same rows under a declaration that
    /// entails nothing — reports the absence rather than inventing a block.
    #[test]
    fn a_unit_with_no_block_column_carries_the_entailed_block() {
        let rows = [vec![Some(TermValue::iri("http://example.org/doc"))]];
        let entailed = rank_candidates(
            &variables(),
            &rows,
            &CandidateDomains::within([block_tag("docs")]),
        )
        .expect("a single-block declaration needs no column");
        assert_eq!(
            entailed.first().map(|(_, _, block)| block.clone()),
            Some(RowBlock::Declared(block_tag("docs")))
        );

        let silent = rank_candidates(&variables(), &rows, &CandidateDomains::Unrestricted)
            .expect("an unrestricted producer still answers");
        assert_eq!(
            silent.first().map(|(_, _, block)| block.clone()),
            Some(RowBlock::Undeclared),
            "silence is reported as silence, never filled in"
        );
    }

    // -----------------------------------------------------------------------
    // The sole-witness rule, over hand-built witnesses
    // -----------------------------------------------------------------------

    /// The relation a conforming unit calls. Two spellings, because the rule
    /// counts relations and a test that used one name twice would be counting
    /// nothing.
    const ONE_RELATION: &str = "http://example.org/pf/alpha";
    const ANOTHER_RELATION: &str = "http://example.org/pf/beta";

    fn declared(generation: &str) -> IndexGeneration {
        IndexGeneration::declared(generation)
    }

    fn incomplete(reason: &str) -> ServiceLevel {
        ServiceLevel::Incomplete {
            reason: reason.to_owned(),
        }
    }

    /// A witness built the way the evaluator builds one: by recording
    /// invocations. Nothing here reaches inside the type, so a rule that only
    /// held for a hand-assembled map would not pass.
    fn witness_of(invocations: &[(&str, IndexGeneration, ServiceLevel)]) -> RelationWitness {
        let mut witness = RelationWitness::default();
        for (relation, generation, service) in invocations {
            witness.record(relation, generation.clone(), service.clone());
        }
        witness
    }

    #[test]
    fn a_witness_with_one_relation_and_one_generation_is_that_units_attestation() {
        let witness = witness_of(&[(ONE_RELATION, declared("gen-7"), ServiceLevel::Undeclared)]);
        assert_eq!(
            sole_attestation(&witness),
            Ok(PfAttestation {
                generation: declared("gen-7"),
                service: ServiceLevel::Undeclared,
            }),
            "one relation, one generation, nothing short: the conforming case"
        );

        // And the incompleteness comes through verbatim, because the reason is
        // what tells an operator which index to rebuild.
        let witness = witness_of(&[(ONE_RELATION, declared("gen-7"), incomplete("rebuilding"))]);
        assert_eq!(
            sole_attestation(&witness),
            Ok(PfAttestation {
                generation: declared("gen-7"),
                service: incomplete("rebuilding"),
            })
        );
    }

    /// **The count is not part of the rule.** How many times a relation enters
    /// host code is a function of how the evaluator chunked its driving rows —
    /// a forked `FILTER EXISTS` re-evaluates per chunk — so a rule that read it
    /// would fail on an input size rather than on a defect. Seven invocations
    /// of one index are still one index.
    #[test]
    fn seven_invocations_of_one_relation_are_accepted() {
        let mut witness = RelationWitness::default();
        for _ in 0..7 {
            witness.record(ONE_RELATION, declared("gen-7"), ServiceLevel::Undeclared);
        }
        assert_eq!(
            witness
                .get(ONE_RELATION)
                .expect("the relation attested")
                .invocations,
            7,
            "the fixture really does count seven, or the claim below is vacuous"
        );
        assert_eq!(
            sole_attestation(&witness),
            Ok(PfAttestation {
                generation: declared("gen-7"),
                service: ServiceLevel::Undeclared,
            }),
            "the count is read and deliberately ignored"
        );
    }

    /// And the count is gone by the time an answer's evidence identity is taken:
    /// the SHIPPED encoder — the only canonical encoding of what the indexes
    /// attested — is driven here over what the collapse produced from one
    /// invocation and from seven.
    ///
    /// This is the assertion that makes the ignoring load-bearing rather than
    /// incidental. Had the evidence bytes been derived from the evaluator's
    /// ledger instead, they would carry `invocations`, and these two runs over
    /// one unchanged index would have been handed different `EvidenceId`s purely
    /// because the evaluator chunked more driving rows.
    #[test]
    fn the_evidence_digest_does_not_move_with_the_invocation_count() {
        let bytes_after = |invocations: usize| {
            let mut witness = RelationWitness::default();
            for _ in 0..invocations {
                witness.record(ONE_RELATION, declared("gen-7"), ServiceLevel::Undeclared);
            }
            let attestation = sole_attestation(&witness).expect("the conforming shape");
            let stratum = Iri::parse("http://example.org/stratum/text").expect("a valid IRI");
            crate::fusion_stream::evidence_canonical_bytes(
                &BTreeMap::from([(stratum, attestation)]),
                &BTreeMap::new(),
            )
        };
        assert_eq!(
            bytes_after(1),
            bytes_after(7),
            "seven invocations of one index are the same evidence as one"
        );

        // Not vacuous: the encoder really does move when the ATTESTATION moves.
        let rebuilt = {
            let mut witness = RelationWitness::default();
            witness.record(ONE_RELATION, declared("gen-8"), ServiceLevel::Undeclared);
            let attestation = sole_attestation(&witness).expect("the conforming shape");
            let stratum = Iri::parse("http://example.org/stratum/text").expect("a valid IRI");
            crate::fusion_stream::evidence_canonical_bytes(
                &BTreeMap::from([(stratum, attestation)]),
                &BTreeMap::new(),
            )
        };
        assert_ne!(
            bytes_after(1),
            rebuilt,
            "a rebuilt generation is different evidence"
        );
    }

    #[test]
    fn a_witness_naming_no_relation_is_refused_and_names_the_count() {
        let reason = sole_attestation(&RelationWitness::default())
            .expect_err("a unit that returned rows reached its relation to obtain them");
        assert!(
            reason.contains('0'),
            "the refusal names the count: {reason}"
        );
    }

    #[test]
    fn a_witness_naming_two_relations_is_refused_and_names_the_count() {
        let witness = witness_of(&[
            (ONE_RELATION, declared("gen-7"), ServiceLevel::Undeclared),
            (
                ANOTHER_RELATION,
                declared("gen-7"),
                ServiceLevel::Undeclared,
            ),
        ]);
        let reason = sole_attestation(&witness)
            .expect_err("a compiled unit binds exactly one producer, so its run calls one");
        assert!(
            reason.contains('2'),
            "the refusal names the count: {reason}"
        );
    }

    #[test]
    fn one_relation_attesting_two_generations_is_refused_and_names_the_count() {
        let witness = witness_of(&[
            (ONE_RELATION, declared("gen-7"), ServiceLevel::Undeclared),
            (ONE_RELATION, declared("gen-8"), ServiceLevel::Undeclared),
        ]);
        let reason = sole_attestation(&witness)
            .expect_err("a query that straddled a rebuild did not read one index");
        assert!(
            reason.contains('2'),
            "the refusal names the count: {reason}"
        );
        assert!(
            reason.contains(ONE_RELATION),
            "and the relation whose index moved: {reason}"
        );
    }

    #[test]
    fn one_relation_attesting_two_incompleteness_reasons_is_refused_and_names_the_count() {
        let witness = witness_of(&[
            (ONE_RELATION, declared("gen-7"), incomplete("shard 1")),
            (ONE_RELATION, declared("gen-7"), incomplete("shard 2")),
        ]);
        let reason =
            sole_attestation(&witness).expect_err("one invocation can give at most one reason");
        assert!(
            reason.contains('2'),
            "the refusal names the count: {reason}"
        );
    }

    /// The over-refusal guard on the one condition that is not absolute. A unit
    /// that returned no row has nothing for an attestation to be *about*, so an
    /// empty witness there is silence rather than a contradiction — and the
    /// neighbouring case, the same empty witness after rows were returned, is
    /// still refused.
    #[test]
    fn an_empty_witness_refuses_only_when_the_unit_returned_rows() {
        assert_eq!(
            read_attestation(&RelationWitness::default(), true),
            Ok(PfAttestation::UNDECLARED),
            "a unit that answered with nothing attested nothing, which is not a defect"
        );
        assert!(
            read_attestation(&RelationWitness::default(), false).is_err(),
            "but rows with no relation behind them did not come from this unit's text"
        );
        // And a unit that returned no rows while its relation DID attest is read
        // exactly as any other: the incompleteness is not lost with the rows.
        assert_eq!(
            read_attestation(
                &witness_of(&[(ONE_RELATION, declared("gen-7"), incomplete("rebuilding"))]),
                true,
            ),
            Ok(PfAttestation {
                generation: declared("gen-7"),
                service: incomplete("rebuilding"),
            })
        );
    }

    // -----------------------------------------------------------------------
    // The probe row is a read, never a value
    // -----------------------------------------------------------------------

    /// The attribution a compiled unit over a single-mode producer carries: the bound
    /// was read at the very mode the call is made under.
    ///
    /// Written once here because every case below is about the *numbers*; the mode a
    /// refusal names is varied in `tests/per_mode_row_bound.rs`, over a producer whose
    /// declaration really is a function of it.
    fn invoked_mode() -> BoundMode {
        BoundMode::Invoked {
            mode: BindingPattern::from_code("fb"),
        }
    }

    /// The `depth + 1`-th row decides the ending and is then dropped; every row
    /// that stays keeps the rank it was given, so nothing is renumbered.
    #[test]
    fn the_probe_row_changes_the_ending_and_nothing_else() {
        let rows = |count: u64| {
            (1..=count)
                .map(|rank| {
                    (
                        rank,
                        Term::new(format!("<http://example.org/doc{rank}>")),
                        RowBlock::Undeclared,
                    )
                })
                .collect::<Vec<_>>()
        };
        let stratum = Iri::parse("http://example.org/stratum/alpha").expect("a valid fixture IRI");
        // Every case here is a producer the evaluator bounds, so the read always
        // reaches one row past the depth; the self-bounding shape has its own test
        // below.
        let bounded = |count, depth, declared| {
            bound_to_depth(
                rows(count),
                depth,
                declared,
                invoked_mode(),
                ReadReach::PastDepth,
                &stratum,
            )
        };

        let (kept, ending, status) = bounded(4, 3, Some(100)).expect("below the declaration");
        assert_eq!(
            kept.iter().map(|(rank, ..)| *rank).collect::<Vec<_>>(),
            vec![1, 2, 3],
            "the probe row is dropped and its neighbours keep their ranks"
        );
        assert_eq!(ending, StreamEnding::DepthReached { rank: 3 });
        assert_eq!(status, ProducerStatus::DepthReached { rank: 3 });

        // Exactly at the depth: the probe never arrived, so the read ran out.
        let (kept, ending, status) = bounded(3, 3, Some(100)).expect("the read ran out");
        assert_eq!(kept.len(), 3);
        assert_eq!(ending, StreamEnding::Exhausted);
        assert_eq!(status, ProducerStatus::Exhausted { rows_emitted: 3 });

        // And below it, where the depth was never the binding constraint.
        let (kept, ending, status) = bounded(1, 3, Some(100)).expect("the read ran out");
        assert_eq!(kept.len(), 1);
        assert_eq!(ending, StreamEnding::Exhausted);
        assert_eq!(status, ProducerStatus::Exhausted { rows_emitted: 1 });

        // An undeclared bound promises nothing, so the probe is the ordinary
        // `DepthReached` and never a breach.
        let (kept, ending, status) = bounded(4, 3, None).expect("nothing was declared to breach");
        assert_eq!(kept.len(), 3);
        assert_eq!(ending, StreamEnding::DepthReached { rank: 3 });
        assert_eq!(status, ProducerStatus::DepthReached { rank: 3 });
    }

    /// The probe row landing past the **declaration** is a different fact from
    /// the probe row landing past the depth, and it is refused rather than
    /// truncated into `Exhausted`.
    ///
    /// The honest neighbour is asserted in the same test: a producer that
    /// declared three and really holds three returns three rows into the four-row
    /// bound, the slot comes back empty, and the exhaustion claim stands. The
    /// refusal must catch the liar without costing that producer anything.
    #[test]
    fn a_row_past_the_declaration_is_refused_and_an_honest_one_is_not() {
        let rows = |count: u64| {
            (1..=count)
                .map(|rank| {
                    (
                        rank,
                        Term::new(format!("<http://example.org/doc{rank}>")),
                        RowBlock::Undeclared,
                    )
                })
                .collect::<Vec<_>>()
        };
        let stratum = Iri::parse("http://example.org/stratum/alpha").expect("a valid fixture IRI");

        let breach = bound_to_depth(
            rows(4),
            3,
            Some(3),
            invoked_mode(),
            ReadReach::PastDepth,
            &stratum,
        )
        .expect_err("a fourth row from a producer that declared three is a breach");
        match &breach {
            ExecutionError::RowBoundBreached {
                stratum: named,
                declared,
                pulled,
                mode,
            } => {
                assert_eq!(named.as_str(), stratum.as_str());
                assert_eq!(*declared, 3, "the promise a host has to go and fix");
                assert_eq!(*pulled, 4, "and the evidence that it is false");
                assert_eq!(
                    *mode,
                    invoked_mode(),
                    "and the declaration the promise was read at, which is the one its \
                     author has to go and look at"
                );
            }
            other => panic!("the breach is reported by name, not as {other:?}"),
        }
        assert!(
            breach.to_string().contains("at most 3 rows")
                && breach.to_string().contains("returned 4")
                && breach.to_string().contains("under mode `fb`"),
            "both numbers and the mode they were read at reach a host that only reads \
             the message: {breach}"
        );

        let (kept, ending, status) = bound_to_depth(
            rows(3),
            3,
            Some(3),
            invoked_mode(),
            ReadReach::PastDepth,
            &stratum,
        )
        .expect("a producer that declared three and holds three is not a liar");
        assert_eq!(
            kept.len(),
            3,
            "the honest producer loses no row to the probe"
        );
        assert_eq!(ending, StreamEnding::Exhausted);
        assert_eq!(
            status,
            ProducerStatus::Exhausted { rows_emitted: 3 },
            "and its exhaustion is now verified by the empty slot rather than \
             believed on the strength of the bound"
        );
    }

    /// A producer that bounds itself at its own declaration reports that bound as
    /// the stopper, because at that depth no ending is observable.
    ///
    /// The read was asked for exactly `depth` rows and returned exactly `depth`
    /// rows, so an index holding a thousand and an index holding three are
    /// indistinguishable from here. `Exhausted` would be a completeness claim minted
    /// from the declaration and `DepthReached` would blame a depth that cut nothing,
    /// so neither is written.
    ///
    /// Both neighbours are executed beside it, because this ending must not spread:
    /// the same producer that returned *fewer* rows than it was allowed really did
    /// run out, and the same producer read below its declaration still receives the
    /// probe and still reports the ordinary endings.
    #[test]
    fn a_self_bounding_producer_at_its_declaration_reports_the_bound_that_stopped_it() {
        let rows = |count: u64| {
            (1..=count)
                .map(|rank| {
                    (
                        rank,
                        Term::new(format!("<http://example.org/doc{rank}>")),
                        RowBlock::Undeclared,
                    )
                })
                .collect::<Vec<_>>()
        };
        let stratum = Iri::parse("http://example.org/stratum/alpha").expect("a valid fixture IRI");

        // Asked for three, returned three, and the fourth row could not have been
        // requested: the declaration is the stopper and the ending says so.
        let (kept, ending, status) = bound_to_depth(
            rows(3),
            3,
            Some(3),
            invoked_mode(),
            ReadReach::AtDepth,
            &stratum,
        )
        .expect("a producer at its own declared bound is not a liar");
        assert_eq!(kept.len(), 3, "every row it was allowed reaches the stream");
        assert_eq!(ending, StreamEnding::RowBoundReached { rank: 3 });
        assert_eq!(status, ProducerStatus::RowBoundReached { rank: 3 });

        // Fewer rows than it was allowed: nothing stopped it, so this really is
        // exhaustion and it must not be reported as a bound.
        let (kept, ending, status) = bound_to_depth(
            rows(2),
            3,
            Some(3),
            invoked_mode(),
            ReadReach::AtDepth,
            &stratum,
        )
        .expect("a short answer to a request for three");
        assert_eq!(kept.len(), 2);
        assert_eq!(ending, StreamEnding::Exhausted);
        assert_eq!(status, ProducerStatus::Exhausted { rows_emitted: 2 });

        // And the same producer read below its declaration: the argument carries the
        // probe there, so the ending is observable and stays `DepthReached`.
        let (kept, ending, status) = bound_to_depth(
            rows(4),
            3,
            Some(9),
            invoked_mode(),
            ReadReach::PastDepth,
            &stratum,
        )
        .expect("a depth below the declaration is probed like any other");
        assert_eq!(kept.len(), 3);
        assert_eq!(ending, StreamEnding::DepthReached { rank: 3 });
        assert_eq!(status, ProducerStatus::DepthReached { rank: 3 });

        // A self-bounding producer that beats the argument it was handed is still
        // caught by the unit's own bound rather than reported as any ending.
        let breach = bound_to_depth(
            rows(4),
            3,
            Some(3),
            invoked_mode(),
            ReadReach::AtDepth,
            &stratum,
        )
        .expect_err("a fourth row from a producer asked for three is a breach");
        assert!(
            matches!(
                breach,
                ExecutionError::RowBoundBreached {
                    declared: 3,
                    pulled: 4,
                    ..
                }
            ),
            "got {breach:?}"
        );
    }
}
