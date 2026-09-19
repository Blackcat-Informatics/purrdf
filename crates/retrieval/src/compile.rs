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
//! That bound reads no rows: it invokes no relation, and the stratum is then
//! reported exhausted having emitted nothing — the strongest completeness claim
//! this layer makes, about a query that never ran. An estimate may narrow a read
//! and must never eliminate one, so the planner floors every derived depth at
//! one and admission refuses a zero outright
//! ([`AdmissionError::ZeroDepth`]). Emptiness is reported by the producer, in
//! the receipt fusion verifies against the rows it actually pulled, and a
//! stratum that is to run at all runs deep enough to ask.
//!
//! A stratum that is to read nothing is expressed by carrying no depth entry —
//! which is also how the planner expresses it, since it records a depth only for
//! a stratum a surviving producer ranks under. Absence emits no unit and claims
//! nothing; a zero would have claimed everything.
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
//! So the emitted bound is `min(depth + 1, declared row bound)`, on the branch,
//! on the unit, and in the depth argument of a producer that takes the depth
//! itself — because for such a producer the argument *is* the only bound there
//! is, and a probe the producer never hears about is not a probe. The extra row
//! is a **read, never a value**: [`execute`](crate::execute) emits at most
//! `depth` rows onto the stream and uses the arrival of the `depth + 1`-th only
//! to end the stream [`DepthReached`](crate::ProducerReceipt::DepthReached)
//! instead of `Exhausted`. No plan field, no identity and no recorded
//! resolution moves by one: [`PlannedResolution::requested_depth`] is the
//! depth, and so is [`StratumUnit::depth`].
//!
//! The `min` is what keeps the probe from becoming an over-refusal of its own.
//! A producer that declared it can yield `n` rows per invocation has already
//! promised there is no `n + 1`-th, so asking for one is asking a question
//! whose answer the registry already gave — and at `depth == n` the probe would
//! push the emitted bound past a bound admission itself enforces
//! ([`AdmissionError::DepthBoundViolation`]). At that depth exactly `depth` is
//! emitted and `Exhausted` is honest by contract. A registry that declared no
//! row count at all promised nothing, so there is nothing to read the answer
//! off and the probe is the only way to learn it.

use std::collections::BTreeMap;

use purrdf_sparql_eval::{PfDescriptor, RankedDeclaration, RegistryId};

use crate::admission::{AdmissionEnvironment, AdmissionError, RowBound, admit_plan};
use crate::id::PlanId;
use crate::iri::Iri;
use crate::matching::{PlacementError, place, render_slots};
use crate::plan::{Plan, ProducerBinding};
use crate::ranked_stream::StreamContract;
use crate::reciprocal_rank::MonotoneDepth;
use crate::render::RenderError;

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
    /// row deeper wherever the registry left room for it, so that the executor
    /// can tell a read the depth cut from a read that ran out; see this module's
    /// header. The probe row is a read and never a value, so the number a
    /// consumer reasons about — and the number every other field of this bundle
    /// is keyed to — is this one.
    ///
    /// It travels on the unit rather than being looked up again from the plan
    /// for the reason [`Self::contract`] does: [`execute`](crate::execute) is
    /// handed the compiled bundle and nothing else, and a depth re-read from a
    /// plan at execution time would be a fact about that plan rather than about
    /// the text that is actually being run.
    pub depth: u32,
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
            emitted_limit(*depth, bound),
        )?;
        units.push(StratumUnit {
            stratum: stratum.clone(),
            sparql,
            contract: StreamContract::declared(declaration),
            depth: *depth,
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
        resolution,
    })
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
/// `min(depth + 1, declared)`, and the whole argument for both halves is in this
/// module's header. The addition saturates because it is arithmetic on untrusted
/// input, and the saturation is not a silent narrowing: at `u32::MAX` the extra
/// row is not expressible in a `LIMIT` this emitter can write, so the read ends
/// exactly where it would have ended anyway and is reported as what it is.
///
/// A declared bound wider than a `u32` is clamped before the `min`, which cannot
/// change the answer: `depth + 1` is a `u32`, so a wider bound can never be the
/// smaller of the two.
fn emitted_limit(depth: u32, bound: RowBound) -> u32 {
    let probe = depth.saturating_add(1);
    match bound {
        // The producer already promised there is no row past `declared`, so a
        // probe for one asks a question the registry answered at registration —
        // and at `depth == declared` it would also push the emitted bound past
        // the very number admission holds the depth to.
        RowBound::Declared(declared) => u32::try_from(declared).unwrap_or(u32::MAX).min(probe),
        // Nothing was declared, so there is no promise to read the answer off
        // and the probe is the only way to learn whether the depth cut the read.
        RowBound::Undeclared => probe,
    }
}

/// Render one stratum's `SELECT` over its one `binding`, bounded at `limit`.
///
/// `limit` is the emitted bound from [`emitted_limit`], not the stratum's depth:
/// the unit is written one row deeper than the plan reads wherever the registry
/// left room, so the executor can tell a cut read from an exhausted one.
fn emit_unit(
    plan: &Plan,
    binding: &ProducerBinding,
    declaration: &RankedDeclaration,
    descriptors: &BTreeMap<String, PfDescriptor>,
    limit: u32,
) -> Result<String, AdmissionError> {
    let branch = emit_branch(plan, binding, declaration, descriptors, limit)?;
    Ok(format!(
        "SELECT ?{CANDIDATE_NAME} WHERE {{\n  {branch}\n}}\nLIMIT {limit}"
    ))
}

/// Render the stratum's producer as the unit's one branch, bounded at `limit`.
fn emit_branch(
    plan: &Plan,
    binding: &ProducerBinding,
    declaration: &RankedDeclaration,
    descriptors: &BTreeMap<String, PfDescriptor>,
    limit: u32,
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
        limit,
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

    let args = render_slots(&invocation).map_err(|error| unrenderable(binding, &error))?;
    let subject_text = args[..subject].join(" ");
    let object_text = args[subject..].join(" ");
    // A producer that took the depth as an argument bounds itself — with the
    // number `place` just rendered into that argument, which is the emitted
    // bound and therefore carries the probe row too; one that did not is bounded
    // here, per this module's header.
    let limit = if declaration.depth_placement.is_none() {
        format!(" LIMIT {limit}")
    } else {
        String::new()
    };
    Ok(format!(
        "{{ SELECT (?c{candidate} AS ?{CANDIDATE_NAME}) WHERE {{ ( {subject_text} ) <{}> ( {object_text} ) }}{limit} }}",
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
