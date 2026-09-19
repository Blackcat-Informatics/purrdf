// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The compiler: admitted plan in, independently executable SPARQL out.
//!
//! [`compile`] is the narrow admission waist. It runs every semantic check in
//! [`admission`](crate::admission) and, when the plan is admitted, emits one
//! SPARQL `SELECT` per stratum. Each unit names its stratum's producers by their
//! registered IRIs and is executable through `purrdf-sparql-eval` with no
//! composition layer in the path — the text is the whole executable content, so
//! nothing can live between planning and execution.
//!
//! # Why text, not an algebra node
//!
//! A caller can take an emitted unit and hand it to the evaluator directly; the
//! conformance corpus already insists the evaluator is the single source of query
//! truth, and a text seam lets a caller stand on that same surface without
//! adopting this crate's types. An AST carried between `compile` and `execute`
//! would create a second, non-textual artifact in exactly the gap the design
//! closes.
//!
//! # What a unit contains
//!
//! For each stratum the plan declares, the unit is a `SELECT ?candidate` over the
//! one branch of that stratum's one producer. The branch is a sub-`SELECT` that
//! projects the producer's own declared candidate position under the common name
//! `?candidate`, so strata whose producers name their candidate in different
//! argument positions still read back through one column.
//!
//! # One stratum, one branch — there is no union to emit
//!
//! A stratum carries exactly one producer, refused at registration by
//! [`register_ranked`](purrdf_sparql_eval::PropertyFunctionRegistry::register_ranked)
//! and again at this waist for an edited plan. So there is never a second branch
//! to compose with, and the `UNION` this emitter once wrote could only ever have
//! run against a configuration the registry now refuses to build.
//!
//! It was also never a *merge*. A `UNION` concatenates: the second producer's
//! rank-1 row surfaced at stratum rank `n+1`, below the whole of the first
//! producer's output, and decayed as though it had lost to rows it never competed
//! with; a candidate both producers named arrived twice in one stratum's stream,
//! which an honest `Unique` declaration says cannot happen; and a first producer
//! that filled the depth left the second contributing nothing while the trailer
//! reported a clean exhaustion. Ranks are comparable only within the list that
//! assigned them, which is exactly why the merge belongs either inside one
//! producer (same scoring law, no weight between the two) or across strata in the
//! fusion sum (different scoring laws — or one law whose bounded score cannot
//! carry the weight the host means between two classes), and never in a
//! concatenation here.
//!
//! # The request is in the text
//!
//! Every argument position a producer's declaration places a request facet into
//! carries that facet as a **constant**; only the positions nothing was placed
//! into are free `?cN` variables. The placement rule is
//! [`matching::place`](crate::matching::place) — the same function the planner
//! ran, so admission re-derives the planner's decision rather than a second
//! approximation of it.
//!
//! So `search("quick brown fox")` and `search("")` compile to different queries
//! wherever a producer declared a placement for the needle, which is the whole
//! point of compiling a request rather than an arity. Where a producer declared
//! **none** — a declaration that accepts the shape and writes no part of it —
//! the two compile to the same query, because that producer asked to be called
//! with the request absent from its arguments. That is not a silent loss: such a
//! term is never bound to the producer, and the plan names it in
//! [`Plan::unserved_terms`] as
//! [`UnservedReason::AcceptedWithoutPlacement`](crate::UnservedReason::AcceptedWithoutPlacement).
//! An empty unserved list therefore does mean the text carries every term.
//!
//! # The subject argument list is always parenthesized
//!
//! Even at subject arity one, the subject side is written `( ?c0 )`. A bare
//! subject would make the call's parse depend on the grammar tolerating whatever
//! term landed there (a literal subject, say), whereas a parenthesized group
//! followed by a registered property-function IRI is routed unambiguously to the
//! argument-list production.
//!
//! # The branch carries the stratum's depth, and so does the unit
//!
//! A branch whose producer takes no depth argument is bounded by its own
//! `LIMIT <depth>`, and the unit repeats that `LIMIT` on the outside. The inner
//! one is the producer's licence to stop early — the evaluator offers it to the
//! relation as a row ceiling — and the outer one is the stratum's contract with
//! fusion, which holds whatever bounds the branch below it carries. A producer
//! that declares a [`DepthPlacement`](purrdf_sparql_eval::DepthPlacement) already
//! received the depth as an argument and bounds itself, so its branch carries no
//! `LIMIT`; the outer one still applies.
//!
//! No admitted plan carries a depth of zero, so nothing here emits `LIMIT 0`.
//! That bound reads no rows: whatever the relation holds, the unit hands back
//! nothing, and the stratum is then reported exhausted having emitted nothing —
//! the strongest completeness claim this layer makes, made about the bound rather
//! than about the data, and indistinguishable in every trailer field from an
//! honest empty answer. A bound may narrow a read and
//! must never eliminate one, so the planner floors every derived depth at one,
//! admission refuses a zero outright ([`AdmissionError::ZeroDepth`]), and
//! [`emitted_limit`] floors the emitted bound at one even where the registry's
//! declared row count is zero. Emptiness is reported by the producer, in the
//! receipt fusion verifies against the rows it actually pulled, and a stratum
//! that is to run at all runs deep enough to ask.
//!
//! A stratum that is to read nothing is expressed by carrying no depth entry —
//! which is also how the planner expresses it, since it records a depth only for
//! a stratum a surviving producer ranks under. Absence emits no unit and claims
//! nothing; a zero would have claimed everything.
//!
//! # The depth arrives already narrowed, and this stage narrows nothing
//!
//! A request states how much of the answer it is for
//! ([`ReadBound`](crate::ReadBound)), and the *planner* is what turns that into a
//! depth — see [`plan`](crate::plan) for the rule and its proof. So by the time a
//! plan reaches this waist the narrowing has happened, and
//! [`Plan::stratum_depths`] is the true read.
//!
//! This stage deliberately does not narrow a `LIMIT` on its own. Emitting five
//! rows for a depth the plan recorded as four hundred would make that recorded
//! depth a fiction: every field keyed to it — [`StratumUnit::depth`],
//! [`PlannedResolution::requested_depth`], the executor's reading of the probe row
//! — would describe a read nobody took, and the identity a plan carries would
//! not distinguish the two reads at all. The bound belongs where the depth is
//! decided.
//!
//! What this stage does add is [`CompiledRetrieval::fused_bound`]: the request's
//! bound resolved once, against the plan in hand, into the row count a fusion of
//! these units must be run at. It travels with the streams so that fusing them at
//! some other bound is refused by name rather than served out of depths derived
//! for another question.
//!
//! # The emitted bound is one row deeper than the depth, and that row is a probe
//!
//! A unit bounded at exactly its depth cannot tell the two endings apart that
//! its consumer most needs to distinguish. A producer holding nine rows read at
//! depth three and a producer holding exactly three rows read at depth three
//! both hand back three rows and an empty cursor, so an executor that only ever
//! saw `depth` rows had to guess — and the guess it used to make was
//! [`ProducerStatus::Exhausted`](crate::ProducerStatus), which is the strongest
//! completeness claim this layer can utter, minted for a read the plan itself
//! cut short. That is the zero-depth fault one size larger: a bound on the read
//! silently becoming a statement about the answer.
//!
//! So the emitted bound is `max(1, min(depth, declared row bound) + 1)`, on the
//! branch and on the unit. The extra row is a **read, never a value**:
//! [`execute`](crate::execute) emits at most `depth` rows onto the stream and
//! uses the arrival of the `depth + 1`-th only to end the stream
//! [`DepthReached`](crate::ProducerReceipt::DepthReached) instead of
//! `Exhausted`. No plan field, no identity and no recorded resolution moves by
//! one: [`PlannedResolution::requested_depth`] is the depth, and so is
//! [`StratumUnit::depth`].
//!
//! # The `+ 1` is outside the `min`, and that placement is the whole point
//!
//! Admission already refuses a recorded depth above the registry's declared row
//! bound ([`AdmissionError::DepthBoundViolation`]), so every depth that reaches
//! here is at or below that declaration. The interesting case is *at* it: a
//! stratum planned at exactly the number its producer declared. Writing the
//! probe inside the `min` — `min(depth + 1, declared)` — erases it at precisely
//! that depth, because the `min` then selects `declared`, which is the depth.
//! The unit would be emitted at its own depth, the probe slot would not exist,
//! and the read would be reported `Exhausted` — the strongest completeness claim
//! this layer has, minted for a read a bound cut, with nothing anywhere saying
//! so. That is the same fault as `LIMIT 0`, one size smaller, and it is the
//! fault this whole header exists to close.
//!
//! Adding the row *after* the `min` keeps the slot at every depth. It does not
//! raise the recorded depth, which is the number admission enforces and the
//! number every other field of the bundle is keyed to; it raises the emitted
//! `LIMIT`, which admission does not read at all. `StratumUnit::depth` and
//! [`PlannedResolution::requested_depth`] are unchanged by this, and so is the
//! plan's identity.
//!
//! The probe slot is not an over-refusal either, because it costs nothing when
//! the declaration is honest. A producer that declared it can yield `n` rows per
//! invocation and can really only yield `n` returns `n` rows into an `n + 1` row
//! bound, the slot comes back empty, and `Exhausted` is *verified* rather than
//! believed. A row arriving in it means the producer yielded an `n + 1`-th row
//! after promising there is none, and [`execute`](crate::execute) refuses that
//! run by name ([`ExecutionError::RowBoundBreached`](crate::ExecutionError))
//! rather than certifying the truncation as completeness.
//!
//! A registry that declared no row count at all promised nothing, so there is no
//! declaration for a row to breach: the probe is the only way to learn the
//! ending, and its arrival is the ordinary `DepthReached`.
//!
//! The outer `max(1)` is what a declared **zero** lands on. `min(depth, 0) + 1`
//! is one, which is the floor the planner and admission already apply — that one
//! row is how a producer whose index is empty reports its own emptiness rather
//! than having a `LIMIT 0` report it for them (see `RowBound`). At a declared
//! zero the emitted bound is that floor and there is no slot past the depth, so
//! the refusal above is not reachable there and does not try to be: the layer
//! deliberately reads a zero declaration rather than obeying it, and a row it
//! asked for on purpose cannot be a breach of anything.
//!
//! # The depth *argument* is bounded differently, because it is a request
//!
//! A producer that declares a [`DepthPlacement`](purrdf_sparql_eval::DepthPlacement)
//! is handed the number rather than bounded by it, and the two are not the same
//! kind of thing. A `LIMIT` is applied by the evaluator to a cursor the producer
//! never hears about, so asking for one row more than the registry declared
//! costs nothing and tells the truth about what came back. An argument is a
//! *request*, and a request for `declared + 1` rows asks the producer to exceed
//! the declaration it registered — which a well-built producer refuses, because
//! serving it would be a short answer returned as a complete one. The
//! nearest-neighbour relation refuses exactly that, by name, against its own
//! configured guard.
//!
//! So the argument carries `max(1, min(depth + 1, declared row bound))` — the
//! probe where the declaration leaves room for it, and the declaration itself
//! where it does not. This is not the silent cap the `LIMIT` used to have.
//! The unit's own `LIMIT` still sits at `depth + 1` around such a branch, so a
//! self-bounding producer that returns more rows than it declared is still
//! caught and still refused; what is no longer done is asking it to.
//!
//! Where the two differ — a depth already at the declaration — the ending is the
//! producer's own bound rather than the layer's, and that is the honest reading:
//! the producer was asked for exactly the number it declared it can serve, and
//! bounded itself there. Under-declaring is still loud for such a producer, in
//! the place under-declaring is loud for every producer: admission refuses the
//! plans that ask for more.

use std::collections::BTreeMap;

use purrdf_sparql_eval::{PfDescriptor, RankedDeclaration, RegistryId};

use crate::admission::{AdmissionEnvironment, AdmissionError, RowBound, admit_plan};
use crate::fuse::TopK;
use crate::id::PlanId;
use crate::iri::Iri;
use crate::matching::{PlacementError, place, render_slots};
use crate::plan::{Plan, ProducerBinding};
use crate::ranked_stream::StreamContract;
use crate::reciprocal_rank::MonotoneDepth;
use crate::render::RenderError;
use crate::request::ReadBound;

/// One stratum's independently executable query text, and the contract the rows
/// it returns will arrive under.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StratumUnit {
    /// The caller-supplied stratum the unit ranks within.
    pub stratum: Iri,
    /// A self-contained SPARQL `SELECT` over the stratum's registered relations.
    pub sparql: String,
    /// The duplicate handling and the candidate domains the stratum's producer
    /// declared — read off the registry here, at the one stage that is already
    /// holding the declaration, and carried forward rather than re-fetched.
    ///
    /// A stratum carries exactly one producer, so this is that producer's own
    /// declaration with nothing derived: there is no second producer's contract
    /// to reconcile it with, and none to weaken it to. [`execute`](crate::execute)
    /// tags each stream with it and the fusion engine reads it before pulling a
    /// row, so the promise a stream is held to is the one the registry the unit
    /// was *compiled against* stated — an identity `execute` re-checks before it
    /// runs anything.
    pub contract: StreamContract,
    /// The stratum's planned depth: the most rows this unit may contribute to
    /// the answer, exactly as [`Plan::stratum_depths`] records it.
    ///
    /// This is **not** the `LIMIT` in [`Self::sparql`]. The emitted bound is one
    /// row deeper, at every depth, so that the executor can tell a read the depth
    /// cut from a read that ran out; see this module's header. The probe row is a
    /// read and never a value, so the number a consumer reasons about — and the
    /// number every other field of this bundle is keyed to — is this one.
    ///
    /// It travels on the unit rather than being looked up again from the plan
    /// for the reason [`Self::contract`] does: [`execute`](crate::execute) is
    /// handed the compiled bundle and nothing else, and a depth re-read from a
    /// plan at execution time would be a fact about that plan rather than about
    /// the text that is actually being run.
    pub depth: u32,
    /// The row bound the registry declared for this stratum's one producer, or
    /// `None` where it declared no access mode and so declared no bound at all.
    ///
    /// "Declared nothing" and "declared zero" are different facts and do not
    /// share a representation here for the same reason they do not share one in
    /// admission: a missing declaration can refuse nothing, while a zero is a
    /// measurement of the producer's data.
    ///
    /// [`execute`](crate::execute) needs it to read the probe row. A row
    /// arriving past [`Self::depth`] means something further existed, and only
    /// this number says *what*: below the declaration it is the depth that cut
    /// the read, which is an ordinary
    /// [`ProducerStatus::DepthReached`](crate::ProducerStatus); at the
    /// declaration it is the producer yielding a row after promising there is
    /// none, which is
    /// [`ExecutionError::RowBoundBreached`](crate::ExecutionError). Without it
    /// the executor would have to treat the two alike, and the only ending it
    /// could pick for the second is the completeness claim that breach makes
    /// false.
    ///
    /// It travels on the unit for the reason [`Self::contract`] does: the
    /// declaration this unit was compiled against is the one the run must be
    /// judged by, and a bound re-read from a registry at execution time would be
    /// a fact about that registry rather than about the text being run.
    pub declared_rows: Option<u64>,
}

/// The admitted, compiled plan: the query units plus the identities that pin them.
///
/// `plan_id` is the plan's canonical identity, so a stream or answer can name the
/// pinned plan it descends from; `registry_id` and `registry_fingerprint` are the
/// registry the units were compiled against, so [`execute`](crate::execute) can
/// refuse to run the same text against a different registry and silently obtain a
/// different meaning.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompiledRetrieval {
    /// Per-stratum query units, ordered by stratum IRI.
    pub units: Vec<StratumUnit>,
    /// The canonical identity of the admitted plan.
    pub plan_id: PlanId,
    /// The live registry instance the units were compiled against.
    pub registry_id: RegistryId,
    /// The durable content fingerprint of the registry the units were compiled
    /// against.
    pub registry_fingerprint: String,
    /// The row bound these units' depths were derived for, as the [`TopK`] a
    /// fusion of them must be run at.
    ///
    /// The plan carries a [`ReadBound`], which is a caller's request; this is that
    /// request resolved against the plan in hand, once, at the stage that holds
    /// both. [`ReadBound::Bounded`] resolves to its own row count.
    /// [`ReadBound::Complete`] resolves to the sum of the depths the plan records
    /// — the count at which a fusion of these units provably cannot truncate,
    /// because every unit is bounded at its depth and a fused answer holds each
    /// candidate once.
    ///
    /// [`execute`](crate::execute) tags every stream with it and
    /// [`fuse`](crate::fuse) reads it back, so fusing a bundle at some other bound
    /// is a named refusal
    /// ([`FusionError::ReadBoundMismatch`](crate::FusionError::ReadBoundMismatch))
    /// rather than an answer served out of depths that were derived for a
    /// different question. It travels on the bundle for the reason
    /// [`StratumUnit::depth`] does: `execute` is handed the bundle and nothing
    /// else.
    pub fused_bound: TopK,
    /// Per-stratum rank resolution this plan will fuse at, when the environment
    /// named the profile it will be fused under.
    ///
    /// Empty when it did not. A profile is deliberately not a planning input, so
    /// a caller that has not yet chosen one is not asked to, and gets no
    /// resolution evidence because none can honestly be computed.
    pub resolution: BTreeMap<Iri, PlannedResolution>,
}

/// What a planned depth costs in rank resolution under a named fusion profile.
///
/// This is evidence, not a verdict. A stratum read past its separating range
/// still produces a correct, deterministic answer — the declared tie-break is
/// total — at a coarser resolution, so the honest thing to hand back is the two
/// numbers and let the caller decide, rather than refuse the plan. It is
/// available here, at the waist, rather than only in the fused trailer, so a
/// caller learns what a plan will cost *before* paying to execute it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlannedResolution {
    /// Where this profile stops separating adjacent ranks in this stratum.
    pub separation: MonotoneDepth,
    /// The per-stratum depth the plan recorded.
    pub requested_depth: u32,
}

impl PlannedResolution {
    /// Whether every rank this plan reads is still separated from its
    /// neighbours by score alone.
    ///
    /// `false` does not mean the answer is wrong. It means ranks past the
    /// separating point are ordered by the tie-break's later keys — best stratum
    /// rank ascending, then canonical term bytes — rather than by the fused
    /// score. Ask [`FusionProfile::class_width`](crate::FusionProfile::class_width)
    /// how coarse that is.
    #[must_use]
    pub const fn fully_separated(self) -> bool {
        self.separation.covers(self.requested_depth as u64)
    }
}

/// The name of the variable every branch projects its candidate under, without
/// the `?` sigil.
///
/// Spelled once, because [`execute`](crate::execute) reads the emitted unit's
/// solutions back by exactly this name: two constants that could drift would let
/// the executor look for a column the compiler stopped writing.
pub(crate) const CANDIDATE_NAME: &str = "candidate";

/// The name of the variable a branch projects each row's block under, without
/// the `?` sigil.
///
/// Spelled once beside [`CANDIDATE_NAME`] and for the same reason: the executor
/// finds this column by exactly this name, and a second constant could drift.
///
/// The column is emitted only for a producer whose declaration names the position
/// to read it from
/// ([`RankedDeclaration::block_position`](purrdf_sparql_eval::RankedDeclaration)),
/// so a unit compiled for a producer that names no block per row is
/// byte-identical to the one this compiler emitted before blocks existed.
pub(crate) const BLOCK_NAME: &str = "block";

/// Admit `plan` against `env` and emit its per-stratum SPARQL units.
///
/// The admission checks are documented on [`AdmissionError`]; this function adds
/// only the emission. A plan that passes admission yields exactly one unit per
/// stratum the plan declares that also has at least one bound producer, ordered by
/// stratum IRI, each independently executable through `purrdf-sparql-eval`.
///
/// # Errors
///
/// The distinct [`AdmissionError`] variant of the first violated admission
/// dimension; see [`AdmissionError::dimension`].
pub fn compile(
    plan: &Plan,
    env: &AdmissionEnvironment<'_>,
) -> Result<CompiledRetrieval, AdmissionError> {
    let admitted = admit_plan(plan, env)?;

    let mut units = Vec::new();
    for (stratum, depth) in &plan.stratum_depths {
        let Some(binding) = admitted.stratum_bindings.get(stratum) else {
            // A stratum the plan gives a depth but no producer has no relation
            // to run, so there is nothing to emit for it. That is the only case
            // this arm can reach: admission refuses a binding whose stratum the
            // plan records no depth for, so iterating the depth keys cannot skip
            // a bound producer. Without that check this `continue` would be the
            // silent narrowing — a producer dropped from the emitted text with
            // nothing reporting it.
            continue;
        };
        let declaration = ranked_declaration(binding, &admitted.descriptors)?;
        // The bound the registry declared for this stratum, taken from the map
        // admission already decided the depth against. A stratum the registry
        // declares nothing about cannot appear here — admission refuses a
        // binding whose stratum no ranked producer emits under — but the lookup
        // is still total, and the total answer is the one that enforces nothing:
        // `Undeclared` is silence, and silence bounds no read.
        let bound = admitted
            .stratum_row_bounds
            .get(stratum)
            .copied()
            .unwrap_or(RowBound::Undeclared);
        let sparql = emit_unit(
            plan,
            binding,
            declaration,
            &admitted.descriptors,
            EmittedBounds {
                limit: emitted_limit(*depth, bound),
                depth_argument: depth_argument(*depth, bound),
            },
        )?;
        units.push(StratumUnit {
            stratum: stratum.clone(),
            sparql,
            contract: StreamContract::declared(declaration),
            depth: *depth,
            declared_rows: match bound {
                RowBound::Declared(declared) => Some(declared),
                RowBound::Undeclared => None,
            },
        });
    }
    units.sort_by(|left, right| left.stratum.cmp(&right.stratum));

    // The profile the answer will be fused under is read here for what it can
    // say about this plan's depths, and it says it rather than refusing it. A
    // stratum the profile does not weight contributes nothing to that fusion, so
    // a profile silent about it has nothing to report and gets no entry.
    let resolution = env.fusion_profile.map_or_else(BTreeMap::new, |profile| {
        plan.stratum_depths
            .iter()
            .filter_map(|(stratum, depth)| {
                profile.monotone_depth(stratum).map(|separation| {
                    (
                        stratum.clone(),
                        PlannedResolution {
                            separation,
                            requested_depth: *depth,
                        },
                    )
                })
            })
            .collect()
    });

    Ok(CompiledRetrieval {
        units,
        plan_id: plan.id(),
        registry_id: admitted.instance_id,
        registry_fingerprint: admitted.fingerprint,
        fused_bound: fused_bound(plan),
        resolution,
    })
}

/// The [`TopK`] a fusion of this plan's units must be run at.
///
/// [`ReadBound::Bounded`] is already that number. [`ReadBound::Complete`] asked
/// for everything the strata hold, and what they hold is bounded by the depths the
/// plan records: each unit contributes at most its own depth rows, a fused answer
/// carries each candidate exactly once, so the sum over the depths is a count no
/// fusion of these units can reach — and therefore a bound that truncates
/// nothing. It is the honest resolution of "complete" for a bundle whose reads are
/// themselves bounded, rather than a number picked to be large.
///
/// The sum saturates. A saturated sum is still a bound nothing can reach, because
/// reaching it would need more distinct candidates than a `usize` can count.
fn fused_bound(plan: &Plan) -> TopK {
    match plan.read_bound {
        ReadBound::Bounded(top_k) => top_k,
        ReadBound::Complete => {
            TopK::new(plan.stratum_depths.values().fold(0_usize, |total, depth| {
                total.saturating_add(*depth as usize)
            }))
        }
    }
}

/// The ranked declaration `binding`'s producer supplied at registration.
///
/// Looked up once per stratum and handed to both readers — the emitter, which
/// needs its placements, and the unit's [`StreamContract`], which is two of its
/// fields. One lookup because one declaration: a second read could drift from
/// the first, and the text and the contract must describe the same producer.
fn ranked_declaration<'a>(
    binding: &ProducerBinding,
    descriptors: &'a BTreeMap<String, PfDescriptor>,
) -> Result<&'a RankedDeclaration, AdmissionError> {
    descriptors
        .get(&binding.producer)
        .ok_or_else(|| malformed(binding, "has no registry declaration to compile against"))?
        .ranked
        .as_ref()
        .ok_or_else(|| malformed(binding, "declares no ranked capability to compile against"))
}

/// The row bound the emitted text actually carries for a stratum planned at
/// `depth` over a producer whose registry declared `bound`.
///
/// `max(1, min(depth, declared) + 1)`, and the whole argument for the placement
/// of that `+ 1` — outside the `min`, never inside it — is in this module's
/// header. Inside, it vanishes at `depth == declared`, which is the one depth
/// where the probe is most needed and the one depth an under-declaring producer
/// lands a plan on.
///
/// The addition saturates because it is arithmetic on untrusted input, and the
/// saturation is not a silent narrowing: at `u32::MAX` the extra row is not
/// expressible in a `LIMIT` this emitter can write, so the read ends exactly
/// where it would have ended anyway and is reported as what it is.
///
/// A declared bound wider than a `u32` is clamped before the `min`, which cannot
/// change the answer: `depth` is a `u32`, so a wider bound can never be the
/// smaller of the two.
///
/// # A declared zero still emits one row, and the floor is now structural
///
/// `min(depth, 0)` is zero, and emitting *that* would write `LIMIT 0` for a
/// producer whose every access mode declares zero rows per invocation — a bound
/// that reads nothing: it invokes no relation, and the stratum is then reported
/// exhausted having emitted nothing, which is this layer's strongest completeness
/// claim made about a query that never ran. The same floor of one the planner
/// applies to the depth and admission applies to the bound is owed here, and with
/// the `+ 1` outside the `min` it is no longer a separate clamp that could be
/// dropped: the smallest value this function can return is `0 + 1`. Stating
/// `max(1, …)` on top of that would be a guard with nothing left to guard, so
/// the floor is documented rather than re-applied — and it is the `+ 1`'s
/// placement, not an extra call, that holds it.
fn emitted_limit(depth: u32, bound: RowBound) -> u32 {
    let ceiling = match bound {
        // The declaration caps how far the read is taken, and the probe row sits
        // one past that cap rather than being erased by it: a row arriving there
        // is the producer contradicting its own declaration, which `execute`
        // refuses by name instead of reporting as exhaustion.
        RowBound::Declared(declared) => u32::try_from(declared).unwrap_or(u32::MAX).min(depth),
        // Nothing was declared, so there is no cap and no promise to read the
        // ending off: the probe is the only way to learn whether the depth cut
        // the read.
        RowBound::Undeclared => depth,
    };
    ceiling.saturating_add(1)
}

/// The number handed to a producer that declares a
/// [`DepthPlacement`](purrdf_sparql_eval::DepthPlacement), for a stratum planned
/// at `depth` over a producer whose registry declared `bound`.
///
/// `max(1, min(depth + 1, declared))`, which differs from [`emitted_limit`] at
/// exactly one depth — a depth already at the declaration — and differs there
/// because this number is a *request* rather than a ceiling; this module's
/// header has the argument. Asking a producer for `declared + 1` rows asks it to
/// contradict its own registration, and the shipped nearest-neighbour relation
/// refuses that against its configured guard rather than returning a short
/// answer as a complete one. The `min` here is therefore not the silent cap it
/// was on the `LIMIT`: the unit's own bound still reaches one row past the
/// declaration and still catches a producer that beats it.
///
/// The floor of one is the same floor [`emitted_limit`] carries, for the same
/// reason: a declared zero would otherwise ask such a producer for no rows at
/// all, and an answer of nothing to a request for nothing proves nothing about
/// the index.
fn depth_argument(depth: u32, bound: RowBound) -> u32 {
    let probe = depth.saturating_add(1);
    match bound {
        RowBound::Declared(declared) => u32::try_from(declared)
            .unwrap_or(u32::MAX)
            .min(probe)
            .max(1),
        // Nothing was declared, so there is no registration for the request to
        // exceed and the probe is asked for outright.
        RowBound::Undeclared => probe,
    }
}

/// The two numbers one unit is emitted with: the ceiling the evaluator applies,
/// and the request a self-bounding producer receives.
///
/// They travel together because they are derived together and read one line
/// apart, and they are distinct because they are not the same kind of promise —
/// see this module's header. A single number would have to be one or the other,
/// and whichever it was would be wrong at a depth that sits on the declaration.
#[derive(Clone, Copy, Debug)]
struct EmittedBounds {
    /// The `LIMIT` written on the branch and on the unit, from
    /// [`emitted_limit`].
    limit: u32,
    /// The value rendered into a declared depth placement, from
    /// [`depth_argument`].
    depth_argument: u32,
}

/// Render one stratum's `SELECT` over its one `binding`, bounded at
/// `bounds.limit`.
///
/// That is the emitted bound from [`emitted_limit`], not the stratum's depth:
/// the unit is written one row deeper than the plan reads, so the executor can
/// tell a cut read from an exhausted one.
fn emit_unit(
    plan: &Plan,
    binding: &ProducerBinding,
    declaration: &RankedDeclaration,
    descriptors: &BTreeMap<String, PfDescriptor>,
    bounds: EmittedBounds,
) -> Result<String, AdmissionError> {
    let branch = emit_branch(plan, binding, declaration, descriptors, bounds)?;
    let limit = bounds.limit;
    // The block column rides beside the candidate only where the producer
    // declared a position to read it from. A producer that names no block per row
    // yields the identical text this function has always emitted, so nothing
    // about an existing unit, its identity or its cost moves.
    let projected = if declaration.block_position.is_some() {
        format!("?{CANDIDATE_NAME} ?{BLOCK_NAME}")
    } else {
        format!("?{CANDIDATE_NAME}")
    };
    Ok(format!(
        "SELECT {projected} WHERE {{\n  {branch}\n}}\nLIMIT {limit}"
    ))
}

/// Render the stratum's producer as the unit's one branch, bounded at
/// `bounds.limit` — or, where the producer takes the depth as an argument, by
/// `bounds.depth_argument` rendered into that argument.
fn emit_branch(
    plan: &Plan,
    binding: &ProducerBinding,
    declaration: &RankedDeclaration,
    descriptors: &BTreeMap<String, PfDescriptor>,
    bounds: EmittedBounds,
) -> Result<String, AdmissionError> {
    let descriptor = descriptors
        .get(&binding.producer)
        .ok_or_else(|| malformed(binding, "has no registry declaration to compile against"))?;
    let subject = descriptor.subject_arity;
    let total = subject + descriptor.object_arity;
    if total == 0 {
        return Err(malformed(
            binding,
            "declares zero arguments, so a call cannot name a candidate",
        ));
    }

    // The plan is untrusted input, so what `place` decided when the planner
    // called it is decided again here, by that same function and never by a
    // second approximation: every facet the declaration places renders into a
    // SPARQL constant, no two placements contend for one argument position, and
    // some declared access pattern serves the resulting invocation.
    //
    // What `place` does NOT re-derive is that the bound terms are carried at
    // all. It iterates the matching alternative's placements, so an alternative
    // declaring none gives it nothing to do and it returns success on a binding
    // that transports nothing. That property is a different rule —
    // `matching::carries_content`, which the planner applies when it chooses
    // what to bind — and admission re-derives it before emission is reached
    // (`AdmissionError::HollowBinding`).

    let invocation = place(
        &binding.producer,
        descriptor,
        declaration,
        &plan.request_terms,
        &binding.request_terms,
        bounds.depth_argument,
    )
    .map_err(|error| unsatisfiable(binding, &error))?;
    let candidate = declaration.candidate_position;
    if candidate >= total || invocation.mode.is_bound(candidate) {
        // The registry validates that the candidate position exists and is never
        // a placement target, so this is a registry that moved under the plan.
        return Err(malformed(
            binding,
            "projects a candidate from a position that is not a free argument",
        ));
    }

    // The block column is the candidate column's sibling: a position the unit
    // *reads*, so it must exist and must be free for the invocation to bind
    // anything into. The registry validates both at registration; reaching this
    // means the registry moved under the plan, which is the same claim the
    // candidate's own check makes.
    let block = match declaration.block_position {
        None => None,
        Some(position) if position < total && !invocation.mode.is_bound(position) => Some(position),
        Some(_) => {
            return Err(malformed(
                binding,
                "projects each row's block from a position that is not a free argument",
            ));
        }
    };

    let args = render_slots(&invocation).map_err(|error| unrenderable(binding, &error))?;
    let subject_text = args[..subject].join(" ");
    let object_text = args[subject..].join(" ");
    // A producer that took the depth as an argument bounds itself — with the
    // number `place` just rendered into that argument, which carries the probe
    // row wherever its own declaration leaves room for it; one that did not is
    // bounded here, by the emitted `LIMIT`, which carries the probe row always.
    // Per this module's header, those are two different promises and are
    // deliberately not one number.
    let limit = if declaration.depth_placement.is_none() {
        format!(" LIMIT {}", bounds.limit)
    } else {
        String::new()
    };
    // One projection per position the unit reads back, in the order the executor
    // reads them: the candidate, then the block where the producer declared one.
    let projections = match block {
        None => format!("(?c{candidate} AS ?{CANDIDATE_NAME})"),
        Some(block) => {
            format!("(?c{candidate} AS ?{CANDIDATE_NAME}) (?c{block} AS ?{BLOCK_NAME})")
        }
    };
    Ok(format!(
        "{{ SELECT {projections} WHERE {{ ( {subject_text} ) <{}> ( {object_text} ) }}{limit} }}",
        binding.producer
    ))
}

/// A structural defect in the plan-plus-registry pair, named by producer.
fn malformed(binding: &ProducerBinding, what: &str) -> AdmissionError {
    AdmissionError::MalformedPlan {
        reason: format!("producer {} {what}", binding.producer),
    }
}

/// A producer that cannot be invoked for the terms the plan gives it.
fn unsatisfiable(binding: &ProducerBinding, error: &PlacementError) -> AdmissionError {
    match Iri::parse(&binding.producer) {
        Ok(producer) => AdmissionError::UnsatisfiablePlacement {
            producer: Box::new(producer),
            rule: error.rule(),
            detail: error.to_string(),
            invocation: error.invocation(),
            declared: error.declared(),
        },
        Err(invalid) => AdmissionError::MalformedPlan {
            reason: format!("plan binds invalid producer IRI {invalid}"),
        },
    }
}

/// A placed value that has no SPARQL constant form.
///
/// [`place`] proves every slot it fills renders, so reaching this means the
/// registry moved between the two calls; it is reported on the same dimension
/// because it is the same claim.
fn unrenderable(binding: &ProducerBinding, error: &RenderError) -> AdmissionError {
    match Iri::parse(&binding.producer) {
        Ok(producer) => AdmissionError::UnsatisfiablePlacement {
            producer: Box::new(producer),
            rule: "unrenderable",
            detail: error.to_string(),
            invocation: None,
            declared: Vec::new(),
        },
        Err(invalid) => AdmissionError::MalformedPlan {
            reason: format!("plan binds invalid producer IRI {invalid}"),
        },
    }
}
