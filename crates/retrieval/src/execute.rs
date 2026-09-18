// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The executor: each admitted unit runs independently through the evaluator.
//!
//! [`execute`] is the only stage that runs a query. It runs one unit per stratum
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
//! [`RankedStreamImpl`] carries `(rank, candidate)` only. A fusion contribution
//! depends on the fusion profile's weights and smoothing constant, which are
//! deliberately not a plan input, so the contribution is attached at `fuse` time.
//! Keeping the executor profile-free is what lets the unfused rung be consumed
//! with no fusion law in the path at all. It is not what makes it cheap: a
//! stratum's rows are materialized by the evaluator before the first one is
//! read, so that rung costs what the stratum's own result costs.
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
//! its rank ordering and its duplicate handling — carried through from the
//! compiled unit into [`StratumStream::contract`]. That is not a fusion input
//! the way a weight is: it is the producer's declaration about its own rows, and
//! the same one-stratum-one-producer rule the ranks rest on makes it
//! unambiguous, so a stratum's contract is simply its producer's. It travels
//! with the rows rather than being looked up again at the end, for the reason
//! the plan identity does. The attestation read off the witness travels the
//! same way, in [`StratumStream::attestation`].
//!
//! # The unit is read one row deeper than the stratum is
//!
//! [`compile`](crate::compile) emits `LIMIT min(depth + 1, declared row bound)`,
//! so a unit whose producer still had rows past the planned depth hands back one
//! more row than the stratum may contribute. That row is a **probe**: it is
//! never emitted onto the stream, never ranked, and never counted anywhere. All
//! it does is decide the stream's ending — [`StreamEnding::DepthReached`] when
//! it arrived, [`StreamEnding::Exhausted`] when it did not. Without it an
//! executor could only ever say `Exhausted`, which is the strongest
//! completeness claim this layer makes, uttered about a read the plan itself cut
//! short.

use std::collections::{HashMap, VecDeque};

use purrdf_core::{DatasetView, SparqlResult, TermValue};
use purrdf_sparql_eval::{
    GovernedOutcome, NativeSparqlEngine, PfAttestation, PropertyFunctionRegistry, QueryGovernors,
    QueryOptions, RegistryId, RelationWitness, ServiceLevel,
};

use crate::compile::{CANDIDATE_NAME, CompiledRetrieval};
use crate::fusion_stream::ProducerStatus;
use crate::id::PlanId;
use crate::iri::{Iri, Term};
use crate::ranked_stream::{ProducerReceipt, ProtocolError, StreamContract};
use crate::render::candidate_lexical;

/// One stratum's ranked rows, tagged with the pinned plan they descend from and
/// the contract its producer declared them under.
#[derive(Debug)]
pub struct StratumStream {
    /// The stratum the stream's ranks are within.
    pub stratum: Iri,
    /// The admitted plan the stream was compiled from.
    pub plan_id: PlanId,
    /// The rank ordering and duplicate handling the stratum's producer declared,
    /// carried from [`StratumUnit::contract`](crate::StratumUnit).
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
    /// [`PfAttestation::UNDECLARED`] is the honest answer for a unit whose
    /// relation declared neither fact, and it stays an absence all the way out:
    /// silence is never a certificate that the index was current or whole.
    pub attestation: PfAttestation,
    /// The evaluator's rows, in rank order.
    pub stream: RankedStreamImpl,
}

/// The result of running every compiled unit: the surviving streams and every
/// stratum's status.
///
/// A failed stratum appears in `statuses` as
/// [`ProducerStatus::ExecutionFailed`] and has no entry in `streams`; a surviving
/// stratum appears in both. Nothing reduces the statuses to one flag.
#[derive(Debug)]
pub struct ExecutionResult {
    /// One stream per stratum that ran, ordered as compiled.
    pub streams: Vec<StratumStream>,
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
/// this one is reached by a real observation the executor now makes.
///
/// What makes it whole-run rather than per-stratum is *what it observes*. A
/// witness that does not describe exactly one relation, one index generation
/// and at most one incompleteness reason is not a report about one stratum's
/// producer behaving badly; it is the evidence channel disagreeing with the
/// admission waist about what was registered, or an index moving underneath the
/// query. Neither fact is confined to the stratum that noticed it, and the
/// remaining strata's answers rest on the same two assumptions.
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
}

/// A concrete ranked stream of `(rank, candidate)` rows.
///
/// The rows are materialized by the evaluator and drained in order; `next` never
/// pends, so the stream is usable under any executor. A caller that wants to fuse
/// the rows wraps them with the fusion profile at `fuse` time.
///
/// There is deliberately no bulk accessor beside [`next`](Self::next): reading
/// the rows one at a time and then taking the [`receipt`](Self::receipt) *is*
/// the unfused rung, and a second way to get at the same rows would be a second
/// protocol — one with no receipt at the end of it, and so no way for a caller
/// to tell a stratum that ended from a stratum it stopped reading.
#[derive(Debug)]
pub struct RankedStreamImpl {
    rows: VecDeque<(u64, Term)>,
    pulled: u64,
    exhausted: bool,
    ending: StreamEnding,
}

/// How a materialized stream ends, decided before the first row is pulled.
///
/// The two endings answer "what stopped this read", and only the producer's own
/// side of the seam can tell them apart: an empty cursor looks identical whether
/// the rows ran out or the bound did. [`execute`] can tell, because
/// [`compile`](crate::compile) emits a bound one row deeper than the stratum
/// reads, so the arrival of that extra row *is* the distinction — see this
/// module's header.
///
/// It is fixed at construction rather than computed in
/// [`RankedStreamImpl::receipt`] because the fact is about the evaluator's
/// answer, not about how much of the stream a consumer chose to pull: a stream
/// whose ending were derived at the end would say something different to a
/// caller that stopped early, which is precisely the falsifiable status the
/// ranked-stream protocol forbids.
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
}

impl RankedStreamImpl {
    /// Build a stream over pre-ranked `(rank, candidate)` rows that ends the way
    /// `ending` says.
    ///
    /// The ending is a parameter rather than something inferred from `rows`,
    /// because it cannot be inferred from `rows`: the row count is the same
    /// either way, and the whole point of the distinction is that only the
    /// reader of the underlying answer knows which one happened.
    #[must_use]
    pub fn new(rows: Vec<(u64, Term)>, ending: StreamEnding) -> Self {
        Self {
            rows: rows.into(),
            pulled: 0,
            exhausted: false,
            ending,
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
    pub async fn next(&mut self) -> Result<Option<(u64, Term)>, ProtocolError> {
        match self.rows.pop_front() {
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
    /// depth stopped it at. Fusion measures either number against the rows it
    /// pulled and refuses a disagreement
    /// ([`ProtocolError::ForgedReceipt`]), which is why the rank is the depth
    /// the rows were truncated to and not the number of rows the unit returned.
    ///
    /// # Errors
    ///
    /// [`ProtocolError::NeverEndingSource`] when called before the stream was
    /// drained, because a completeness claim from a partially read stream is
    /// exactly the falsifiable status the protocol forbids.
    #[allow(clippy::unused_async, clippy::unused_async_trait_impl)]
    pub async fn receipt(&mut self) -> Result<ProducerReceipt, ProtocolError> {
        if !self.exhausted {
            return Err(ProtocolError::NeverEndingSource);
        }
        Ok(match self.ending {
            StreamEnding::Exhausted => ProducerReceipt::Exhausted {
                rows_emitted: self.pulled,
            },
            StreamEnding::DepthReached { rank } => ProducerReceipt::DepthReached { rank },
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
pub async fn execute<D: DatasetView + Sync>(
    compiled: &CompiledRetrieval,
    registry: &PropertyFunctionRegistry,
    dataset: &D,
) -> Result<ExecutionResult, ExecutionError> {
    if compiled.registry_id != registry.instance_id() {
        return Err(ExecutionError::RegistryMismatch {
            expected: compiled.registry_id,
            got: registry.instance_id(),
        });
    }

    let engine = NativeSparqlEngine::new();
    let mut streams = Vec::with_capacity(compiled.units.len());
    let mut statuses = HashMap::with_capacity(compiled.units.len());
    // The registry is named identically at prepare and at evaluation: the
    // evaluator refuses a plan prepared against a different registry than the
    // one it is run under, because a plan prepared without one has already
    // lowered every relation's predicate to an ordinary triple pattern. One
    // spelling, called twice, so the two sites cannot drift apart.
    let options = || QueryOptions {
        property_functions: registry,
        ..QueryOptions::EMPTY
    };

    for unit in &compiled.units {
        if unit.sparql.trim().is_empty() {
            statuses.insert(
                unit.stratum.clone(),
                ProducerStatus::ExecutionFailed {
                    reason: "compiled unit is empty".to_owned(),
                },
            );
            continue;
        }
        let prepared = match engine.prepare_query_with_options(&unit.sparql, None, options()) {
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
                match rank_candidates(&variables, &rows) {
                    Ok(ranked) => {
                        let (ranked, ending, status) = bound_to_depth(ranked, unit.depth);
                        streams.push(StratumStream {
                            stratum: unit.stratum.clone(),
                            plan_id: compiled.plan_id,
                            contract: unit.contract,
                            attestation,
                            stream: RankedStreamImpl::new(ranked, ending),
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
                // `UNBOUNDED` declines every caller-settable ceiling, so a trip
                // is the same class of impossibility the witness rule catches:
                // something other than this call's governors stopped the run,
                // and whatever rows it reached are a partial answer nothing here
                // asked for. It is reported as the whole-run refusal it is,
                // naming the governor, rather than as a stratum that answered.
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

    Ok(ExecutionResult { streams, statuses })
}

/// Cut `ranked` down to the stratum's `depth`, and say which ending that was.
///
/// The unit was emitted one row deeper than `depth` wherever the registry left
/// room, so a `depth + 1`-th row here means the producer still had rows when the
/// plan's bound ran out. That row is dropped — it is a probe and never a value —
/// and its only effect is the ending. Every other row keeps the rank
/// [`rank_candidates`] gave it, so nothing is renumbered.
///
/// The status returned is the mirror of the ending, so the terminal report and
/// the stream's own receipt cannot say different things about the same read.
fn bound_to_depth(
    mut ranked: Vec<(u64, Term)>,
    depth: u32,
) -> (Vec<(u64, Term)>, StreamEnding, ProducerStatus) {
    let ceiling = usize::try_from(depth).unwrap_or(usize::MAX);
    if ranked.len() > ceiling {
        ranked.truncate(ceiling);
        let rank = u64::from(depth);
        return (
            ranked,
            StreamEnding::DepthReached { rank },
            ProducerStatus::DepthReached { rank },
        );
    }
    let rows_emitted = u64::try_from(ranked.len()).unwrap_or(u64::MAX);
    (
        ranked,
        StreamEnding::Exhausted,
        ProducerStatus::Exhausted { rows_emitted },
    )
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

/// Read a unit's projected candidate column into ranked `(rank, candidate)` rows.
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
) -> Result<Vec<(u64, Term)>, String> {
    let column = variables
        .iter()
        .position(|name| name == CANDIDATE_NAME)
        .ok_or_else(|| {
            format!("the unit's solutions project no ?{CANDIDATE_NAME} column: {variables:?}")
        })?;
    let mut ranked = Vec::with_capacity(rows.len());
    for (index, row) in rows.iter().enumerate() {
        let rank = u64::try_from(index + 1).unwrap_or(u64::MAX);
        let value = row.get(column).and_then(Option::as_ref).ok_or_else(|| {
            format!("the projected ?{CANDIDATE_NAME} column is unbound in row {rank}")
        })?;
        ranked.push((rank, term_candidate(value)));
    }
    Ok(ranked)
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
fn term_candidate(value: &TermValue) -> Term {
    Term::new(candidate_lexical(value))
}

#[cfg(test)]
mod tests {
    use purrdf_core::TermValue;
    use purrdf_sparql_eval::{IndexGeneration, PfAttestation, RelationWitness, ServiceLevel};

    use super::{
        ProducerStatus, StreamEnding, bound_to_depth, rank_candidates, read_attestation,
        sole_attestation, term_candidate,
    };
    use crate::compile::CANDIDATE_NAME;
    use crate::iri::Term;
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
            let candidate = term_candidate(&value);
            assert_eq!(
                decode_term(candidate.as_str()),
                Ok(value),
                "the candidate {} reads back as the value it names",
                candidate.as_str()
            );
        }
        assert_eq!(
            term_candidate(&TermValue::iri("http://example.org/doc")).as_str(),
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
        )
        .expect("a blank node among the answers is still an answer");

        assert_eq!(
            ranked
                .iter()
                .map(|(_, term)| term.as_str())
                .collect::<Vec<_>>(),
            vec![
                "<http://example.org/doc>",
                "_:b0",
                "<http://example.org/other>"
            ],
            "the blank node is named in place, and its neighbours keep their ranks"
        );
        assert_eq!(
            ranked.iter().map(|(rank, _)| *rank).collect::<Vec<_>>(),
            vec![1, 2, 3],
            "ranks stay 1-based and contiguous, so no row was dropped"
        );
        assert_eq!(
            decode_term("_:b0"),
            Ok(TermValue::blank("b0")),
            "and the decoder reads the label back, so nothing is lost"
        );
    }

    #[test]
    fn the_candidate_column_is_read_by_name_not_by_first_binding() {
        // A row whose earlier column is bound and whose candidate column is not
        // is a refusal, never the earlier column's term promoted into the rank.
        let variables = vec!["other".to_owned(), CANDIDATE_NAME.to_owned()];
        let rows = vec![vec![Some(TermValue::iri("http://example.org/other")), None]];
        let reason = rank_candidates(&variables, &rows)
            .expect_err("an unbound candidate column is a refusal");
        assert!(reason.contains("unbound"), "{reason}");

        // Bound in the candidate column, it is that column that ranks.
        let rows = vec![vec![
            Some(TermValue::iri("http://example.org/other")),
            Some(TermValue::iri("http://example.org/doc")),
        ]];
        assert_eq!(
            rank_candidates(&variables, &rows).expect("the candidate column ranks"),
            vec![(1, Term::new("<http://example.org/doc>".to_owned()))]
        );
    }

    #[test]
    fn a_unit_that_projects_no_candidate_column_is_refused() {
        let reason = rank_candidates(&["other".to_owned()], &[vec![None]])
            .expect_err("a unit with no candidate column cannot be ranked");
        assert!(reason.contains(CANDIDATE_NAME), "{reason}");
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
        IndexGeneration::Declared(generation.to_owned())
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

    /// The `depth + 1`-th row decides the ending and is then dropped; every row
    /// that stays keeps the rank it was given, so nothing is renumbered.
    #[test]
    fn the_probe_row_changes_the_ending_and_nothing_else() {
        let rows = |count: u64| {
            (1..=count)
                .map(|rank| (rank, Term::new(format!("<http://example.org/doc{rank}>"))))
                .collect::<Vec<_>>()
        };

        let (kept, ending, status) = bound_to_depth(rows(4), 3);
        assert_eq!(
            kept.iter().map(|(rank, _)| *rank).collect::<Vec<_>>(),
            vec![1, 2, 3],
            "the probe row is dropped and its neighbours keep their ranks"
        );
        assert_eq!(ending, StreamEnding::DepthReached { rank: 3 });
        assert_eq!(status, ProducerStatus::DepthReached { rank: 3 });

        // Exactly at the depth: the probe never arrived, so the read ran out.
        let (kept, ending, status) = bound_to_depth(rows(3), 3);
        assert_eq!(kept.len(), 3);
        assert_eq!(ending, StreamEnding::Exhausted);
        assert_eq!(status, ProducerStatus::Exhausted { rows_emitted: 3 });

        // And below it, where the depth was never the binding constraint.
        let (kept, ending, status) = bound_to_depth(rows(1), 3);
        assert_eq!(kept.len(), 1);
        assert_eq!(ending, StreamEnding::Exhausted);
        assert_eq!(status, ProducerStatus::Exhausted { rows_emitted: 1 });
    }
}
