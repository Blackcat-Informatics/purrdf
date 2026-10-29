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
use std::fmt;

use purrdf_core::{DatasetView, TermValue};
use purrdf_sparql_eval::{
    CallCursor, CandidateDomains, DomainTag, ExtensionEnv, InternedOutcome, NativeSparqlEngine,
    PfAttestation, PreparedExecution, PropertyFunctionRegistry, QueryGovernors, QueryOptions,
    RegistryId, RelationWitness, ServiceLevel,
};

use crate::admission::BoundMode;
use crate::compile::{
    BLOCK_NAME, CANDIDATE_NAME, CompiledRead, EXCLUSION_LIMIT, ReadReach, ReadSchedule, StratumUnit,
};
use crate::fuse::TopK;
use crate::fusion_stream::ProducerStatus;
use crate::id::PlanId;
use crate::iri::{Iri, Term};
use crate::ranked_stream::{
    ExclusionVerdict, ProducerReceipt, ProtocolError, RowBlock, StreamContract,
};
#[cfg(test)]
use crate::render::{RenderError, candidate_lexical};

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
    pub streams: purrdf_sparql_eval::AdmittedVec<StratumStream<'d>>,
    /// Every stratum's own terminal status.
    pub statuses: purrdf_sparql_eval::AdmittedOrderedMap<Iri, ProducerStatus>,
}

/// One actual native ranked read with an independent prefix depth.
///
/// No fusion weight, TopK or score is attached. contract and attestation are the
/// original producer facts; stream owns the same invocation/receipt/cost state.
#[derive(Debug)]
pub struct ReadStratum<'d> {
    /// The actual producer stratum.
    pub stratum: Iri,
    /// The pinned candidate or fusion plan.
    pub plan_id: PlanId,
    /// The actual independent unit depth, without its read-only probe row.
    pub requested_depth: u32,
    /// The producer's declared duplicate/domain/fidelity/exclusion promises.
    pub contract: StreamContract,
    /// The original generation/service announcement before the first row.
    pub attestation: PfAttestation,
    /// The native read, preserving rank, settlement and producer cost.
    pub stream: RankedStreamImpl<'d>,
    /// Settlement retained in the original read arena if result admission fails.
    /// A successful value retains the actual final attestation, to compare with
    /// the announcement above; a refusal retains the original error unchanged.
    pub admission_failure_settlement: Option<Result<Option<PfAttestation>, ProtocolError>>,
}

/// Unscored candidate streams and the failures of producers that did not open.
#[derive(Debug)]
pub struct CandidateExecutionResult<'d> {
    /// The actual native unit reads, ascending by stratum.
    pub streams: purrdf_sparql_eval::AdmittedVec<ReadStratum<'d>>,
    /// Original terminal statuses known at execute time, including failures.
    pub statuses: purrdf_sparql_eval::AdmittedOrderedMap<Iri, ProducerStatus>,
    /// The explicit candidate-plan identity.
    pub plan_id: PlanId,
    /// The caller work depths compiled into the actual units.
    pub depths: crate::candidate::CandidateDepths,
    /// The admitted contract of every selected stratum, including failed reads.
    pub contracts: purrdf_sparql_eval::SharedWorkspace<
        purrdf_sparql_eval::AdmittedOrderedMap<Iri, StreamContract>,
    >,
    /// Request terms and placements no selected producer could serve.
    pub unserved_terms: purrdf_sparql_eval::SharedWorkspace<
        purrdf_sparql_eval::AdmittedVec<crate::plan::UnservedTerm>,
    >,
    /// Immutable caller-plan binding records retained from compilation; no execution copy.
    pub producer_bindings: purrdf_sparql_eval::SharedWorkspace<Vec<crate::plan::ProducerBinding>>,
    /// Immutable caller-plan decisions retained from compilation; no execution copy.
    pub producer_decisions: purrdf_sparql_eval::SharedWorkspace<Vec<crate::plan::ProducerDecision>>,
}

impl purrdf_hash::error::StructuredError for ReadStratum<'_> {
    fn visit<'error>(&'error self, visitor: &mut dyn purrdf_hash::error::ErrorVisitor<'error>) {
        use purrdf_hash::error::ErrorValue;
        let Self {
            stratum,
            plan_id,
            requested_depth,
            contract,
            attestation,
            stream,
            admission_failure_settlement,
        } = self;
        visitor.variant("retrieval-read-stratum");
        visitor.value(Some("stratum"), ErrorValue::Iri(stratum.as_str()));
        visitor.value(Some("plan_id"), ErrorValue::Record(plan_id));
        visitor.value(
            Some("requested_depth"),
            ErrorValue::Unsigned(u128::from(*requested_depth)),
        );
        visitor.value(Some("contract"), ErrorValue::Record(contract));
        visitor.value(Some("attestation"), ErrorValue::Record(attestation));
        visitor.value(Some("stream"), ErrorValue::Record(stream));
        match admission_failure_settlement {
            None => visitor.value(Some("admission_failure_settlement"), ErrorValue::Absent),
            Some(result) => {
                visitor.begin_record(
                    Some("admission_failure_settlement"),
                    "retrieval-recorded-settlement",
                );
                match result {
                    Ok(attestation) => {
                        visitor.value(Some("kind"), ErrorValue::Code("settled"));
                        visitor.value(
                            Some("attestation"),
                            attestation
                                .as_ref()
                                .map_or(ErrorValue::Absent, |value| ErrorValue::Record(value)),
                        );
                    }
                    Err(error) => {
                        visitor.value(Some("kind"), ErrorValue::Code("refused"));
                        visitor.value(Some("cause"), ErrorValue::Cause(error));
                    }
                }
                visitor.end_record();
            }
        }
    }
}

impl purrdf_hash::error::StructuredError for CandidateExecutionResult<'_> {
    fn visit<'error>(&'error self, visitor: &mut dyn purrdf_hash::error::ErrorVisitor<'error>) {
        use purrdf_hash::error::{ErrorValue, visit_list};
        let Self {
            streams,
            statuses,
            plan_id,
            depths,
            contracts,
            unserved_terms,
            producer_bindings,
            producer_decisions,
        } = self;
        visitor.variant("retrieval-candidate-execution");
        visitor.value(Some("plan_id"), ErrorValue::Record(plan_id));
        visitor.value(Some("depths"), ErrorValue::Record(depths));
        visit_list(
            visitor,
            Some("streams"),
            streams.iter().map(|value| ErrorValue::Record(value)),
        );
        visitor.begin_list(Some("statuses"), statuses.len());
        for (stratum, status) in statuses {
            visitor.begin_record(None, "retrieval-stratum-status");
            visitor.value(Some("stratum"), ErrorValue::Iri(stratum.as_str()));
            visitor.value(Some("status"), ErrorValue::Record(status));
            visitor.end_record();
        }
        visitor.end_list();
        visitor.begin_list(Some("contracts"), contracts.len());
        for (stratum, contract) in contracts.iter() {
            visitor.begin_record(None, "retrieval-stratum-contract");
            visitor.value(Some("stratum"), ErrorValue::Iri(stratum.as_str()));
            visitor.value(Some("contract"), ErrorValue::Record(contract));
            visitor.end_record();
        }
        visitor.end_list();
        visit_list(
            visitor,
            Some("unserved_terms"),
            unserved_terms.iter().map(|value| ErrorValue::Record(value)),
        );
        visit_list(
            visitor,
            Some("producer_bindings"),
            producer_bindings
                .iter()
                .map(|value| ErrorValue::Record(value)),
        );
        visit_list(
            visitor,
            Some("producer_decisions"),
            producer_decisions
                .iter()
                .map(|value| ErrorValue::Record(value)),
        );
    }
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
#[derive(Debug)]
#[non_exhaustive]
pub enum ExecutionError {
    /// The original native refusal, preserving its cause and payload admission.
    NativeFailure(purrdf_sparql_eval::EvalError),
    /// Hard constraints require their original admitted candidate evaluator.
    /// A normal unscoped entry point cannot silently drop the compiled scope.
    MissingCandidateScope,
    /// The backing read session failed; no stratum may publish partial source data.
    SourceRead {
        /// The operational refusal, retained for the caller.
        reason: String,
    },
    /// The compiled units were built against a different live registry instance.
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
    EnvironmentNotDerivable {
        /// The registry's own failure, propagated unchanged.
        reason: String,
    },
}

impl purrdf_hash::error::StructuredError for ExecutionError {
    purrdf_hash::error_visit! {
        NativeFailure(cause) => "retrieval-execution-native-failure" { "cause" => Cause(cause) },
        MissingCandidateScope => "retrieval-execution-missing-candidate-scope" {},
        SourceRead { reason } => "retrieval-execution-source-read" { "reason" => Text(reason) },
        RegistryMismatch { expected, got } => "retrieval-execution-registry-mismatch" { "expected" => Record(expected), "got" => Record(got) },
        InconsistentWitness { stratum, reason } => "retrieval-execution-inconsistent-witness" { "stratum" => Iri(stratum.as_str()), "reason" => Text(reason) },
        RowBoundBreached { stratum, declared, pulled, mode } => "retrieval-execution-row-bound-breached" { "stratum" => Iri(stratum.as_str()), "declared" => Unsigned(u128::from(*declared)), "pulled" => Unsigned(u128::from(*pulled)), "mode" => Record(mode) },
        UnitsNotAsAssembled { plan, reason } => "retrieval-execution-units-not-as-assembled" { "plan" => Record(plan), "reason" => Text(reason) },
        EnvironmentNotDerivable { reason } => "retrieval-execution-environment-not-derivable" { "reason" => Text(reason) },
    }
}

impl fmt::Display for ExecutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NativeFailure(error) => error.fmt(f),
            Self::MissingCandidateScope => {
                f.write_str("scoped retrieval requires its original admitted candidate evaluator")
            }
            Self::SourceRead { reason } => write!(f, "retrieval source read failed: {reason}"),
            Self::RegistryMismatch { expected, got } => write!(
                f,
                "compiled units name registry instance {expected:?}, but execution holds {got:?}"
            ),
            Self::InconsistentWitness { stratum, reason } => write!(
                f,
                "stratum {stratum}: the relation witness is not a compiled unit's: {reason}"
            ),
            Self::RowBoundBreached {
                stratum,
                declared,
                pulled,
                mode,
            } => write!(
                f,
                "stratum {stratum}: the registry declares at most {declared} rows per invocation{mode}, and the read returned {pulled}"
            ),
            Self::UnitsNotAsAssembled { plan, reason } => write!(
                f,
                "the compiled bundle for plan {plan} is not the bundle it was assembled as: {reason}"
            ),
            Self::EnvironmentNotDerivable { reason } => write!(
                f,
                "the execution environment could not be derived from the registry: {reason}"
            ),
        }
    }
}

impl std::error::Error for ExecutionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::NativeFailure(error) => Some(error),
            _ => None,
        }
    }
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
    workspace: Option<purrdf_sparql_eval::WorkspaceCapability>,
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
    exclusion: Option<purrdf_sparql_eval::WorkspaceBox<dyn ExclusionLookup + 'd>>,
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
    native_failure: Option<purrdf_sparql_eval::EvalError>,
}

/// Where a [`RankedStreamImpl`]'s rows come from.
enum RowSource<'d> {
    /// A read that finished before this stream existed: its rows, already cut to the
    /// depth, and the ending it had.
    Materialised {
        rows: RankedRowsStorage,
        ending: StreamEnding,
        /// What the read's witness attested when it finished, for a read this
        /// executor took; `None` for rows a caller handed [`RankedStreamImpl::new`],
        /// which came from no read this crate observed.
        attested: Option<PfAttestation>,
    },
    /// One invocation held open and read a row per pull, and — once it has ended —
    /// how it ended.
    OnDemand {
        read: purrdf_sparql_eval::WorkspaceBox<dyn OnDemandRead + 'd>,
        ended: Option<OnDemandEnd>,
    },
}

enum RankedRowsStorage {
    Caller(std::vec::IntoIter<(u64, Term, RowBlock)>),
    Native(purrdf_sparql_eval::AdmittedVecIntoIter<(u64, Term, RowBlock)>),
}

impl purrdf_hash::error::StructuredError for RankedRowsStorage {
    fn visit<'error>(&'error self, visitor: &mut dyn purrdf_hash::error::ErrorVisitor<'error>) {
        use purrdf_hash::error::ErrorValue;
        let rows = match self {
            Self::Caller(rows) => rows.as_slice(),
            Self::Native(rows) => rows.as_slice(),
        };
        visitor.variant("retrieval-unconsumed-rows");
        visitor.begin_list(Some("rows"), rows.len());
        for (rank, term, block) in rows {
            visitor.begin_record(None, "retrieval-ranked-row");
            visitor.value(Some("rank"), ErrorValue::Unsigned(u128::from(*rank)));
            visitor.value(Some("term"), ErrorValue::Term(term));
            visitor.value(Some("block"), ErrorValue::Record(block));
            visitor.end_record();
        }
        visitor.end_list();
    }
}

impl purrdf_hash::error::StructuredError for StreamEnding {
    purrdf_hash::error_visit! {
        Exhausted => "retrieval-stream-exhausted" {},
        DepthReached { rank } => "retrieval-stream-depth-reached" { "rank" => Unsigned(u128::from(*rank)) },
        RowBoundReached { rank } => "retrieval-stream-row-bound-reached" { "rank" => Unsigned(u128::from(*rank)) },
        SuppliedQueryEnded { rank } => "retrieval-stream-supplied-query-ended" { "rank" => Unsigned(u128::from(*rank)) },
    }
}

impl purrdf_hash::error::StructuredError for OnDemandEnd {
    purrdf_hash::error_visit! {
        Ended(ending) => "retrieval-demand-ended" { "ending" => Record(ending) },
        Failed(reason) => "retrieval-demand-failed" { "reason" => Text(reason.as_ref()) },
    }
}

impl purrdf_hash::error::StructuredError for RowSource<'_> {
    fn visit<'error>(&'error self, visitor: &mut dyn purrdf_hash::error::ErrorVisitor<'error>) {
        use purrdf_hash::error::ErrorValue;
        match self {
            Self::Materialised {
                rows,
                ending,
                attested,
            } => {
                visitor.variant("retrieval-source-materialised");
                visitor.value(Some("rows"), ErrorValue::Record(rows));
                visitor.value(Some("ending"), ErrorValue::Record(ending));
                visitor.value(
                    Some("attested"),
                    attested
                        .as_ref()
                        .map_or(ErrorValue::Absent, |value| ErrorValue::Record(value)),
                );
            }
            Self::OnDemand { read: _, ended } => {
                // The live read is an execution capability. Never call even a
                // nominally read-only provider method while inspecting a failure.
                visitor.variant("retrieval-source-on-demand");
                visitor.value(
                    Some("ended"),
                    ended
                        .as_ref()
                        .map_or(ErrorValue::Absent, |value| ErrorValue::Record(value)),
                );
            }
        }
    }
}

impl purrdf_hash::error::StructuredError for RankedStreamImpl<'_> {
    purrdf_hash::error_visit! {
        @record self { workspace, source, pulled, exhausted, exclusion, materialised,
            run_invalidated, native_failure } => "retrieval-ranked-stream" {
            "source" => Record(&self.source),
            "pulled" => Unsigned(u128::from(self.pulled)),
            "exhausted" => Boolean(self.exhausted),
            "has_exclusion" => Boolean(self.exclusion.is_some()),
            "materialised_counter" => Unsigned(u128::from(self.materialised)),
            "run_invalidated" => Boolean(self.run_invalidated),
            "native_failure" => Value(self.native_failure.as_ref().map_or(purrdf_hash::error::ErrorValue::Absent, |value| purrdf_hash::error::ErrorValue::Cause(value)))
        }
    }
}

impl Iterator for RankedRowsStorage {
    type Item = (u64, Term, RowBlock);
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Caller(rows) => rows.next(),
            Self::Native(rows) => rows.next(),
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            Self::Caller(rows) => rows.size_hint(),
            Self::Native(rows) => rows.size_hint(),
        }
    }
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
                .field("rows_left", &rows.size_hint().0)
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
    Failed(purrdf_lex::allocation::OwnedText),
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
    fn settle(&mut self) -> Result<PfAttestation, ReadFault>;
}

/// One step of an on-demand read.
enum ReadStep {
    /// The next row, ranked.
    Row((u64, Term, RowBlock)),
    /// The read has ended, this way.
    Ended(StreamEnding),
}

/// Why an on-demand read could not go on.
enum ReadFault {
    /// Original native cause; never rendered into unadmitted replacement text.
    Native(purrdf_sparql_eval::EvalError),
    /// An original invocation refusal, isolated from native storage authority.
    Invocation(purrdf_sparql_eval::EvalError),
    /// The refusal, rendered as a materialised read would render it.
    Producer {
        reason: String,
        /// Whether the failure invalidates the whole run whenever it is observed:
        /// a native source/storage refusal, a failed scoped declaration/publication,
        /// or a producer beating its declared row bound is not an isolated stratum.
        whole_run: bool,
    },
}

impl ReadFault {
    fn invocation(error: purrdf_sparql_eval::EvalError, scoped: bool) -> Self {
        if scoped || error.is_native_storage_failure() {
            Self::Native(error)
        } else {
            Self::Invocation(error)
        }
    }
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
    Value(purrdf_sparql_eval::SharedWorkspace<purrdf_sparql_eval::WorkspaceTerm>),
}

/// Every candidate the ranking reads of one [`execute_within`] call named, keyed by
/// the [`Term`] the fusion stage will ask about. Shared by that call's lookups.
type CandidateIndex<I> = purrdf_sparql_eval::AdmittedOrderedMap<Term, CandidateBinding<I>>;

/// One stratum's prepared lookups, held until every ranking read of the call is
/// open and they can be attached to its stream.
struct PendingLookup<'d, I: purrdf_core::ViewTermId> {
    /// The stream's position in the call's streams.
    position: usize,
    /// The stratum the lookups answer for.
    stratum: Iri,
    /// The lookups, one list per alternative — see [`DatasetExclusion::lookups`].
    lookups: purrdf_sparql_eval::AdmittedVec<purrdf_sparql_eval::AdmittedVec<PreparedLookup>>,
    /// What the stratum's ranked read pinned, which every lookup is held to.
    pinned: PfAttestation,
    /// Original caller scope and source retained for subsequent point reads.
    scoped: Option<ScopedLookup<'d, I>>,
}

struct ScopedLookup<'d, I: purrdf_core::ViewTermId> {
    scope: purrdf_sparql_eval::NativeCandidateScope<'d>,
    source: &'d dyn purrdf_sparql_eval::EvidenceSource<I>,
    workspace: purrdf_sparql_eval::WorkspaceCapability,
    rows: crate::ScopedRows,
}

/// One exclusion lookup, prepared once with the candidate as its one parameter.
struct PreparedLookup {
    /// The prepared execution.
    execution: PreparedExecution,
    /// The candidate parameter's slot, resolved once.
    slot: usize,
    /// Whether its call is driven by patterns of the caller's text, and so invoked
    /// once per distinct binding of its inputs rather than once — see
    /// [`LookupText::driven`](crate::compile::LookupText).
    driven: bool,
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
/// `?candidate` column from several calls, grouped into **alternatives**
/// ([`StratumUnit::exclusion_sparql`](crate::StratumUnit)): every solution naming a
/// candidate is one in which every call of some alternative emitted it. Within an
/// alternative — a join of calls on the column — any one call finding no row proves
/// the alternative never names the candidate; across alternatives — the branches of
/// a `UNION` — the candidate is out of the text's reach only when every alternative
/// is. So the verdict is `Excluded` exactly when each alternative, asked in order,
/// has a lookup that finds no row, its lookups asked in order and the first such
/// ending that alternative; the first alternative all of whose lookups find the
/// candidate answers `Possible`. Each lookup is held, before its rows are read at all,
/// to the one attestation the stratum's ranked read pinned — the sole-witness rule
/// makes that one attestation cover every call the text made, in every branch, since
/// every call of a conforming unit invokes the one relation at one generation — so a
/// verdict from any lookup answered by another index generation is refused, whichever
/// alternative and lookup it was.
struct DatasetExclusion<'d, D: DatasetView + Sync> {
    operation: purrdf_sparql_eval::ReadOperation<D::ReadError>,
    scoped: Option<ScopedLookup<'d, D::Id>>,
    /// The stratum this lookup answers for, so its refusals name a producer
    /// rather than a query.
    stratum: Iri,
    /// The engine the execution was prepared on, shared by every lookup the same
    /// [`execute_within`] call compiled. Shared rather than owned per stratum
    /// because the stream outlives that call, and an engine is not a fact about
    /// any one stratum.
    engine: purrdf_sparql_eval::SharedWorkspace<NativeSparqlEngine>,
    /// The extension environment the execution was prepared against, shared with
    /// the run that produced the rows. An execution prepared against one registry
    /// and run under another is refused by the evaluator, so the same value must
    /// reach both calls — see [`execute_within`]'s own single-environment note.
    env: purrdf_sparql_eval::SharedWorkspace<ExtensionEnv<'static>>,
    /// The lookups themselves, one list per alternative and one lookup per
    /// qualifying call of it, in the order
    /// [`StratumUnit::exclusion_sparql`](crate::StratumUnit) derived them, each
    /// prepared once with the candidate as its one parameter.
    lookups: purrdf_sparql_eval::AdmittedVec<purrdf_sparql_eval::AdmittedVec<PreparedLookup>>,
    /// How each candidate this execution read is bound, shared by its lookups and
    /// written by the reads that name candidates — on demand, as each row is pulled.
    candidates: purrdf_sparql_eval::SharedWorkspace<RefCell<CandidateIndex<D::Id>>>,
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
        for alternative in 0..self.lookups.len() {
            let mut excluded = false;
            for index in 0..self.lookups[alternative].len() {
                if self.ask(alternative, index, candidate, binding.as_ref())?
                    == ExclusionVerdict::Excluded
                {
                    excluded = true;
                    break;
                }
            }
            if !excluded {
                return Ok(ExclusionVerdict::Possible);
            }
        }
        Ok(ExclusionVerdict::Excluded)
    }
}

impl<D: DatasetView + Sync> DatasetExclusion<'_, D> {
    /// Ask the `index`-th lookup of the `alternative`-th alternative whether
    /// `candidate`, bound as `binding` says, is out of its call's reach.
    fn ask(
        &mut self,
        alternative: usize,
        index: usize,
        candidate: &Term,
        binding: Option<&CandidateBinding<D::Id>>,
    ) -> Result<ExclusionVerdict, ProtocolError> {
        let lookup_workspace = self.candidates.borrow().workspace().clone();
        let external = if binding.is_none() {
            let value = crate::render::decode_term_admitted(candidate.as_str(), &lookup_workspace)
                .map_err(ProtocolError::NativeFailure)?;
            Some(CandidateBinding::Value(
                purrdf_sparql_eval::SharedWorkspace::new_admitted(value, &lookup_workspace)
                    .map_err(ProtocolError::NativeFailure)?,
            ))
        } else {
            None
        };
        let binding = binding.or(external.as_ref());
        let stratum = &self.stratum;
        let failed = |reason: String| ProtocolError::ExclusionLookupFailed {
            stratum: stratum.as_str().to_owned(),
            reason,
        };
        let PreparedLookup {
            execution,
            slot,
            driven,
        } = &mut self.lookups[alternative][index];
        let (slot, driven) = (*slot, *driven);
        match binding {
            Some(CandidateBinding::Id(id)) => {
                execution
                    .bind_read_id_admitted(slot, self.dataset, *id, &self.operation)
                    .map_err(ProtocolError::NativeFailure)?;
            }
            Some(CandidateBinding::Value(value)) => {
                execution
                    .bind_admitted(slot, value.clone(), &lookup_workspace)
                    .map_err(ProtocolError::NativeFailure)?;
            }
            // A candidate no ranking read of this execution named: one from a
            // stream assembled elsewhere and fused beside these. Its canonical
            // lexical is the only form it arrives in, so it is read back through the
            // crate's own decoder. A term this layer wrote and cannot read back is a
            // defect in one of the two halves, and it is reported as the failed
            // lookup it is rather than answered as `Possible`.
            None => unreachable!("external canonical candidate has its retained native binding"),
        }
        let mut options = QueryOptions::new().with_env(&self.env);
        if let Some(scoped) = &self.scoped {
            options = options.with_candidate_scope(&scoped.scope);
        }
        // Both doors run the same prepared lookup and visitor. Scoped lookup
        // evidence is retained while its original context and source are alive.
        let visit =
            |outcome: InternedOutcome<'_, '_, D>| -> Result<_, purrdf_sparql_eval::EvalError> {
                let InternedOutcome::Solutions(solutions) = outcome else {
                    return Ok(None);
                };
                let rows = solutions.len();
                let pulled = if driven {
                    let mut answered = purrdf_sparql_eval::AdmittedMap::default();
                    for row in solutions.rows() {
                        if let Some(count) = answered.get_mut(row.as_slice()) {
                            *count += 1;
                        } else {
                            answered.insert_admitted(row.as_slice(), 1_u64, &lookup_workspace)?;
                        }
                    }
                    answered.values().copied().max().unwrap_or(0)
                } else {
                    u64::try_from(rows).unwrap_or(u64::MAX)
                };
                let evidence = self
                    .scoped
                    .as_ref()
                    .map(|_| solutions.existential_evidence())
                    .transpose()?;
                Ok(Some((rows, pulled, evidence)))
            };
        let (rows, witness) = self
            .engine
            .execute_read_witnessed_admitted(
                execution,
                self.dataset,
                options,
                self.scoped.as_ref().map(|scoped| scoped.source),
                &self.operation,
                visit,
            )
            .map_err(|error| match error {
                purrdf_sparql_eval::EvaluationFailure::Evaluation(error) => {
                    ProtocolError::NativeFailure(error)
                }
                error => failed(error.to_string()),
            })?;
        let (rows, pulled, evidence) = rows
            .map_err(ProtocolError::NativeFailure)?
            .ok_or_else(|| failed("the exclusion lookup did not return solutions".to_owned()))?;
        let closed = if let (Some(scoped), Some(evidence)) = (&self.scoped, evidence) {
            let closed = !evidence.is_undecided()
                && evidence
                    .with_record(|record| {
                        record.completeness() == purrdf_core::evidence::SearchCompleteness::Complete
                    })
                    .map_err(ProtocolError::NativeFailure)?;
            let value = match binding {
                Some(CandidateBinding::Id(id)) => purrdf_sparql_eval::ScratchInterner::default()
                    .try_owned_value_of(
                        self.dataset,
                        purrdf_sparql_eval::SolutionTerm::Existing(*id),
                        &scoped.workspace,
                        |error| self.operation.source_error(error),
                    ),
                Some(CandidateBinding::Value(value)) => scoped.workspace.clone_term(value),
                None => unreachable!("external canonical candidate retains its native value"),
            }
            .map_err(ProtocolError::NativeFailure)?;
            let mut cells = purrdf_sparql_eval::AdmittedVec::with_capacity(1, &scoped.workspace)
                .map_err(ProtocolError::NativeFailure)?;
            cells
                .push(Some(value))
                .map_err(ProtocolError::NativeFailure)?;
            scoped
                .rows
                .push(crate::ScopedRow { cells, evidence })
                .map_err(ProtocolError::NativeFailure)?;
            closed
        } else {
            true
        };
        // Held to the read before its verdict is read at all. A verdict is only a
        // statement about the rows the ranked read would have gone on to name if it
        // came from the index generation that read pinned; one from anywhere else
        // is refused whatever it says, because a verdict that happens to agree has
        // still certified rows on the word of an index the answer's evidence does
        // not name. The lookup is one call with constant arguments, so its witness
        // is read under the rule a compiled unit's is — one relation, one
        // generation — and a witness that breaks it is the same refusal.
        // A driven lookup whose driving pattern binds no input at all invoked its call
        // not once, so nothing attested — and nothing needed to. The driving pattern
        // is implied by the text, so the text binds no input either and never
        // invokes the call: no candidate is one it names. That is a verdict about
        // the dataset the ranking read, which this lookup read too, and about no
        // index generation. An undriven lookup always invokes its call, so an empty
        // witness there stays the refusal below; so does an empty witness beside a
        // row, which no invocation produced.
        if driven && rows == 0 && witness.is_empty() {
            return Ok(if closed {
                ExclusionVerdict::Excluded
            } else {
                ExclusionVerdict::Possible
            });
        }
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
        // The point bound a declared exclusion basis rests on, derived from the
        // ceiling the lookup is read under rather than written twice: the text
        // asks for one row past the bound precisely so a producer that beats it
        // is observed instead of truncated into looking conforming. A driven
        // lookup is read whole instead, and `pulled` is then the most rows any one
        // of its invocations answered — each distinct binding of its inputs is one
        // invocation, so a row repeated is an invocation answering twice.
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
        Ok(if rows == 0 && closed {
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
        /// The last emitted positive rank, independent of the planned row depth.
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
        /// The last emitted positive rank, independent of the declared row bound.
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
        /// The last emitted positive rank, independent of row count.
        /// Zero for a read that returned nothing — which is not a claim that
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
            workspace: None,
            source: RowSource::Materialised {
                rows: RankedRowsStorage::Caller(rows.into_iter()),
                ending,
                attested: None,
            },
            pulled: 0,
            exhausted: false,
            exclusion: None,
            materialised,
            run_invalidated: false,
            native_failure: None,
        }
    }

    fn from_admitted(
        rows: purrdf_sparql_eval::AdmittedVec<(u64, Term, RowBlock)>,
        ending: StreamEnding,
        workspace: &purrdf_sparql_eval::WorkspaceCapability,
    ) -> Self {
        let materialised = u64::try_from(rows.len()).unwrap_or(u64::MAX);
        Self {
            workspace: Some(workspace.clone()),
            source: RowSource::Materialised {
                rows: RankedRowsStorage::Native(rows.into_iter()),
                ending,
                attested: None,
            },
            pulled: 0,
            exhausted: false,
            exclusion: None,
            materialised,
            run_invalidated: false,
            native_failure: None,
        }
    }

    /// A stream over one invocation held open, producing a row per pull.
    ///
    /// Not public: an on-demand read is one this executor opened, against a dataset
    /// and a registry it was handed, and its ending is decided by observations only
    /// the executor's read makes.
    fn on_demand(
        read: purrdf_sparql_eval::WorkspaceBox<dyn OnDemandRead + 'd>,
        workspace: &purrdf_sparql_eval::WorkspaceCapability,
    ) -> Self {
        Self {
            workspace: Some(workspace.clone()),
            source: RowSource::OnDemand { read, ended: None },
            pulled: 0,
            exhausted: false,
            exclusion: None,
            materialised: 0,
            run_invalidated: false,
            native_failure: None,
        }
    }

    /// Borrow the original native authority without detaching row ownership.
    pub fn workspace(&self) -> Option<&purrdf_sparql_eval::WorkspaceCapability> {
        self.workspace.as_ref()
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
        self.check_native_failure()?;
        match &mut self.source {
            RowSource::Materialised { attested, .. } => Ok(attested.clone()),
            RowSource::OnDemand { read, .. } => match read.settle() {
                Ok(attestation) => Ok(Some(attestation)),
                Err(ReadFault::Native(error) | ReadFault::Invocation(error)) => {
                    self.run_invalidated = true;
                    self.native_failure = Some(error.clone());
                    Err(ProtocolError::NativeFailure(error))
                }
                Err(ReadFault::Producer { reason, .. }) => Err(ProtocolError::AttestationMoved {
                    stratum: read.stratum().to_owned(),
                    reason,
                }),
            },
        }
    }

    fn check_native_failure(&self) -> Result<(), ProtocolError> {
        self.native_failure.as_ref().map_or(Ok(()), |error| {
            Err(ProtocolError::NativeFailure(error.clone()))
        })
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
    pub(crate) fn attach_exclusion(
        &mut self,
        lookup: purrdf_sparql_eval::WorkspaceBox<dyn ExclusionLookup + 'd>,
    ) {
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
        self.check_native_failure()?;
        let result = match self.exclusion.as_mut() {
            Some(lookup) => lookup.look_up(candidate),
            None => Err(ProtocolError::ExclusionUnavailable),
        };
        if let Err(ProtocolError::NativeFailure(error)) = &result {
            self.run_invalidated = true;
            self.native_failure = Some(error.clone());
        }
        result
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
        self.check_native_failure()?;
        let row = match &mut self.source {
            RowSource::Materialised { rows, .. } => rows.next(),
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
                        Err(ReadFault::Producer {
                            reason,
                            whole_run: false,
                        }) if self.pulled == 0 => {
                            let workspace =
                                self.workspace.as_ref().expect("on-demand native authority");
                            *ended = Some(OnDemandEnd::Failed(
                                workspace
                                    .authored_text(&reason)
                                    .map_err(ProtocolError::NativeFailure)?
                                    .into(),
                            ));
                            None
                        }
                        // After rows were handed out — or at any point, for a
                        // breach of the declared row bound — it fails the request by
                        // name, because those rows may already be in the answer.
                        Err(ReadFault::Invocation(error)) if self.pulled == 0 => {
                            let workspace =
                                self.workspace.as_ref().expect("on-demand native authority");
                            *ended = Some(OnDemandEnd::Failed(
                                workspace
                                    .authored_text(&error)
                                    .map_err(ProtocolError::NativeFailure)?
                                    .into(),
                            ));
                            None
                        }
                        Err(ReadFault::Invocation(error)) => {
                            self.run_invalidated = false;
                            return Err(ProtocolError::ReadFailed {
                                stratum: read.stratum().to_owned(),
                                rows_before: self.pulled,
                                reason: error.to_string(),
                            });
                        }
                        Err(ReadFault::Native(error)) => {
                            self.run_invalidated = true;
                            self.native_failure = Some(error.clone());
                            return Err(ProtocolError::NativeFailure(error));
                        }
                        Err(ReadFault::Producer { reason, whole_run }) => {
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
    /// its producer's own declaration stopped it at. Fusion measures row counts
    /// and last ranks separately and refuses a disagreement
    /// ([`ProtocolError::ForgedReceipt`]). Probe rows do not change the last rank emitted.
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
        self.check_native_failure()?;
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
pub async fn execute<
    'd,
    D: DatasetView<ReadError = std::convert::Infallible> + Sync,
    C: CompiledRead,
>(
    compiled: &C,
    registry: &PropertyFunctionRegistry<'static>,
    dataset: &'d D,
) -> Result<C::Output<'d>, ExecutionError> {
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
pub async fn execute_within<
    'd,
    D: DatasetView<ReadError = std::convert::Infallible> + Sync,
    C: CompiledRead,
>(
    compiled: &C,
    registry: &PropertyFunctionRegistry<'static>,
    dataset: &'d D,
    schedule: ReadSchedule,
) -> Result<C::Output<'d>, ExecutionError> {
    let operation = resident_operation(dataset)?;
    execute_read_within(compiled, registry, dataset, schedule, &operation)
}

pub(crate) fn resident_operation<D: DatasetView<ReadError = std::convert::Infallible>>(
    dataset: &D,
) -> Result<purrdf_sparql_eval::ReadOperation<std::convert::Infallible>, ExecutionError> {
    NativeSparqlEngine::workspace_for_view_admitted(dataset)
        .map(|workspace| purrdf_sparql_eval::ReadOperation::from_capability(&workspace))
        .map_err(|error| match error {
            purrdf_sparql_eval::EvaluationFailure::Evaluation(error) => {
                ExecutionError::NativeFailure(error)
            }
            error => ExecutionError::SourceRead {
                reason: error.to_string(),
            },
        })
}

pub(crate) fn execute_read_within<'d, D: DatasetView + Sync, C: CompiledRead>(
    compiled: &C,
    registry: &PropertyFunctionRegistry<'static>,
    dataset: &'d D,
    schedule: ReadSchedule,
    operation: &purrdf_sparql_eval::ReadOperation<D::ReadError>,
) -> Result<C::Output<'d>, ExecutionError> {
    execute_bound(compiled, registry, dataset, schedule, None, operation)
}

/// Execute the complete compiled hard-constraint set through its original
/// candidate evaluators and evidence source. Unknown rows stay outside ranked
/// fusion and remain available in the returned original unit receipts.
///
/// # Errors
/// Preserves binding/source failures and the ordinary executor's whole-run gates.
#[allow(clippy::unused_async, clippy::future_not_send)]
pub async fn execute_scoped_within<
    'd,
    D: DatasetView<ReadError = std::convert::Infallible> + Sync,
    C: CompiledRead,
>(
    compiled: &C,
    registry: &PropertyFunctionRegistry<'static>,
    dataset: &'d D,
    schedule: ReadSchedule,
    bindings: &'d crate::ConstraintBindings<'d>,
    source: &'d dyn purrdf_sparql_eval::EvidenceSource<D::Id>,
) -> Result<crate::ScopedExecution<C::Output<'d>>, ExecutionError> {
    let operation = resident_operation(dataset)?;
    execute_scoped_in_read(
        compiled, registry, dataset, schedule, bindings, source, &operation,
    )
}

/// Execute scoped units under the original enclosing read operation.
/// Consume and settle returned streams inside
/// [`purrdf_sparql_eval::with_fallible_read`] before publishing their answer.
/// Deferred exclusion lookups share that operation's original source-error owner.
///
/// # Errors
/// Preserves native execution failures. Original operational causes remain in the
/// enclosing operation and are returned at its final read checkpoint.
pub fn execute_scoped_in_read<'d, D: DatasetView + Sync, C: CompiledRead>(
    compiled: &C,
    registry: &PropertyFunctionRegistry<'static>,
    dataset: &'d D,
    schedule: ReadSchedule,
    bindings: &'d crate::ConstraintBindings<'d>,
    source: &'d dyn purrdf_sparql_eval::EvidenceSource<D::Id>,
    operation: &purrdf_sparql_eval::ReadOperation<D::ReadError>,
) -> Result<crate::ScopedExecution<C::Output<'d>>, ExecutionError> {
    let workspace = operation.capability();
    let mut scopes =
        purrdf_sparql_eval::AdmittedVec::with_capacity(compiled.units().len(), &workspace)
            .map_err(ExecutionError::NativeFailure)?;
    let result = execute_bound(
        compiled,
        registry,
        dataset,
        schedule,
        Some(ScopedExecutionInput {
            bindings,
            source,
            receipts: &mut scopes,
        }),
        operation,
    )?;
    Ok(crate::ScopedExecution {
        result,
        scopes,
        fused: None,
    })
}

struct ScopedExecutionInput<'a, 'b, I: purrdf_core::ViewTermId> {
    bindings: &'b crate::ConstraintBindings<'b>,
    source: &'b dyn purrdf_sparql_eval::EvidenceSource<I>,
    receipts: &'a mut purrdf_sparql_eval::AdmittedVec<crate::ScopedStratum>,
}

fn execute_bound<'d, D: DatasetView + Sync, C: CompiledRead>(
    compiled: &C,
    registry: &PropertyFunctionRegistry<'static>,
    dataset: &'d D,
    schedule: ReadSchedule,
    mut scoped: Option<ScopedExecutionInput<'_, 'd, D::Id>>,
    operation: &purrdf_sparql_eval::ReadOperation<D::ReadError>,
) -> Result<C::Output<'d>, ExecutionError> {
    if !compiled.constraints().is_empty() && scoped.is_none() {
        return Err(ExecutionError::MissingCandidateScope);
    }
    if compiled.registry_id() != registry.instance_id() {
        return Err(ExecutionError::RegistryMismatch {
            expected: compiled.registry_id(),
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
    let execution_workspace = operation.capability();
    let engine = purrdf_sparql_eval::SharedWorkspace::new_admitted(
        NativeSparqlEngine::new(),
        &execution_workspace,
    )
    .map_err(ExecutionError::NativeFailure)?;
    let mut streams = purrdf_sparql_eval::AdmittedVec::with_capacity(
        compiled.units().len(),
        &execution_workspace,
    )
    .map_err(ExecutionError::NativeFailure)?;
    // The lookups this call compiled, attached once every ranking read is open —
    // see `CandidateIndex` — beside the index of each candidate those reads named.
    // The index is filled only when some stratum compiled a lookup, because it is
    // read by lookups and by nothing else; a read produced on demand fills it as
    // each row is pulled, which is before any lookup can ask about that row's
    // candidate, because the fusion asks only about candidates it has pulled.
    let mut lookups: purrdf_sparql_eval::AdmittedVec<PendingLookup<'d, D::Id>> =
        purrdf_sparql_eval::AdmittedVec::new(&execution_workspace);
    let candidates: purrdf_sparql_eval::SharedWorkspace<RefCell<CandidateIndex<D::Id>>> =
        purrdf_sparql_eval::SharedWorkspace::new_admitted(
            RefCell::new(CandidateIndex::new(&execution_workspace)),
            &execution_workspace,
        )
        .map_err(ExecutionError::NativeFailure)?;
    let index_candidates = compiled.units().iter().any(StratumUnit::declares_exclusion);
    let mut statuses = purrdf_sparql_eval::AdmittedOrderedMap::new(&execution_workspace);
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
    let env = purrdf_sparql_eval::SharedWorkspace::new_admitted(
        ExtensionEnv::over_relations(registry.clone()).map_err(|e| {
            ExecutionError::EnvironmentNotDerivable {
                reason: e.to_string(),
            }
        })?,
        &execution_workspace,
    )
    .map_err(ExecutionError::NativeFailure)?;
    let options = || QueryOptions::new().with_env(env.as_ref());

    for unit in compiled.units() {
        let scope_workspace = execution_workspace.clone();
        let mut candidate_scope = scoped
            .as_ref()
            .map(|input| {
                input
                    .bindings
                    .bind(compiled, &scope_workspace)
                    .map_err(ExecutionError::NativeFailure)
            })
            .transpose()?;
        let exclusion_rows = scoped
            .as_ref()
            .map(|_| {
                crate::ScopedRows::new(
                    purrdf_sparql_eval::AdmittedVec::new(&scope_workspace),
                    false,
                    &scope_workspace,
                )
                .map_err(ExecutionError::NativeFailure)
            })
            .transpose()?;
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
        let sparql = if scoped.is_some() {
            unit.sparql_scoped()
        } else {
            unit.sparql()
        };
        let prepared =
            match engine.prepare_query_admitted(&sparql, None, options(), &scope_workspace) {
                Ok(prepared) => prepared,
                Err(purrdf_sparql_eval::EvaluationFailure::Evaluation(error))
                    if scoped.is_some() || error.is_native_storage_failure() =>
                {
                    return Err(ExecutionError::NativeFailure(error));
                }
                Err(diagnostic) => {
                    statuses
                        .insert(
                            unit.stratum
                                .clone_admitted(&scope_workspace)
                                .map_err(ExecutionError::NativeFailure)?,
                            ProducerStatus::failed_admitted(&diagnostic, &scope_workspace)
                                .map_err(ExecutionError::NativeFailure)?,
                        )
                        .map_err(ExecutionError::NativeFailure)?;
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
                    unit.stratum.clone_admitted(&scope_workspace).map_err(ExecutionError::NativeFailure)?,
                    ProducerStatus::failed_admitted(&format_args!(
                            "stratum {}: the exclusion lookup its contract declared could not be \
                             derived from its query: {reason}",
                            unit.stratum
                        ), &scope_workspace).map_err(ExecutionError::NativeFailure)?,
                ).map_err(ExecutionError::NativeFailure)?;
                continue;
            }
            Some(Ok(alternatives)) => {
                let prepared_lookups = purrdf_sparql_eval::AdmittedVec::try_from_iter_admitted(
                    alternatives.iter().map(|alternative| {
                        purrdf_sparql_eval::AdmittedVec::try_from_iter_admitted(
                            alternative.iter().map(|lookup| {
                                engine
                                    .prepare_execution_admitted(
                                        &lookup.text,
                                        None,
                                        &[lookup.parameter.as_str()],
                                        options(),
                                        &scope_workspace,
                                    )
                                    .map(|execution| {
                                        let slot = execution.slot(&lookup.parameter).expect(
                                            "the one parameter this execution was prepared with",
                                        );
                                        PreparedLookup {
                                            execution,
                                            slot,
                                            driven: lookup.driven,
                                        }
                                    })
                            }),
                            &scope_workspace,
                        )
                    }),
                    &scope_workspace,
                );
                match prepared_lookups {
                    Ok(lookups) => Some(lookups),
                    Err(purrdf_sparql_eval::EvaluationFailure::Evaluation(error))
                        if scoped.is_some() || error.is_native_storage_failure() =>
                    {
                        return Err(ExecutionError::NativeFailure(error));
                    }
                    Err(diagnostic) => {
                        statuses
                            .insert(
                                unit.stratum
                                    .clone_admitted(&scope_workspace)
                                    .map_err(ExecutionError::NativeFailure)?,
                                ProducerStatus::failed_admitted(
                                    &format_args!(
                                        "stratum {}: the exclusion lookup its producer declared \
                                     could not be prepared: {diagnostic}",
                                        unit.stratum
                                    ),
                                    &scope_workspace,
                                )
                                .map_err(ExecutionError::NativeFailure)?,
                            )
                            .map_err(ExecutionError::NativeFailure)?;
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
            let opened =
                if let (Some(input), Some(scope)) = (scoped.as_ref(), candidate_scope.as_ref()) {
                    engine.open_call_cursor_evidence(
                        &prepared,
                        options().with_candidate_scope(scope),
                        &scope_workspace,
                        input.source,
                    )
                } else {
                    engine.open_call_cursor_admitted(&prepared, options(), &scope_workspace)
                }
                .and_then(|cursor| {
                    CandidateColumns::locate(
                        cursor.variables(),
                        &unit.contract.domains,
                        unit.contract.rank_law,
                    )
                    .map(|columns| (cursor, columns))
                    .map_err(|reason| {
                        purrdf_sparql_eval::RetainedDiagnostic::render(
                            "native-sparql-query-eval",
                            &format_args!("stratum {}: {reason}", unit.stratum),
                            &scope_workspace,
                        )
                        .map_or_else(
                            purrdf_sparql_eval::EvaluationFailure::Evaluation,
                            purrdf_sparql_eval::EvaluationFailure::RetainedDiagnostic,
                        )
                    })
                });
            match opened {
                Ok((cursor, columns)) => {
                    let attestation = cursor.opened().clone();
                    let retained = if let (Some(input), Some(scope)) =
                        (scoped.as_mut(), candidate_scope.as_ref())
                    {
                        let receipt = crate::ScopedStratum::opened(
                            unit.stratum.as_str(),
                            &cursor,
                            scope,
                            exclusion_rows
                                .as_ref()
                                .expect("scoped exclusion owner")
                                .clone(),
                            &scope_workspace,
                        )
                        .map_err(ExecutionError::NativeFailure)?;
                        let rows = receipt.rows.clone();
                        input
                            .receipts
                            .push(receipt)
                            .map_err(ExecutionError::NativeFailure)?;
                        Some(rows)
                    } else {
                        None
                    };
                    let declared_rows = if retained.is_some() {
                        Some(cursor.physical_row_bound())
                    } else {
                        unit.declared_rows()
                    };
                    let read = CallRead {
                        operation: operation.clone(),
                        workspace: scope_workspace.clone(),
                        stratum: unit
                            .stratum
                            .clone_admitted(&scope_workspace)
                            .map_err(ExecutionError::NativeFailure)?,
                        cursor,
                        columns,
                        depth: depth.get(),
                        declared_rows,
                        declared_mode: unit.declared_mode(),
                        reach: unit.reach(),
                        materialised: 0,
                        established: 0,
                        last_rank: 0,
                        retained,
                        candidates: index_candidates.then(|| candidates.clone()),
                        dataset,
                    };
                    if let Some(prepared_lookups) = exclusion {
                        lookups
                            .push(PendingLookup {
                                position: streams.len(),
                                stratum: unit
                                    .stratum
                                    .clone_admitted(&scope_workspace)
                                    .map_err(ExecutionError::NativeFailure)?,
                                lookups: prepared_lookups,
                                pinned: attestation.clone(),
                                scoped: candidate_scope.take().map(|scope| ScopedLookup {
                                    scope,
                                    source: scoped.as_ref().expect("scoped source").source,
                                    workspace: scope_workspace.clone(),
                                    rows: exclusion_rows
                                        .as_ref()
                                        .expect("scoped exclusion owner")
                                        .clone(),
                                }),
                            })
                            .map_err(ExecutionError::NativeFailure)?;
                    }
                    streams
                        .push(ReadStratum {
                            admission_failure_settlement: None,
                            stratum: unit
                                .stratum
                                .clone_admitted(&scope_workspace)
                                .map_err(ExecutionError::NativeFailure)?,
                            plan_id: compiled.plan_id(),
                            requested_depth: depth.get(),
                            contract: unit.contract.clone(),
                            attestation,
                            stream: RankedStreamImpl::on_demand(
                                purrdf_sparql_eval::WorkspaceBox::new_admitted(
                                    read,
                                    &scope_workspace,
                                )
                                .map_err(ExecutionError::NativeFailure)?
                                .coerce(|read| -> Box<dyn OnDemandRead + '_> { read }),
                                &scope_workspace,
                            ),
                        })
                        .map_err(ExecutionError::NativeFailure)?;
                }
                Err(purrdf_sparql_eval::EvaluationFailure::Evaluation(error))
                    if scoped.is_some() || error.is_native_storage_failure() =>
                {
                    return Err(ExecutionError::NativeFailure(error));
                }
                Err(reason) => {
                    if scoped.is_some() {
                        return Err(ExecutionError::SourceRead {
                            reason: reason.to_string(),
                        });
                    }
                    statuses
                        .insert(
                            unit.stratum
                                .clone_admitted(&scope_workspace)
                                .map_err(ExecutionError::NativeFailure)?,
                            ProducerStatus::failed_admitted(&reason, &scope_workspace)
                                .map_err(ExecutionError::NativeFailure)?,
                        )
                        .map_err(ExecutionError::NativeFailure)?;
                }
            }
            continue;
        }
        if let (Some(input), Some(scope)) = (scoped.as_mut(), candidate_scope.as_ref()) {
            let query_options = options().with_candidate_scope(scope);
            let mut execution = engine
                .prepare_execution_admitted(&sparql, None, &[], query_options, &scope_workspace)
                .map_err(|error| match error {
                    purrdf_sparql_eval::EvaluationFailure::Evaluation(error) => {
                        ExecutionError::NativeFailure(error)
                    }
                    diagnostic => ExecutionError::SourceRead {
                        reason: diagnostic.to_string(),
                    },
                })?;
            let (receipt, witness) = engine
                .execute_read_witnessed_admitted(
                    &mut execution,
                    dataset,
                    query_options,
                    Some(input.source),
                    operation,
                    |outcome| {
                        let InternedOutcome::Solutions(solutions) = outcome else {
                            return Err(purrdf_sparql_eval::NativeDiagnostic::error(
                                purrdf_sparql_eval::NativeDiagnosticKind::Internal,
                                "compiled scoped unit did not return solutions",
                                &scope_workspace,
                            ));
                        };
                        let MaterializedCells { variables, rows } =
                            materialize_cells_admitted(&solutions, &scope_workspace)?;
                        let constraints = crate::scope::retain_counts(scope, &scope_workspace)?;
                        let rows = crate::ScopedRows::new(rows, true, &scope_workspace)?;
                        rows.retain_search(solutions.search_evidence())?;
                        Ok(crate::ScopedStratum {
                            stratum: scope_workspace.authored_text(unit.stratum.as_str())?,
                            variables,
                            search: rows.search(),
                            rows,
                            exclusions: exclusion_rows
                                .as_ref()
                                .expect("scoped exclusion owner")
                                .clone(),
                            constraints,
                        })
                    },
                )
                .map_err(|error| match error {
                    purrdf_sparql_eval::EvaluationFailure::Evaluation(error) => {
                        ExecutionError::NativeFailure(error)
                    }
                    diagnostic => ExecutionError::SourceRead {
                        reason: diagnostic.to_string(),
                    },
                })?;
            let receipt = receipt.map_err(ExecutionError::NativeFailure)?;
            let attestation = read_attestation(
                &witness,
                receipt
                    .rows
                    .with_rows(<[crate::ScopedRow]>::is_empty)
                    .map_err(ExecutionError::NativeFailure)?,
            )
            .map_err(|reason| ExecutionError::InconsistentWitness {
                stratum: Box::new(unit.stratum.clone()),
                reason,
            })?;
            let ranked = receipt
                .rows
                .with_rows(|rows| {
                    let known_rows = rows
                        .iter()
                        .filter(|row| !row.evidence.is_undecided())
                        .map(|row| &row.cells[..]);
                    let ranked = rank_candidate_rows(
                        &receipt.variables,
                        known_rows.clone(),
                        &unit.contract.domains,
                        unit.contract.rank_law,
                        &scope_workspace,
                    )
                    .map_err(ExecutionError::NativeFailure)?;
                    if index_candidates {
                        index_candidate_rows(
                            &mut candidates.borrow_mut(),
                            &receipt.variables,
                            known_rows,
                            &ranked,
                            dataset,
                            |error| ExecutionError::NativeFailure(operation.source_error(error)),
                        )?;
                    }
                    Ok::<_, ExecutionError>(ranked)
                })
                .map_err(ExecutionError::NativeFailure)??;
            let (ranked, ending, status) = bound_to_depth(
                ranked,
                depth.get(),
                unit.declared_rows(),
                unit.declared_mode(),
                unit.reach(),
                &unit.stratum,
            )?;
            let materialised = u64::try_from(
                receipt
                    .rows
                    .with_rows(<[crate::ScopedRow]>::len)
                    .map_err(ExecutionError::NativeFailure)?,
            )
            .map_err(|_| ExecutionError::SourceRead {
                reason: "scoped materialized row count overflow".to_owned(),
            })?;
            let stream = RankedStreamImpl::from_admitted(ranked, ending, &scope_workspace)
                .with_materialised_rows(materialised)
                .with_attested(attestation.clone());
            if let Some(prepared_lookups) = exclusion {
                lookups
                    .push(PendingLookup {
                        position: streams.len(),
                        stratum: unit
                            .stratum
                            .clone_admitted(&scope_workspace)
                            .map_err(ExecutionError::NativeFailure)?,
                        lookups: prepared_lookups,
                        pinned: attestation.clone(),
                        scoped: candidate_scope.take().map(|scope| ScopedLookup {
                            scope,
                            source: input.source,
                            workspace: scope_workspace.clone(),
                            rows: exclusion_rows
                                .as_ref()
                                .expect("scoped exclusion owner")
                                .clone(),
                        }),
                    })
                    .map_err(ExecutionError::NativeFailure)?;
            }
            streams
                .push(ReadStratum {
                    admission_failure_settlement: None,
                    stratum: unit
                        .stratum
                        .clone_admitted(&scope_workspace)
                        .map_err(ExecutionError::NativeFailure)?,
                    plan_id: compiled.plan_id(),
                    requested_depth: depth.get(),
                    contract: unit.contract.clone(),
                    attestation,
                    stream,
                })
                .map_err(ExecutionError::NativeFailure)?;
            statuses
                .insert(
                    unit.stratum
                        .clone_admitted(&scope_workspace)
                        .map_err(ExecutionError::NativeFailure)?,
                    status,
                )
                .map_err(ExecutionError::NativeFailure)?;
            input
                .receipts
                .push(receipt)
                .map_err(ExecutionError::NativeFailure)?;
            continue;
        }
        // Governors are per call, never per engine: the value is built here,
        // used once, and dropped with the call. `UNBOUNDED` declines every
        // caller-settable ceiling, so this is the governed lane for its receipt
        // and for nothing else — see this module's header.
        let mut execution = match engine.prepare_execution_admitted(
            &sparql,
            None,
            &[],
            options(),
            &scope_workspace,
        ) {
            Ok(execution) => execution,
            Err(purrdf_sparql_eval::EvaluationFailure::Evaluation(error))
                if scoped.is_some() || error.is_native_storage_failure() =>
            {
                return Err(ExecutionError::NativeFailure(error));
            }
            Err(diagnostic) => {
                statuses
                    .insert(
                        unit.stratum
                            .clone_admitted(&scope_workspace)
                            .map_err(ExecutionError::NativeFailure)?,
                        ProducerStatus::failed_admitted(&diagnostic, &scope_workspace)
                            .map_err(ExecutionError::NativeFailure)?,
                    )
                    .map_err(ExecutionError::NativeFailure)?;
                continue;
            }
        };
        let outcome = engine.execute_read_governed_admitted(
            &mut execution,
            dataset,
            options(),
            &QueryGovernors::UNBOUNDED,
            operation,
            |outcome| match outcome {
                InternedOutcome::Solutions(solutions) => {
                    materialize_cells_admitted(&solutions, &scope_workspace).map(Some)
                }
                _ => Ok(None),
            },
        );
        match outcome {
            Ok(receipt) => {
                let materialized = receipt.consume(
                    |value, _, relations| {
                        let Some(cells) = value.map_err(ExecutionError::NativeFailure)? else {
                            return Ok(None);
                        };
                        let attestation =
                            read_attestation(&relations.witness, cells.rows.is_empty()).map_err(
                                |reason| ExecutionError::InconsistentWitness {
                                    stratum: Box::new(unit.stratum.clone()),
                                    reason,
                                },
                            )?;
                        let materialised = u64::try_from(cells.rows.len()).map_err(|_| {
                            ExecutionError::NativeFailure(
                                purrdf_sparql_eval::EvalError::WorkspaceBoundOverflow,
                            )
                        })?;
                        let ranked = rank_candidate_rows(
                            &cells.variables,
                            cells.rows.iter().map(|row| row.cells.as_slice()),
                            &unit.contract.domains,
                            unit.contract.rank_law,
                            &scope_workspace,
                        )
                        .map_err(ExecutionError::NativeFailure)?;
                        if index_candidates {
                            index_candidate_rows(
                                &mut candidates.borrow_mut(),
                                &cells.variables,
                                cells.rows.iter().map(|row| row.cells.as_slice()),
                                &ranked,
                                dataset,
                                |error| {
                                    ExecutionError::NativeFailure(operation.source_error(error))
                                },
                            )?;
                        }
                        Ok(Some((ranked, materialised, attestation)))
                    },
                    |exhausted| {
                        Err(ExecutionError::InconsistentWitness {
                            stratum: Box::new(unit.stratum.clone()),
                            reason: format!(
                                "the run declined every ceiling, yet {} stopped it",
                                exhausted.tripped
                            ),
                        })
                    },
                );
                let materialized = match materialized {
                    Ok(materialized) => materialized,
                    Err(ExecutionError::NativeFailure(error))
                        if scoped.is_none() && !error.is_native_storage_failure() =>
                    {
                        statuses
                            .insert(
                                unit.stratum
                                    .clone_admitted(&scope_workspace)
                                    .map_err(ExecutionError::NativeFailure)?,
                                ProducerStatus::failed_admitted(
                                    &format_args!("stratum {}: {error}", unit.stratum),
                                    &scope_workspace,
                                )
                                .map_err(ExecutionError::NativeFailure)?,
                            )
                            .map_err(ExecutionError::NativeFailure)?;
                        continue;
                    }
                    Err(error) => return Err(error),
                };
                if let Some((ranked, materialised, attestation)) = materialized {
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
                    let stream = RankedStreamImpl::from_admitted(ranked, ending, &scope_workspace)
                        .with_materialised_rows(materialised)
                        .with_attested(attestation.clone());
                    if let Some(prepared_lookups) = exclusion {
                        lookups
                            .push(PendingLookup {
                                position: streams.len(),
                                stratum: unit
                                    .stratum
                                    .clone_admitted(&scope_workspace)
                                    .map_err(ExecutionError::NativeFailure)?,
                                lookups: prepared_lookups,
                                pinned: attestation.clone(),
                                scoped: None,
                            })
                            .map_err(ExecutionError::NativeFailure)?;
                    }
                    streams
                        .push(ReadStratum {
                            admission_failure_settlement: None,
                            stratum: unit
                                .stratum
                                .clone_admitted(&scope_workspace)
                                .map_err(ExecutionError::NativeFailure)?,
                            plan_id: compiled.plan_id(),
                            requested_depth: depth.get(),
                            contract: unit.contract.clone(),
                            attestation,
                            stream,
                        })
                        .map_err(ExecutionError::NativeFailure)?;
                    statuses
                        .insert(
                            unit.stratum
                                .clone_admitted(&scope_workspace)
                                .map_err(ExecutionError::NativeFailure)?,
                            status,
                        )
                        .map_err(ExecutionError::NativeFailure)?;
                } else {
                    statuses
                        .insert(
                            unit.stratum
                                .clone_admitted(&scope_workspace)
                                .map_err(ExecutionError::NativeFailure)?,
                            ProducerStatus::failed_admitted(
                                "compiled unit did not return solutions",
                                &scope_workspace,
                            )
                            .map_err(ExecutionError::NativeFailure)?,
                        )
                        .map_err(ExecutionError::NativeFailure)?;
                }
            }
            Err(purrdf_sparql_eval::EvaluationFailure::Evaluation(error))
                if scoped.is_some() || error.is_native_storage_failure() =>
            {
                return Err(ExecutionError::NativeFailure(error));
            }
            Err(diagnostic) => {
                statuses
                    .insert(
                        unit.stratum
                            .clone_admitted(&scope_workspace)
                            .map_err(ExecutionError::NativeFailure)?,
                        ProducerStatus::failed_admitted(&diagnostic, &scope_workspace)
                            .map_err(ExecutionError::NativeFailure)?,
                    )
                    .map_err(ExecutionError::NativeFailure)?;
            }
        }
    }

    for PendingLookup {
        position,
        stratum,
        lookups,
        pinned,
        scoped,
    } in lookups
    {
        streams[position].stream.attach_exclusion(
            purrdf_sparql_eval::WorkspaceBox::new_admitted(
                DatasetExclusion {
                    operation: operation.clone(),
                    stratum,
                    engine: engine.clone(),
                    env: env.clone(),
                    lookups,
                    candidates: candidates.clone(),
                    dataset,
                    pinned,
                    scoped,
                },
                &execution_workspace,
            )
            .map_err(ExecutionError::NativeFailure)?
            .coerce(|lookup| -> Box<dyn ExclusionLookup + '_> { lookup }),
        );
    }

    compiled.finish(streams, statuses)
}

/// Native projected cells and original per-row evidence for materialized reads.
struct MaterializedCells {
    variables: purrdf_sparql_eval::AdmittedVec<purrdf_lex::allocation::SharedText>,
    rows: purrdf_sparql_eval::AdmittedVec<crate::ScopedRow>,
}

fn materialize_cells_admitted<D: DatasetView + Sync>(
    solutions: &purrdf_sparql_eval::InternedSolutions<'_, '_, D>,
    workspace: &purrdf_sparql_eval::WorkspaceCapability,
) -> Result<MaterializedCells, purrdf_sparql_eval::EvalError> {
    let mut variables =
        purrdf_sparql_eval::AdmittedVec::with_capacity(solutions.variables().len(), workspace)?;
    for variable in solutions.variables() {
        variables.push(workspace.authored_text(variable.as_str())?)?;
    }
    let mut rows = purrdf_sparql_eval::AdmittedVec::with_capacity(solutions.len(), workspace)?;
    for row in solutions.rows() {
        let mut cells = purrdf_sparql_eval::AdmittedVec::with_capacity(variables.len(), workspace)?;
        for column in 0..variables.len() {
            cells.push(solutions.try_cell_owned(row, column, workspace)?)?;
        }
        rows.push(crate::ScopedRow {
            cells,
            evidence: row.evidence().clone(),
        })?;
    }
    Ok(MaterializedCells { variables, rows })
}

/// Zip original ranking with its cells without rendering lexical identity again.
fn index_candidate_rows<'a, D: DatasetView, C: std::borrow::Borrow<TermValue> + 'a>(
    index: &mut CandidateIndex<D::Id>,
    variables: &[impl AsRef<str>],
    rows: impl IntoIterator<Item = &'a [Option<C>]>,
    ranked: &[(u64, Term, RowBlock)],
    dataset: &D,
    read_error: impl Fn(D::ReadError) -> ExecutionError,
) -> Result<(), ExecutionError> {
    let Some(column) = variables
        .iter()
        .position(|name| name.as_ref() == CANDIDATE_NAME)
    else {
        return Ok(());
    };
    for ((_, term, _), row) in ranked.iter().zip(rows) {
        if let Some(value) = row.get(column).and_then(Option::as_ref) {
            index_candidate(index, term, value.borrow(), dataset, &read_error)?;
        }
    }
    Ok(())
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
    read_error: impl Fn(D::ReadError) -> ExecutionError,
) -> Result<(), ExecutionError> {
    if index.contains_key(term) {
        return Ok(());
    }
    let identity = dataset.term_id_by_value(value).map_err(read_error)?;
    let binding = match identity {
        Some(id) => CandidateBinding::Id(id),
        None => {
            let workspace = index.workspace();
            let value = workspace
                .clone_term(value)
                .map_err(ExecutionError::NativeFailure)?;
            CandidateBinding::Value(
                purrdf_sparql_eval::SharedWorkspace::new_admitted(value, workspace)
                    .map_err(ExecutionError::NativeFailure)?,
            )
        }
    };
    index
        .insert(term.clone(), binding)
        .map_err(ExecutionError::NativeFailure)?;
    Ok(())
}

/// What one stratum's read came to: the rows that reach the stream, how the read
/// ended, and the status that mirrors that ending.
///
/// The three are produced together by [`bound_to_depth`] and must stay together:
/// the ending is a claim about the rows beside it, and a caller holding one
/// without the others could report an exhaustion that the dropped probe row had
/// already falsified.
type BoundedRead<R> = (R, StreamEnding, ProducerStatus);

trait RankedRows: std::ops::Deref<Target = [(u64, Term, RowBlock)]> {
    fn truncate_rows(&mut self, length: usize);
}
impl RankedRows for Vec<(u64, Term, RowBlock)> {
    fn truncate_rows(&mut self, length: usize) {
        self.truncate(length);
    }
}
impl RankedRows for purrdf_sparql_eval::AdmittedVec<(u64, Term, RowBlock)> {
    fn truncate_rows(&mut self, length: usize) {
        self.truncate(length);
    }
}

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
fn bound_to_depth<R: RankedRows>(
    mut ranked: R,
    depth: u32,
    declared_rows: Option<u64>,
    declared_mode: BoundMode,
    reach: ReadReach,
    stratum: &Iri,
) -> Result<BoundedRead<R>, ExecutionError> {
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
        ranked.truncate_rows(ceiling);
        let rank = ranked.last().map_or(0, |row| row.0);
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
        let rank = ranked.last().map_or(0, |row| row.0);
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
    if matches!(reach, ReadReach::AtDepth) && ranked.len() == ceiling
        || matches!(reach, ReadReach::AtBound(bound) if u64::try_from(ranked.len()) == Ok(bound))
    {
        let rank = ranked.last().map_or(0, |row| row.0);
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
#[cfg(test)]
fn rank_candidates(
    variables: &[String],
    rows: &[Vec<Option<TermValue>>],
    domains: &CandidateDomains,
) -> Result<Vec<(u64, Term, RowBlock)>, String> {
    rank_candidate_rows(
        variables,
        rows.iter().map(Vec::as_slice),
        domains,
        purrdf_sparql_eval::RankLaw::Ordinal,
        &purrdf_sparql_eval::WorkspaceCapability::resident(),
    )
    .map(|rows| {
        rows.try_into_resident()
            .expect("resident oracle retains no native account")
    })
    .map_err(|error| error.to_string())
}

fn rank_candidate_rows<'a, C: std::borrow::Borrow<TermValue> + 'a>(
    variables: &[impl AsRef<str> + fmt::Debug],
    rows: impl IntoIterator<Item = &'a [Option<C>]>,
    domains: &CandidateDomains,
    law: purrdf_sparql_eval::RankLaw,
    workspace: &purrdf_sparql_eval::WorkspaceCapability,
) -> Result<purrdf_sparql_eval::AdmittedVec<(u64, Term, RowBlock)>, purrdf_sparql_eval::EvalError> {
    let columns = CandidateColumns::locate(variables, domains, law).map_err(|error| {
        purrdf_sparql_eval::NativeDiagnostic::error(
            purrdf_sparql_eval::NativeDiagnosticKind::Data,
            error,
            workspace,
        )
    })?;
    let rows = rows.into_iter();
    let mut ranked = purrdf_sparql_eval::AdmittedVec::with_capacity(rows.size_hint().0, workspace)?;
    for (index, row) in rows.enumerate() {
        let rank = index
            .checked_add(1)
            .and_then(|index| u64::try_from(index).ok())
            .ok_or(purrdf_sparql_eval::EvalError::WorkspaceBoundOverflow)?;
        ranked.push(columns.rank_admitted(row, rank, workspace)?)?;
    }
    Ok(ranked)
}

/// Where a unit's solutions carry the candidate and the block, found once per read
/// and applied to every row of it — materialised or produced on demand alike, so the
/// two schedules rank a row by one reading of it.
#[derive(Debug)]
pub(crate) struct CandidateColumns {
    /// Required producer rank column for a declared shared-rank stream.
    rank: Option<usize>,
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
    pub(crate) fn locate(
        variables: &[impl AsRef<str> + fmt::Debug],
        domains: &CandidateDomains,
        law: purrdf_sparql_eval::RankLaw,
    ) -> Result<Self, String> {
        let candidate = variables
            .iter()
            .position(|name| name.as_ref() == CANDIDATE_NAME)
            .ok_or_else(|| {
                format!("the unit's solutions project no ?{CANDIDATE_NAME} column: {variables:?}")
            })?;
        Ok(Self {
            rank: match law {
                purrdf_sparql_eval::RankLaw::Ordinal => None,
                purrdf_sparql_eval::RankLaw::Shared => Some(
                    variables
                        .iter()
                        .position(|name| name.as_ref() == crate::compile::RANK_NAME)
                        .ok_or_else(|| {
                            "a shared-rank unit must project its producer's ?rank column".to_owned()
                        })?,
                ),
            },
            candidate,
            block: variables
                .iter()
                .position(|name| name.as_ref() == BLOCK_NAME),
            entailed: entailed_block(domains),
        })
    }

    /// One ranked row retains the native producer's original lexical authority.
    pub(crate) fn rank_admitted<C: std::borrow::Borrow<TermValue>>(
        &self,
        row: &[Option<C>],
        rank: u64,
        workspace: &purrdf_sparql_eval::WorkspaceCapability,
    ) -> Result<(u64, Term, RowBlock), purrdf_sparql_eval::EvalError> {
        let rank = if let Some(column) = self.rank {
            let value = row
                .get(column)
                .and_then(Option::as_ref)
                .map(std::borrow::Borrow::borrow);
            match value {
                Some(TermValue::Literal { lexical_form, datatype, .. }) =>
                    purrdf_xsd::XsdDatatype::from_iri(datatype)
                        .filter(|datatype| datatype.is_integer_family())
                        .and_then(|datatype| purrdf_xsd::numeric::read_integer_typed(
                            purrdf_lex::terminals::trim_ws(lexical_form), datatype).ok())
                        .and_then(|value| u64::try_from(value).ok())
                        .filter(|rank| *rank > 0),
                _ => None,
            }.ok_or_else(|| purrdf_sparql_eval::NativeDiagnostic::error(
                purrdf_sparql_eval::NativeDiagnosticKind::Data,
                format_args!("row {rank} must supply a positive integer-family ?rank representable as u64, got {value:?}"), workspace))?
        } else {
            rank
        };
        self.identity_admitted(row, rank, workspace)
    }

    /// Read an original candidate observation without assigning a rank to an
    /// undecided scoped row. Its evidence retains its original producer rank.
    pub(crate) fn identity_admitted<C: std::borrow::Borrow<TermValue>>(
        &self,
        row: &[Option<C>],
        rank: u64,
        workspace: &purrdf_sparql_eval::WorkspaceCapability,
    ) -> Result<(u64, Term, RowBlock), purrdf_sparql_eval::EvalError> {
        let value = row
            .get(self.candidate)
            .and_then(Option::as_ref)
            .ok_or_else(|| {
                purrdf_sparql_eval::NativeDiagnostic::error(
                    purrdf_sparql_eval::NativeDiagnosticKind::Data,
                    format_args!("the projected ?{CANDIDATE_NAME} column is unbound in row {rank}"),
                    workspace,
                )
            })?;
        let value = value.borrow();
        let block = match self.block {
            None => self.entailed.clone(),
            Some(column) => row_block_admitted(
                row.get(column)
                    .and_then(Option::as_ref)
                    .map(std::borrow::Borrow::borrow),
                rank,
                workspace,
            )?,
        };
        let candidate = crate::render::candidate_lexical_admitted(value, workspace)
            .map(Term::from_admitted)
            .map_err(|error| {
                if error.is_native_storage_failure() {
                    error
                } else {
                    purrdf_sparql_eval::NativeDiagnostic::error(
                        purrdf_sparql_eval::NativeDiagnosticKind::Data,
                        format_args!("row {rank} names no term: {error}"),
                        workspace,
                    )
                }
            })?;
        Ok((rank, candidate, block))
    }

    /// The candidate cell of `row`, when it is bound.
    fn candidate_of<'r, C: std::borrow::Borrow<TermValue>>(
        &self,
        row: &'r [Option<C>],
    ) -> Option<&'r TermValue> {
        row.get(self.candidate)
            .and_then(Option::as_ref)
            .map(std::borrow::Borrow::borrow)
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
    operation: purrdf_sparql_eval::ReadOperation<D::ReadError>,
    /// Native ownership authority shared by the producer and ranked identities.
    workspace: purrdf_sparql_eval::WorkspaceCapability,
    /// The stratum this read is for.
    stratum: Iri,
    /// The open invocation.
    cursor: CallCursor<'static>,
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
    /// Established ranks advance independently of unknown scope rows.
    established: u64,
    /// Last rank handed to the consumer, excluding the probe row.
    last_rank: u64,
    /// The original live receipt, sharing the actual admitted row owner.
    retained: Option<crate::ScopedRows>,
    /// The index an exclusion lookup binds candidates through, when any stratum of
    /// this execution compiled one.
    candidates: Option<purrdf_sparql_eval::SharedWorkspace<RefCell<CandidateIndex<D::Id>>>>,
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
        let invalidated = |reason: String| ReadFault::Producer {
            reason,
            whole_run: true,
        };
        loop {
            let row = self
                .cursor
                .next_admitted_row(self.dataset)
                .map_err(|error| ReadFault::invocation(error, self.retained.is_some()))?;
            if let Some(retained) = &self.retained {
                retained
                    .retain_search(self.cursor.search_evidence())
                    .map_err(ReadFault::Native)?;
            } else if !self.cursor.search_evidence().is_virtual() {
                return Err(invalidated(
                    "an unscoped ranked read cannot discard original search evidence".to_owned(),
                ));
            }
            let Some(row) = row else {
                // Ran out. A caller's own text is the stopper nobody observed past, and
                // the ending names it, exactly as `bound_to_depth` does: a `LIMIT` inside
                // that text may have cut the read before this layer's bound was reached.
                // Filling a depth the read could not be taken past is the producer's own
                // bound stopping it, which nobody could observe past either; anything
                // else is a verified exhaustion.
                let ending = match self.reach {
                    ReadReach::Unknown => StreamEnding::SuppliedQueryEnded {
                        rank: self.last_rank,
                    },
                    ReadReach::AtDepth if self.established == u64::from(self.depth) => {
                        StreamEnding::RowBoundReached {
                            rank: self.last_rank,
                        }
                    }
                    ReadReach::AtBound(bound) if self.established == bound => {
                        StreamEnding::RowBoundReached {
                            rank: self.last_rank,
                        }
                    }
                    ReadReach::AtDepth | ReadReach::AtBound(_) | ReadReach::PastDepth => {
                        StreamEnding::Exhausted
                    }
                };
                return Ok(ReadStep::Ended(ending));
            };
            self.materialised = self
                .materialised
                .checked_add(1)
                .ok_or_else(|| invalidated("physical row count overflow".to_owned()))?;
            if let Some(declared) = self.declared_rows
                && self.materialised > declared
            {
                return Err(ReadFault::Producer {
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
            let unknown = row.evidence.is_undecided();
            if self.retained.is_none() && !row.evidence.is_virtual() {
                return Err(invalidated(
                    "an unscoped ranked read cannot discard original row evidence".to_owned(),
                ));
            }
            let ranked = if unknown {
                None
            } else {
                self.established = self
                    .established
                    .checked_add(1)
                    .ok_or_else(|| invalidated("established rank overflow".to_owned()))?;
                Some(
                    self.columns
                        .rank_admitted(&row.cells, self.established, &self.workspace)
                        .map_err(|error| ReadFault::invocation(error, self.retained.is_some()))?,
                )
            };
            if let Some(index) = &self.candidates
                && let Some(ranked) = &ranked
                && let Some(value) = self.columns.candidate_of(&row.cells)
            {
                index_candidate(
                    &mut index.borrow_mut(),
                    &ranked.1,
                    value,
                    self.dataset,
                    |error| ExecutionError::NativeFailure(self.operation.source_error(error)),
                )
                .map_err(|error| match error {
                    ExecutionError::NativeFailure(error) => ReadFault::Native(error),
                    error => ReadFault::Producer {
                        reason: error.to_string(),
                        whole_run: true,
                    },
                })?;
            }
            if let Some(retained) = &self.retained {
                retained
                    .push(crate::ScopedRow {
                        cells: row.cells,
                        evidence: row.evidence,
                    })
                    .map_err(ReadFault::Native)?;
            }
            let Some(ranked) = ranked else {
                continue;
            };
            if self.established > u64::from(self.depth) {
                return Ok(ReadStep::Ended(StreamEnding::DepthReached {
                    rank: self.last_rank,
                }));
            }
            self.last_rank = ranked.0;
            return Ok(ReadStep::Row(ranked));
        }
    }

    fn materialised(&self) -> u64 {
        self.materialised
    }

    fn settle(&mut self) -> Result<PfAttestation, ReadFault> {
        if let Some(retained) = &self.retained {
            while let Some(row) = self
                .cursor
                .next_admitted_row(self.dataset)
                .map_err(ReadFault::Native)?
            {
                self.materialised = self.materialised.checked_add(1).ok_or(ReadFault::Native(
                    purrdf_sparql_eval::EvalError::WorkspaceBoundOverflow,
                ))?;
                if self
                    .declared_rows
                    .is_some_and(|bound| self.materialised > bound)
                {
                    return Err(ReadFault::Producer {
                        reason: "scoped producer exceeded its original physical row declaration"
                            .to_owned(),
                        whole_run: true,
                    });
                }
                retained
                    .push(crate::ScopedRow {
                        cells: row.cells,
                        evidence: row.evidence,
                    })
                    .map_err(ReadFault::Native)?;
            }
        }
        let witness = self
            .cursor
            .settle()
            .map_err(|error| ReadFault::invocation(error, self.retained.is_some()))?;
        let search = self
            .cursor
            .settled_search_evidence()
            .map_err(ReadFault::Native)?;
        if let Some(retained) = &self.retained {
            retained.retain_search(&search).map_err(ReadFault::Native)?;
        } else if !search.is_virtual() {
            return Err(ReadFault::Producer {
                reason: "an unscoped ranked read cannot discard original search evidence"
                    .to_owned(),
                whole_run: true,
            });
        }
        let attested = sole_attestation(&witness).map_err(|reason| ReadFault::Producer {
            reason,
            whole_run: true,
        })?;
        if let Some(retained) = &self.retained {
            retained.finish().map_err(ReadFault::Native)?;
        }
        Ok(attested)
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
fn row_block_admitted(
    value: Option<&TermValue>,
    rank: u64,
    workspace: &purrdf_sparql_eval::WorkspaceCapability,
) -> Result<RowBlock, purrdf_sparql_eval::EvalError> {
    let value = value.ok_or_else(|| {
        purrdf_sparql_eval::NativeDiagnostic::error(
            purrdf_sparql_eval::NativeDiagnosticKind::Data,
            format_args!(
                "the projected ?{BLOCK_NAME} column is unbound in row {rank}, so that row names \
                 no block of the candidate universe; a producer that declares a block column owes \
                 a block on every row"
            ),
            workspace,
        )
    })?;
    match value {
        // Parsed, never trusted: the tag reaches a consumer that compares it
        // against a host's declared blocks, and a cell that is not an IRI cannot
        // be one of those. The admitted IRI home uses the same validation the registry
        // applied to the declaration, so the two sides are compared after one
        // rule rather than two.
        TermValue::Iri(text) => DomainTag::parse_admitted(text, workspace)
            .map(RowBlock::Declared)
            .map_err(|error| {
                if matches!(&error, purrdf_sparql_eval::EvalError::NativeDiagnostic(diagnostic)
                    if diagnostic.kind() == purrdf_sparql_eval::NativeDiagnosticKind::Data)
                {
                    purrdf_sparql_eval::NativeDiagnostic::error(purrdf_sparql_eval::NativeDiagnosticKind::Data,
                        format_args!("the projected ?{BLOCK_NAME} column in row {rank} is <{text}>, which is not a valid IRI and so names no block: {error}"), workspace)
                } else {
                    // Physical refusals keep their original typed cause; rendering
                    // a grammar explanation after admission failed would hide it.
                    error
                }
            }),
        other => Err(purrdf_sparql_eval::NativeDiagnostic::error(purrdf_sparql_eval::NativeDiagnosticKind::Data, format_args!(
            "the projected ?{BLOCK_NAME} column in row {rank} is {other:?}, and a block of the \
             candidate universe is named by an IRI"
        ), workspace)),
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
#[cfg(test)]
fn term_candidate(value: &TermValue) -> Result<Term, RenderError> {
    candidate_lexical(value).map(Term::new)
}

#[cfg(test)]
mod tests {
    use purrdf_core::TermBox;
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

    #[test]
    fn structured_stream_inspection_leaves_unconsumed_rows_and_receipt_intact() {
        use purrdf_hash::error::ErrorValue;
        use purrdf_testkit::error_capture::Capture;
        let term = Term::new("<http://example.org/remaining>");
        let mut stream = super::RankedStreamImpl::new(
            vec![(u64::MAX, term.clone(), RowBlock::Undeclared)],
            StreamEnding::DepthReached { rank: u64::MAX },
        );
        let fields = Capture::read(&stream);
        assert!(matches!(fields.field("pulled"), ErrorValue::Unsigned(0)));
        let source = fields.child("source");
        let rows = source.child("rows");
        let entries = rows.records("rows");
        assert_eq!(entries.len(), 1);
        assert!(
            matches!(entries[0].field("rank"), ErrorValue::Unsigned(value) if value == u128::from(u64::MAX))
        );
        assert_eq!(
            source.child("ending").code,
            "retrieval-stream-depth-reached"
        );
        drop(fields);
        assert_eq!(
            crate::block_on(stream.next()).unwrap(),
            Some((u64::MAX, term, RowBlock::Undeclared))
        );
        assert_eq!(stream.rows_emitted(), 1);
        assert!(crate::block_on(stream.next()).unwrap().is_none());
    }

    #[derive(Debug)]
    struct RefusingProvider {
        bytes: purrdf_core::SegmentedBytes,
        refuse: std::sync::atomic::AtomicBool,
    }
    impl purrdf_core::SegmentedProvider for RefusingProvider {
        fn snapshot(&self) -> purrdf_core::SegmentedSnapshot {
            purrdf_core::SegmentedProvider::snapshot(&self.bytes)
        }
        fn byte_len(&self) -> u64 {
            purrdf_core::SegmentedProvider::byte_len(&self.bytes)
        }
        fn read_at(
            &self,
            position: u64,
            output: &mut [u8],
        ) -> Result<(), purrdf_core::SegmentedError> {
            if self.refuse.load(std::sync::atomic::Ordering::Relaxed) {
                return Err(purrdf_core::SegmentedError::Provider {
                    operation: "candidate lookup test",
                    host_code: Some(5),
                });
            }
            purrdf_core::SegmentedProvider::read_at(&self.bytes, position, output)
        }
    }

    #[test]
    fn refused_candidate_lookup_cannot_become_an_owned_value_fallback() {
        let value = TermValue::iri("https://example.org/candidate");
        let term = term_candidate(&value).unwrap();
        let limits = purrdf_core::SegmentedBuildLimits::new(8, 4096, 8, 512, 4)
            .unwrap()
            .with_first_term_index((1_u64 << 53) + 7)
            .unwrap();
        let mut builder = purrdf_core::SegmentedBuilder::new(limits).unwrap();
        builder
            .intern_batch(std::slice::from_ref(&value), |_, _| {})
            .unwrap();
        let image = builder.seal().unwrap();
        let provider = std::sync::Arc::new(RefusingProvider {
            bytes: image.provider(),
            refuse: std::sync::atomic::AtomicBool::new(false),
        });
        let source = purrdf_core::SegmentedSession::open(
            provider.clone(),
            image.receipt(),
            purrdf_core::SegmentedReadLimits::new(1_000_000, 2, 256, 1_000_000, 4),
        )
        .unwrap();
        provider
            .refuse
            .store(true, std::sync::atomic::Ordering::Relaxed);
        let mut candidates =
            super::CandidateIndex::new(&purrdf_sparql_eval::WorkspaceCapability::resident());
        let read_error = |error: purrdf_core::SegmentedError| ExecutionError::SourceRead {
            reason: error.to_string(),
        };
        let error = super::index_candidate(&mut candidates, &term, &value, &source, read_error)
            .unwrap_err();
        assert!(matches!(error, ExecutionError::SourceRead { .. }));
        assert_eq!(candidates.len(), 0);
        let healthy = purrdf_core::SegmentedSession::open(
            std::sync::Arc::new(image.provider()),
            image.receipt(),
            purrdf_core::SegmentedReadLimits::new(1_000_000, 2, 256, 1_000_000, 4),
        )
        .unwrap();
        super::index_candidate(&mut candidates, &term, &value, &healthy, read_error).unwrap();
        assert!(
            matches!(candidates.get(&term), Some(super::CandidateBinding::Id(id)) if id.index() > 1_u64 << 53)
        );
    }

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
                s: TermBox::new(TermValue::iri("http://example.org/s")),
                p: TermBox::new(TermValue::iri("http://example.org/p")),
                o: TermBox::new(TermValue::simple_literal("o")),
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
                    s: TermBox::new(TermValue::blank("b0")),
                    p: TermBox::new(TermValue::iri("http://example.org/p")),
                    o: TermBox::new(value.clone()),
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
            s: TermBox::new(TermValue::blank("b0")),
            p: TermBox::new(TermValue::iri("http://example.org/p")),
            o: TermBox::new(tagged.clone()),
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
        let workspace = purrdf_sparql_eval::WorkspaceCapability::resident();
        let candidate = TermValue::iri("http://example.org/member");
        let variables = [CANDIDATE_NAME, crate::compile::RANK_NAME];
        for (variables, row) in [
            (&variables[..1], vec![Some(candidate.clone())]),
            (&variables[..], vec![Some(candidate.clone()), None]),
        ] {
            let reason = super::rank_candidate_rows(
                variables,
                std::iter::once(row.as_slice()),
                &CandidateDomains::Unrestricted,
                purrdf_sparql_eval::RankLaw::Shared,
                &workspace,
            )
            .unwrap_err();
            assert!(reason.to_string().contains("?rank"));
        }
        let row = [
            Some(candidate),
            Some(TermValue::typed_literal(
                "1",
                purrdf_core::datatype::XSD_INTEGER,
            )),
        ];
        let ranked = super::rank_candidate_rows(
            &variables,
            std::iter::once(row.as_slice()),
            &CandidateDomains::Unrestricted,
            purrdf_sparql_eval::RankLaw::Shared,
            &workspace,
        )
        .unwrap();
        assert_eq!(ranked[0].0, 1);
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
        ServiceLevel::incomplete(reason)
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
