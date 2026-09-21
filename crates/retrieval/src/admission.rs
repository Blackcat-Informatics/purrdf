// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Semantic admission: the narrow waist every plan passes through.
//!
//! A [`Plan`](crate::Plan) is a value a caller can inspect, edit and deserialize,
//! which is exactly what makes it untrusted input: an edited plan that drops a
//! producer still compiles to well-formed query text that silently answers less
//! than the registry promised. [`compile`](crate::compile) therefore admits the
//! plan against the registry's declared invariants before emitting anything, and
//! this module holds that admission logic and its typed refusals.
//!
//! # Admission is semantic, not syntactic
//!
//! Nothing here checks that a plan is *decodable* — [`Plan::from_canonical_bytes`]
//! and serde already own that — or that the emitted text parses. Admission checks
//! the plan against what the registry **declared**:
//!
//! * every producer the registry declares **mandatory** is present, is bound to
//!   the stratum the registry ranks it under, and receives every request term
//!   **its own declaration accepts and places**;
//! * every stratum carries at most one binding, which is the plan-side face of
//!   the registry's one-stratum-one-producer rule;
//! * every stratum depth respects the registry's declared row bound, and — when
//!   the environment names the fusion profile the answer will be composed under
//!   — the depth at which that profile's own arithmetic stops ordering ranks;
//! * every stratum depth is at least one ([`AdmissionError::ZeroDepth`]): a
//!   stratum bounded at nothing is never allowed to answer and would still be
//!   reported exhausted, so the one thing a depth may never be is a claim of
//!   emptiness that no producer made;
//! * every stratum depth is shallow enough for the emitted bound to carry its
//!   probe row ([`AdmissionError::DepthWithoutProbe`]) — the same rule as the one
//!   above, read at the other end of the range. The unit is emitted one row
//!   deeper than the depth so the executor can tell a read the bound cut from a
//!   read that ran out; at [`u32::MAX`] that row is not expressible in the
//!   `LIMIT` the compiler writes, and the read would be reported
//!   [`Exhausted`](crate::ProducerStatus::Exhausted) — the one ending that names
//!   no stopper — for a stratum whose ending nobody was able to observe.
//!   A depth whose ending cannot be verified is refused rather than certified,
//!   and [`ProbedDepth`] is the type that carries the proof to the compiler;
//! * every bound producer can actually be *invoked* for the request terms the
//!   plan gives it — every declared placement renders, no two contend for one
//!   position, and some declared access pattern serves the result;
//! * every producer the plan binds is bound to a stratum the plan records a
//!   depth for, so a compiler that emits one unit per depth entry cannot skip a
//!   bound producer;
//! * every request term a binding names is one that producer's own declaration
//!   can **receive** — accepted, and placed somewhere by the alternative that
//!   accepted it — because a binding is the claim that the producer got the term
//!   ([`AdmissionError::HollowBinding`]);
//! * every request term the plan records as unserved addresses the plan's own
//!   request, is recorded once, and is not also bound to a producer — a plan
//!   that both serves a term and reports it unanswered contradicts itself;
//! * the plan was planned against the registry the environment now supplies (the
//!   durable content fingerprint first, then the live instance identity for a
//!   plan that never left this process) and against the statistics revision the
//!   environment reports.
//!
//! # Which registry identity a plan is held to
//!
//! A plan records two registry identities, and they are not interchangeable.
//! [`Plan::registry_content_fingerprint`](crate::Plan::registry_content_fingerprint)
//! is durable: it digests what the registry *declares*, so two registries built
//! independently — in two processes, on two machines — produce the same value
//! when they declare the same relations. It is checked for every plan, always,
//! and is never relaxed.
//!
//! [`Plan::registry_instance_id`](crate::Plan::registry_instance_id) is a
//! process-lifetime counter. Within one process it sees what the fingerprint
//! cannot: two registries can declare byte-identical contents and still register
//! one IRI to two implementations that answer differently. Across a process
//! boundary it sees nothing at all — the counter restarts, so a plan that was
//! serialized and reloaded carries a number no live registry can match, and
//! comparing it would refuse every portable plan there is.
//!
//! So the comparison is made against the plan's own recorded origin
//! ([`PlanOrigin`](crate::PlanOrigin)): a
//! [`SameProcess`](crate::PlanOrigin::SameProcess) plan must name the live
//! instance, and a [`Deserialized`](crate::PlanOrigin::Deserialized) plan stands
//! on the content fingerprint, which is the strongest claim it can make. Origin
//! is recorded by the decode paths rather than inferred here, because a foreign
//! counter and a stale one are the same number.
//!
//! # Mandatory is read, never inferred
//!
//! Admission adds no coverage policy of its own. Whether a producer is mandatory
//! is registry policy, declared as
//! [`RankedDeclaration::mandatory`](purrdf_sparql_eval::RankedDeclaration::mandatory)
//! through the same capability a planner reads; this module enforces exactly
//! what was declared and nothing more. In particular it is not derived from a
//! producer's declared patterns: a permissive producer that happens to accept
//! every request shape is **not** thereby made un-droppable, and a plan that
//! binds such a producer to a subset of the request — or drops it entirely — is
//! admitted, because the registry never said it had to be there.
//!
//! That is a claim about *whether* the flag is set, which only the host says.
//! How far a set flag reaches is a different question, and the next section
//! answers it — from the producer's own declared patterns, because a promise
//! cannot extend past what the thing promising can do.
//!
//! ## What mandatory quantifies over
//!
//! Mandatory means *every request term the producer's own patterns accept*, not
//! *every request term*. The two readings differ on exactly the configuration a
//! coverage floor is built from: a real full-text producer accepts literals and
//! cannot accept an entity seed, so under the wider reading a host that declared
//! it mandatory had every multimodal request refused — a `Lexical` term beside an
//! `EntitySeed` yielded [`InsufficientBindings`](AdmissionError::InsufficientBindings)
//! for a plan in which the text producer was doing precisely its job. That is the
//! over-refusal mirror of the silent drop, and it hides well: withdrawing
//! `mandatory` admits the same request, so the declaration looked like the
//! problem when the quantifier was.
//!
//! Dropping a mandatory producer from a term it **does** accept is still the
//! refusal, and that is the whole of what the flag protects. A term it cannot
//! accept is not its business, and neither is a request it accepts nothing of:
//! there is no term to have dropped it from, so no coverage claim the request can
//! falsify, and the producer is simply not part of that request's answer.
//!
//! One further narrowing, and it is the same argument read once more. A
//! declaration can accept a shape and declare no placement for it, which means
//! the producer is called with none of that term written into its arguments. The
//! *binding count* is therefore taken over the terms the declaration can
//! actually **receive** — accepted, and placed somewhere — because a binding is
//! the claim that the producer got the term, and no honest plan can make that
//! claim for a term whose content reaches no argument position. Presence still
//! quantifies over the accepted set: a mandatory producer that accepts something
//! of the request must be in the plan, even if what it accepts it does not read.
//!
//! The received set bounds bindings in the other direction too, and there the
//! rule is not about `mandatory` at all. *No* binding, on any producer, may name
//! a term its declaration cannot receive: the emitted call would carry no trace
//! of that term while
//! [`Plan::unserved_evidence`](crate::Plan::unserved_evidence) — which reads
//! "served" off binding membership — reported it answered. That is a plan
//! claiming a service its own query text does not perform, and it is
//! [`HollowBinding`](AdmissionError::HollowBinding). Emission cannot catch it:
//! an accepted alternative with no placements gives `place` nothing to do, so
//! `place` succeeds. The planner applies exactly this rule when it binds — it
//! binds the carried set and nothing wider — and the waist re-derives it by
//! calling the very same matching function, because the plan between them is a
//! value a caller can edit.
//!
//! Declaring a producer mandatory is therefore a claim with teeth, and it bites
//! in two distinguishable ways:
//!
//! * the producer accepts at least one of the request's terms and is **not in the
//!   plan at all** — never planned, refused at placement, or edited out — which
//!   is [`MissingMandatoryProducer`](AdmissionError::MissingMandatoryProducer);
//! * the producer is present but bound to only *some* of the terms it accepts,
//!   which is [`InsufficientBindings`](AdmissionError::InsufficientBindings),
//!   naming how many of its accepted terms were required and how many arrived.
//!
//! Both are refusals of a registry misconfiguration: the host declared that this
//! producer serves what it can serve of every request, and this plan takes part
//! of that away. They are loud, typed and name the producer, which is the only
//! useful thing to do with a coverage promise that the plan has just falsified. A
//! producer the registry does **not** declare mandatory and that placement
//! refuses is simply dropped from the plan, with its reason recorded in the
//! plan's decisions.
//!
//! # There is no stratum-weight dimension here
//!
//! There used to be, and it checked a number the plan invented. A plan records
//! no weights: §8 of the design record keeps the fusion profile out of planning,
//! so the weights that fuse an answer are the profile's and there is no second
//! set for this waist to validate. Both halves of what the old dimension asked
//! are answered where the fusing weights actually live, and neither is
//! duplicated here:
//!
//! * *is this weight usable?* — [`FusionProfile::with_decay`] refuses a non-positive
//!   weight ([`FusionError::NonPositiveWeight`](crate::FusionError::NonPositiveWeight)),
//!   an empty weight map, and a weight vector whose admitted ceiling leaves the
//!   fixed-point range. A profile that exists is a profile whose every weight is
//!   already valid, so no plan can present an invalid one.
//! * *does this weight name a stratum that ranks?* — the direction that can cost
//!   a caller rows is a stratum the plan **reaches** that the profile does not
//!   weight, and [`search`](crate::search) reports exactly that, by name, in
//!   [`SearchResult::unweighted_strata`](crate::SearchResult::unweighted_strata),
//!   refusing outright only when the two are wholly disjoint. The mirror — a
//!   profile weighting a stratum no producer emits under — costs nothing and
//!   hides nothing: no stream ever carries that stratum, so the weight is simply
//!   never consulted, and fusion still refuses a *stream* whose stratum has no
//!   weight ([`FusionError::UnknownStratum`](crate::FusionError::UnknownStratum)).
//!   A profile is a reusable law, deliberately chosen without reference to any
//!   one plan; refusing it for naming a stratum this request did not reach would
//!   refuse the reuse it exists for.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use purrdf_core::binding_pattern::BindingPattern;
use purrdf_sparql_eval::{PfDescriptor, PropertyFunctionRegistry, RegistryId};

use crate::fusion_profile::FusionProfile;
use crate::id::PLAN_VERSION;
use crate::iri::Iri;
use crate::matching::{Invocation, PlacementError, carries_content, pattern_matches, place};
use crate::plan::{Plan, PlanOrigin, ProducerBinding};
use crate::statistics::Statistics;

/// Everything `compile` needs from outside the plan: the live registry, the
/// statistics provider the replay is admitted against, and — when the caller has
/// chosen one — the fusion law the answer will be composed under.
///
/// The registry is borrowed, not cloned: admission reads its declared
/// capabilities through [`PropertyFunctionRegistry::describe`] and never opens a
/// relation. The statistics provider is a `dyn` borrow so a caller can pass a
/// heterogeneous provider without the environment becoming generic, and admission
/// consults only its declared source and revision — the query methods plan-time
/// cardinality, not admission.
pub struct AdmissionEnvironment<'a> {
    /// The live registry the compiled units resolve against.
    pub registry: &'a PropertyFunctionRegistry,
    /// The statistics provider whose revision the plan must have recorded.
    pub statistics: &'a dyn Statistics,
    /// The fusion profile the compiled units' rows will be fused under, when
    /// the caller has chosen one.
    ///
    /// It is an `Option` because the design record is explicit that the profile
    /// is **not** a planning input: a caller legitimately plans and compiles
    /// without yet knowing how the result will be fused, and demanding a law it
    /// has not chosen would refuse that.
    ///
    /// An environment that names one *learns* more, not is held to more. Each
    /// stratum's planned depth is measured against the rank resolution that law
    /// actually delivers, and the two numbers are recorded on the compiled value
    /// as [`PlannedResolution`](crate::PlannedResolution) — before anything is
    /// executed, so the cost of a depth is knowable without paying for it. It is
    /// evidence and not a refusal: a plan that reads past its profile's
    /// separating range still yields a correct, deterministic answer, at a
    /// coarser resolution it can now see. [`search`](crate::search) always names
    /// the profile it is about to fuse under, so the whole-pipeline path always
    /// carries this.
    pub fusion_profile: Option<&'a FusionProfile>,
}

impl core::fmt::Debug for AdmissionEnvironment<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("AdmissionEnvironment")
            .field("registry", self.registry)
            .field("statistics_source", &self.statistics.source())
            .field("statistics_revision", &self.statistics.revision())
            .field(
                "fusion_profile",
                &self.fusion_profile.map(FusionProfile::id),
            )
            .finish()
    }
}

/// A semantic refusal to admit a plan.
///
/// Every variant names the exact violated dimension — [`AdmissionError::dimension`]
/// returns a stable spelling for that dimension — so a caller can tell a deleted
/// mandatory producer from a stale statistics snapshot rather than receiving one
/// generic "invalid plan".
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum AdmissionError {
    /// A producer the registry declares mandatory, and whose declaration accepts
    /// at least one of the request's terms, is absent from the plan entirely —
    /// never planned, refused at placement, or edited out.
    ///
    /// A mandatory producer whose declaration accepts *none* of the request's
    /// terms is not reported here: it was never this request's to serve. See
    /// this module's header on what mandatory quantifies over.
    ///
    /// The producer IRI is boxed because [`Iri`] carries five parsed spans and is
    /// large; boxing keeps `Result<_, AdmissionError>` cheap to move, the same
    /// discipline [`PlanError`](crate::PlanError) applies to its boxed terms.
    #[error("the registry's mandatory producer {producer} is absent from the plan")]
    MissingMandatoryProducer {
        /// The registered producer IRI that must be present.
        producer: Box<Iri>,
    },

    /// A producer the registry declares mandatory is present but was not bound
    /// to every request term **its own declaration can receive**.
    ///
    /// The two counts are both about that received set, never about the request
    /// as a whole: `required` is how many of the plan's request terms the
    /// producer's declared patterns accept *and place into an argument
    /// position*, and `provided` is how many of those the plan actually binds it
    /// to. A term the producer cannot accept, or accepts without declaring
    /// anywhere to put it, appears in neither number — the first was never this
    /// producer's to serve and the second reaches none of its arguments. See
    /// this module's header for why the wider readings refused plans that were
    /// always legitimate.
    #[error(
        "the registry's mandatory producer {producer} must receive every request term its \
         declaration places but is bound to {provided} of {required}"
    )]
    InsufficientBindings {
        /// The registered producer IRI that was under-bound.
        producer: Box<Iri>,
        /// How many of the plan's request terms the producer's declaration
        /// accepts and places, and which it must therefore receive.
        required: usize,
        /// How many of those it was actually bound to.
        provided: usize,
    },

    /// A binding names a request term the producer's declaration cannot
    /// **receive**, so the call it compiles to would carry no trace of that term
    /// while the plan reads back as having served it.
    ///
    /// A producer receives a term when its first matching accepted alternative
    /// declares at least one placement for it. A term it accepts and places
    /// nowhere, and a term it does not accept at all, are both unreceivable: in
    /// either case the emitted text mentions nothing of the term, yet
    /// [`Plan::unserved_evidence`](crate::Plan::unserved_evidence) derives
    /// "served" from binding membership and would report it answered. The
    /// planner never writes such an index — it binds the received set — so a
    /// plan carrying one was hand-edited or deserialized from one that was.
    ///
    /// This is distinct from [`Self::UnsatisfiablePlacement`], which is about a
    /// placement the producer *does* declare and cannot perform. This one is
    /// about the absence of a placement, which placement itself cannot see: an
    /// alternative with an empty placement list gives the renderer nothing to do
    /// and therefore succeeds.
    ///
    /// It is also distinct from [`Self::InsufficientBindings`], and in the
    /// opposite direction: that one refuses a binding set too *narrow* for what
    /// the registry declared mandatory, this one refuses a binding set too
    /// *wide* for what the producer can take, on every producer, mandatory or
    /// not.
    #[error(
        "producer {producer} is bound to request term {request_term}, which its declaration \
         cannot receive: nothing of that term is placed into an argument, so the call would \
         carry none of it"
    )]
    HollowBinding {
        /// The producer whose binding names a term it cannot receive.
        producer: Box<Iri>,
        /// The index, into the plan's own request terms, of the term it cannot
        /// receive.
        request_term: u32,
    },

    /// A stratum's recorded depth is zero, so the plan asks for a stratum that
    /// reads nothing.
    ///
    /// The planner cannot write this. It records a depth only for a stratum a
    /// surviving producer ranks under, and the depth it derives is floored at
    /// one: an estimate narrows a read and never eliminates one, because
    /// emptiness is the **producer's** to report through a receipt fusion
    /// verifies against the rows it pulled. A zero here is therefore a plan that
    /// was hand-built or edited, and admitting it would compile a `LIMIT 0`,
    /// take no row from the relation whatever its index holds, and then report
    /// the stratum exhausted with no rows — the one ending that names no stopper,
    /// made about a read that was never allowed to answer.
    ///
    /// Distinct from [`Self::DepthBoundViolation`], and in the opposite
    /// direction: that one refuses a depth *above* what the registry declared,
    /// this one refuses a depth below what any real read can be. A stratum a
    /// caller wants left out of the answer is left out by dropping its entry —
    /// absence is how a plan says a stratum does not run, and it is a fact
    /// [`Plan::unserved_evidence`](crate::Plan::unserved_evidence) can still read
    /// back — not by bounding it at nothing.
    ///
    /// The stratum IRI is boxed for the reason
    /// [`Self::MissingMandatoryProducer`] boxes its producer: [`Iri`] carries
    /// five parsed spans, and a large error variant is paid for on every
    /// `Result` this module returns.
    #[error("stratum {stratum} declares a depth of zero, which reads nothing and proves nothing")]
    ZeroDepth {
        /// The stratum whose depth was recorded as zero.
        stratum: Box<Iri>,
    },

    /// A stratum's recorded depth is so deep that the emitted bound cannot carry
    /// the probe row one past it, so how the read ended could not be observed.
    ///
    /// This is [`Self::ZeroDepth`]'s mirror at the other end of the range, and the
    /// two refuse the same thing: a completeness claim made about the bound rather
    /// than about the data. A unit is emitted one row deeper than its depth, and
    /// that row is the whole of how [`execute`](crate::execute) tells a read the
    /// depth cut ([`DepthReached`](crate::ProducerStatus::DepthReached)) from a
    /// read that ran out ([`Exhausted`](crate::ProducerStatus::Exhausted)). At
    /// `u32::MAX` the `LIMIT` the compiler writes cannot express that row, so the
    /// emitted bound would equal the depth, no probe could ever arrive, and every
    /// such read — however many rows the relation still held — would be reported
    /// exhausted. Saturating the bound there is exactly the fault the probe exists
    /// to close, reappearing at the one depth where the mitigation is dropped.
    ///
    /// The planner cannot write this: it records a derived bound past the ceiling
    /// *at* the ceiling, so a depth here was hand-built or edited. `ceiling` is the deepest depth that
    /// **can** be read — one shallower than the deepest a plan can express,
    /// because the read goes one row deeper than the depth — and a plan wanting
    /// more rows than that from one stratum is past what this layer's rank
    /// encoding can carry, not past a policy.
    ///
    /// The stratum IRI is boxed for the reason [`Self::ZeroDepth`] boxes its own.
    #[error(
        "stratum {stratum} declares depth {depth}, which leaves no room for the probe row: a read \
         is emitted one row deeper than its depth, so {ceiling} is the deepest depth whose ending \
         can be observed and anything past it would be reported exhausted without being read to \
         its end"
    )]
    DepthWithoutProbe {
        /// The stratum whose depth cannot be probed.
        stratum: Box<Iri>,
        /// The depth the plan recorded.
        depth: u32,
        /// The deepest depth whose emitted bound can still carry a probe row.
        ceiling: u32,
    },

    /// A stratum's recorded depth exceeds the registry's declared row bound.
    ///
    /// The bound is read at one declared access mode, and `mode` says which — the
    /// invoked one, or the coarser one that bounds this read tighter than the invoked
    /// one declared. A producer declaring several modes is the case the layer exists to
    /// serve, and for that producer the refused figure is not findable from the count
    /// alone.
    #[error(
        "stratum {stratum} declares depth {requested}, but the registry bounds it at {declared}{mode}"
    )]
    DepthBoundViolation {
        /// The stratum whose depth was raised.
        stratum: Box<Iri>,
        /// The registry's declared row bound for the stratum.
        declared: u32,
        /// The depth the plan requested.
        requested: u32,
        /// Which declared mode `declared` was read at, relative to the invocation.
        mode: BoundMode,
    },

    /// The plan was planned against a different statistics revision than the one
    /// the environment now reports.
    #[error(
        "plan was planned against statistics revision {plan_revision:?}, but the environment reports {current_revision:?}"
    )]
    StaleStatistics {
        /// The revision the plan recorded.
        plan_revision: String,
        /// The revision the environment reports now.
        current_revision: String,
    },

    /// The plan was planned against a different live registry instance.
    ///
    /// This is distinct from [`Self::RegistryFingerprintMismatch`]: the declared
    /// contents may be byte-identical, yet the two registries can register the
    /// same IRI to different implementations that answer differently. The
    /// instance identity is the only value that can see that difference, so it is
    /// checked rather than inferred from the fingerprint.
    ///
    /// Raised only for a plan that records
    /// [`PlanOrigin::SameProcess`](crate::PlanOrigin::SameProcess). A
    /// deserialized plan's recorded counter names no live registry, so there is
    /// nothing here to compare and the durable content fingerprint carries the
    /// whole claim; see this module's header.
    #[error(
        "registry instance mismatch: plan was planned against {expected_instance:?}, environment holds {got_instance:?}"
    )]
    RegistryMismatch {
        /// The instance the plan recorded.
        expected_instance: RegistryId,
        /// The instance the environment holds.
        got_instance: RegistryId,
    },

    /// The plan's durable registry content fingerprint does not match the
    /// environment's, so the registry's declared shape has moved.
    #[error(
        "registry content fingerprint mismatch: plan recorded {expected:?}, environment declares {got:?}"
    )]
    RegistryFingerprintMismatch {
        /// The fingerprint the plan recorded.
        expected: String,
        /// The fingerprint the environment's registry declares.
        got: String,
    },

    /// The plan carries a layout version this build does not write.
    #[error("plan layout version {version} is not the version this build admits")]
    InvalidPlanVersion {
        /// The version the plan carries.
        version: u16,
    },

    /// A bound producer cannot be invoked for the request terms the plan gives
    /// it: some facet it declares a placement for cannot be rendered, two
    /// placements contend for one position, or no declared access pattern
    /// serves the resulting invocation.
    ///
    /// The planner already refuses such a producer, so a plan carrying one was
    /// hand-built or edited. It is refused here rather than emitted as a call
    /// that silently drops the facet — the same reason every other dimension is
    /// re-derived at the waist.
    #[error(
        "producer {producer} cannot be invoked for the plan's request terms ({rule}): {detail}"
    )]
    UnsatisfiablePlacement {
        /// The producer that cannot be invoked.
        producer: Box<Iri>,
        /// The stable name of the placement rule that refused.
        rule: &'static str,
        /// What that rule refused, in words.
        detail: String,
        /// The invocation's own access-pattern code, when one was derived.
        invocation: Option<String>,
        /// Every access pattern the registry declares for the producer.
        declared: Vec<String>,
    },

    /// The plan is internally inconsistent or a registry declaration could not
    /// be read — a structural defect rather than a policy refusal.
    #[error("plan is malformed: {reason}")]
    MalformedPlan {
        /// What is inconsistent.
        reason: String,
    },
}

impl AdmissionError {
    /// The name of the violated dimension.
    ///
    /// The spelling is part of the diagnostic contract: it is the label a caller
    /// switches on and a report quotes, so it is stable and lower-snake-case.
    #[must_use]
    pub const fn dimension(&self) -> &'static str {
        match self {
            Self::MissingMandatoryProducer { .. } => "missing_mandatory_producer",
            Self::InsufficientBindings { .. } => "insufficient_bindings",
            Self::HollowBinding { .. } => "hollow_binding",
            Self::ZeroDepth { .. } => "zero_depth",
            Self::DepthWithoutProbe { .. } => "depth_without_probe",
            Self::DepthBoundViolation { .. } => "depth_bound_violation",
            Self::StaleStatistics { .. } => "stale_statistics",
            Self::RegistryMismatch { .. } => "registry_instance_mismatch",
            Self::RegistryFingerprintMismatch { .. } => "registry_fingerprint_mismatch",
            Self::InvalidPlanVersion { .. } => "invalid_plan_version",
            Self::UnsatisfiablePlacement { .. } => "unsatisfiable_placement",
            Self::MalformedPlan { .. } => "malformed_plan",
        }
    }
}

/// The deepest depth a read can be taken to, which is one shallower than the
/// deepest depth a plan can express.
///
/// A plan records a per-stratum depth as a `u32`, so `u32::MAX` is the deepest
/// depth that is *expressible*. It is not the deepest that can be *read*: the
/// unit is emitted one row deeper than the depth, and that probe row is the whole
/// of how the executor tells a read the bound cut from a read that ran out. A
/// depth of `u32::MAX` would need a `LIMIT` of `u32::MAX + 1` to carry it, so the
/// probe would vanish and the read would be certified exhausted without ever
/// having been read to its end.
///
/// So this — and not `u32::MAX` — is the ceiling the planner derives depths
/// against and the waist admits them against. The distinction is the same one
/// [`MonotoneDepth`](crate::MonotoneDepth) draws between a measured range and an
/// expressible one: what fits in the field and what can be honestly read are two
/// numbers, and conflating them is how a bound becomes a completeness claim.
pub(crate) const MAX_READ_DEPTH: u32 = u32::MAX - 1;

/// A stratum depth that has passed the waist: at least one row, and shallow
/// enough that the emitted bound one row deeper than it still fits the `LIMIT`
/// the compiler writes.
///
/// It exists so the compiler cannot be handed a depth whose probe row does not
/// fit. `emitted_limit` used to add that row with a saturating `+ 1`, which at
/// `u32::MAX` silently emitted a bound *equal* to the depth — no probe could
/// arrive, and the read was reported `Exhausted` however many rows the relation
/// still held. The saturation was the only thing standing between an unprobeable
/// depth and a false completeness claim, and a saturating operator makes no
/// claim at all.
///
/// The field is private to this module and every constructor runs both checks,
/// so a value of this type *is* the proof that they were made.
/// [`ProbedDepth::probe`] can therefore add its row with plain arithmetic: there
/// is no case left for a saturation to hide.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ProbedDepth(u32);

/// Why a depth cannot carry the probe row that makes its ending observable.
///
/// Returned by [`ProbedDepth::checked`] so the two conditions are decided in one
/// place and *named* in two vocabularies: the waist reports them about a plan
/// ([`AdmissionError`]) and [`StratumUnit::new`](crate::StratumUnit::new) reports
/// them about a hand-built bundle ([`UnitError`](crate::UnitError)). Two
/// re-derivations of "can this depth be probed" would be two chances to disagree
/// about the one question every ending hangs on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Unprobeable {
    /// The depth is zero: the read proves nothing, so there is no ending to
    /// observe.
    Zero,
    /// The depth is past [`MAX_READ_DEPTH`], so the bound one row deeper than it
    /// is not a number the emitted `LIMIT` can hold.
    PastCeiling,
}

impl ProbedDepth {
    /// `depth` as a probeable depth, or the reason it is not one.
    ///
    /// The floor and the ceiling are checked here and nowhere else, because
    /// neither has anything to do with what a registry declared: a zero reads
    /// nothing whatever the registry says about that stratum, and a depth whose
    /// probe row is inexpressible cannot report its own ending whatever the
    /// registry says either. The registry's own declared row bound is a third,
    /// separate dimension, decided by the caller once it holds one of these.
    pub(crate) const fn checked(depth: u32) -> Result<Self, Unprobeable> {
        if depth == 0 {
            return Err(Unprobeable::Zero);
        }
        if depth > MAX_READ_DEPTH {
            return Err(Unprobeable::PastCeiling);
        }
        Ok(Self(depth))
    }

    /// Admit `depth` for `stratum`, naming [`Self::checked`]'s refusal as the
    /// plan's.
    ///
    /// Neither end is a value the planner can produce — it records a depth only
    /// for a stratum a surviving producer ranks under, floors what it derives at
    /// one and records a derived bound past the ceiling at the ceiling — so a depth
    /// refused here came from a plan that was hand-built or edited, whatever the
    /// registry says about that stratum. The refusals name that stratum because the
    /// value belongs to the plan in hand and a caller has to be able to find it.
    fn admit(depth: u32, stratum: &Iri) -> Result<Self, AdmissionError> {
        Self::checked(depth).map_err(|reason| match reason {
            Unprobeable::Zero => AdmissionError::ZeroDepth {
                stratum: Box::new(stratum.clone()),
            },
            Unprobeable::PastCeiling => AdmissionError::DepthWithoutProbe {
                stratum: Box::new(stratum.clone()),
                depth,
                ceiling: MAX_READ_DEPTH,
            },
        })
    }

    /// The depth itself: the number the plan recorded, and the number every field
    /// keyed to the read is keyed to.
    pub(crate) const fn get(self) -> u32 {
        self.0
    }

    /// One row past the depth — the probe slot.
    ///
    /// Exact, not saturating: [`Self::checked`] refused the one depth for which
    /// this addition would have had to saturate, so there is no value of this
    /// type it can overflow on.
    pub(crate) const fn probe(self) -> u32 {
        self.0 + 1
    }
}

/// The registry facts admission reads once and `compile` reuses to emit units.
pub(crate) struct AdmittedRegistry<'a> {
    /// Every registered relation's self-description, keyed by its IRI.
    pub(crate) descriptors: BTreeMap<String, PfDescriptor>,
    /// The one binding each stratum carries, keyed by stratum.
    ///
    /// Built by the same pass that proves a stratum carries at most one binding,
    /// so emission cannot reach a second binding admission did not see. `compile`
    /// emits one unit per stratum from this map rather than re-grouping the
    /// plan's binding list, for the reason `descriptors` is shared: two
    /// groupings of one list are two chances to disagree about it.
    pub(crate) stratum_bindings: BTreeMap<Iri, &'a ProducerBinding>,
    /// The placed invocation each bound stratum will be called under, keyed by
    /// stratum: which argument positions carry which constants, and the access mode
    /// that occupancy amounts to.
    ///
    /// Derived at the waist rather than at emission, and carried forward for the
    /// reason `descriptors` is. The mode decides which of a producer's declared row
    /// bounds answers for this read, so the waist has to hold it to judge a depth
    /// against that bound; emitting the call from a *second* placement would let the
    /// text be built under a mode the depth was never admitted for.
    pub(crate) stratum_invocations: BTreeMap<Iri, Invocation>,
    /// What the registry declared about how many rows each of this plan's strata can
    /// yield — read at the mode that stratum's binding will be invoked under — keyed
    /// by stratum, and the same map the depth-bound check above was decided against.
    ///
    /// Carried out of admission rather than re-derived at emission for the
    /// reason `descriptors` is: emission needs this number to decide whether a
    /// read can be probed one row past its planned depth, and a second
    /// derivation of "what did this registry declare" is a second chance to
    /// disagree with the one the depth was already admitted against. One entry per
    /// entry of [`Plan::stratum_depths`](crate::Plan::stratum_depths) the registry
    /// ranks a producer under, which is every stratum a unit is emitted for.
    pub(crate) stratum_row_bounds: BTreeMap<Iri, RowBound>,
    /// Every depth the plan recorded, as the admitted [`ProbedDepth`] it passed
    /// this waist as — one entry per entry of
    /// [`Plan::stratum_depths`](crate::Plan::stratum_depths), and the map the
    /// compiler emits its units from.
    ///
    /// The compiler reads the depths from here rather than from the plan, and that
    /// is the point of carrying them: a `u32` read straight off the plan is a
    /// number that may be zero or unprobeable, and the emitter would be trusting
    /// that some earlier pass looked. Read as [`ProbedDepth`] it cannot be either,
    /// because nothing outside this module can build one.
    pub(crate) stratum_depths: BTreeMap<Iri, ProbedDepth>,
    /// The environment registry's declared content fingerprint.
    pub(crate) fingerprint: String,
    /// The environment registry's live instance identity.
    pub(crate) instance_id: RegistryId,
}

/// The stratum a ranked descriptor emits under, if it is ranked.
///
/// A relation registered without a ranked declaration declares nothing, and
/// nothing is what admission reads back: it emits under no stratum.
fn ranked_stratum(descriptor: &PfDescriptor) -> Option<Iri> {
    descriptor
        .ranked
        .as_ref()
        .map(|declaration| Iri::from(declaration.stratum.clone()))
}

/// What a registry declared about how many rows a stratum can yield.
///
/// "No bound was declared" and "the declared bound is zero" are different facts
/// about a registry and must not share a representation. A declared zero is a
/// measurement of the producer's data — it admits the one floored row that lets
/// the producer report its own emptiness, and refuses every depth past it — while
/// a missing declaration is the registry saying nothing, which can refuse
/// nothing. Collapsing the two is how a producer that declares no access mode
/// ends up bounding its stratum at zero and failing every plan that records a
/// depth for it.
///
/// There is deliberately no widening operation over two of these. The registry
/// refuses a stratum a second producer declares
/// (`PropertyFunctionRegistry::register_ranked`), so a stratum's bound is one
/// producer's declaration and never a worst case taken across several — and a
/// merge that could only ever run against a registry the seam refuses to build
/// would be unreachable code claiming a policy nothing enforces.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RowBound {
    /// The stratum's one producer declared a finite worst-case row count, at the
    /// declared access mode `mode`.
    ///
    /// The mode travels with the number because the number is a function of it: a
    /// producer declaring three rows with its needle bound and a hundred with it free
    /// has made two promises, and a refusal that names the figure without the mode it
    /// was read at names a number the invocation never declared.
    Declared {
        /// The declared worst-case row count.
        rows: u64,
        /// The declared mode that count was read at, which is the invoked mode itself
        /// only where no coarser declared mode bounds the read tighter — see
        /// [`declared_row_bound`].
        mode: BindingPattern,
    },
    /// The stratum's one producer declared no worst-case row count at all, so
    /// the registry set no bound here and admission enforces none.
    Undeclared,
}

impl RowBound {
    /// The declared count, or `None` where nothing was declared.
    ///
    /// A projection of the variant and never a defaulting of it: the absence stays
    /// an absence all the way onto the compiled unit, because a missing declaration
    /// can refuse nothing and a zero is a measurement.
    pub(crate) const fn rows(self) -> Option<u64> {
        match self {
            Self::Declared { rows, .. } => Some(rows),
            Self::Undeclared => None,
        }
    }

    /// How this bound's number is attributed, for an invocation made under `invoked`
    /// (or no invocation at all).
    ///
    /// One spelling, called at both sites that report a bound a read broke — the
    /// waist's own [`AdmissionError::DepthBoundViolation`] and the executor's
    /// [`ExecutionError::RowBoundBreached`](crate::ExecutionError) — so the two cannot
    /// describe the same declaration differently.
    pub(crate) fn attributed(self, invoked: Option<BindingPattern>) -> BoundMode {
        match (self, invoked) {
            (Self::Undeclared, _) => BoundMode::Undeclared,
            (Self::Declared { mode, .. }, None) => BoundMode::Uninvoked { declared: mode },
            (Self::Declared { mode, .. }, Some(invoked)) if mode == invoked => {
                BoundMode::Invoked { mode }
            }
            (Self::Declared { mode, .. }, Some(invoked)) => BoundMode::Subsuming {
                declared: mode,
                invoked,
            },
        }
    }
}

/// Which declared access mode a row bound was read at, relative to the read it bounds.
///
/// Carried by the two refusals that name a declared row count —
/// [`AdmissionError::DepthBoundViolation`] and
/// [`ExecutionError::RowBoundBreached`](crate::ExecutionError) — because the count alone
/// is not actionable for the multi-mode producers the published contract encourages. A
/// producer that declares three rows with its needle bound and a hundred with it free
/// has two promises registered, and "at most three rows per invocation" sends its author
/// looking at whichever of the two they happen to think of first.
///
/// The [`Display`](fmt::Display) rendering is the phrase the messages embed, written
/// once here so both say it the same way. It is a *trailing* phrase and carries its own
/// leading space, which is what lets [`Self::Undeclared`] contribute nothing and leave
/// the sentence around it intact.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum BoundMode {
    /// Read at the very mode the invocation is made under, which is the ordinary case
    /// and the only one for a producer that declares a single mode.
    Invoked {
        /// The mode, which is both the declared one and the invoked one.
        mode: BindingPattern,
    },
    /// Read at a **coarser** declared mode that subsumes the invocation, because that
    /// mode declared fewer rows than the invoked one did.
    ///
    /// The coarser mode demands strictly fewer bindings, so its tuples cover the
    /// invoked mode's; its count therefore bounds this read too, and where it is the
    /// smaller of the two it is the promise the producer actually made.
    ///
    /// This is the ordinary arm for a producer that declares the all-free mode alone
    /// and serves every access pattern of its arity through it, which is the shape the
    /// reference relation has. It is *also* how an over-declaration reads: a producer
    /// that declares the invoked mode too, with a larger count, has promised more where
    /// it is bound than it promised where it is free, and the smaller number is the one
    /// it can keep.
    Subsuming {
        /// The declared mode the number was read at.
        declared: BindingPattern,
        /// The mode this read is invoked under, which `declared` serves.
        invoked: BindingPattern,
    },
    /// Read at the widest mode the producer declares, because the stratum binds no
    /// producer for an invocation to have a mode at all. Nothing is emitted for such a
    /// stratum, so no read is judged by this number — only a recorded depth is.
    Uninvoked {
        /// The declared mode the number was read at.
        declared: BindingPattern,
    },
    /// No declared mode stands behind the number: either no ranked producer declares
    /// anything under the stratum, or the number arrived on a unit a caller assembled
    /// itself ([`StratumUnit::new`](crate::StratumUnit::new)), which records a count
    /// and no mode.
    Undeclared,
}

impl fmt::Display for BoundMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invoked { mode } => write!(f, " under mode `{}`", mode.code()),
            Self::Subsuming { declared, invoked } => write!(
                f,
                " under mode `{}`, which serves this call under mode `{}`",
                declared.code(),
                invoked.code()
            ),
            Self::Uninvoked { declared } => write!(
                f,
                " under mode `{}`, the widest mode declared, this stratum binding no \
                 producer to invoke",
                declared.code()
            ),
            Self::Undeclared => Ok(()),
        }
    }
}

/// What the registry declared about how many rows one read of a stratum can return,
/// read at the access mode that read will actually be made under.
///
/// One function, and the only reading of `rows_per_invocation` anywhere in this layer:
/// the planner derives a depth from it, this waist holds a recorded depth to it, the
/// compiler caps a self-bounding producer's depth argument at it, and the executor
/// refuses a producer that beats it. Those four decisions have to be the same number,
/// and they were not — the planner and the waist each evaluated the declaration at
/// **every** declared mode and took the maximum, a figure no invocation is ever made
/// under.
///
/// # Why the maximum was wrong, and in which direction
///
/// [`PropertyFunction::rows_per_invocation`](purrdf_sparql_eval::PropertyFunction::rows_per_invocation)
/// is a function *of the mode* by contract, and an index-backed producer declares many
/// modes precisely because its indices serve binding directions a scan cannot. A
/// producer declaring `3` rows where its needle is bound and `100` where it is free was
/// planned, admitted and judged at `100`: a depth of nine was admitted for a mode that
/// declared three, a producer emitting twelve rows against a declaration of three was
/// served rather than refused because a *second, larger* mode existed, and the
/// `min(depth + 1, declared)` that exists to keep the layer from asking a producer to
/// contradict its own registration took its `min` against a mode nobody invoked — so a
/// producer guarding its declared bound refused the call and lost its whole stratum.
/// Every one of those is the maximum over-declaring, which is the direction that admits
/// what cannot be served rather than the direction that refuses what can.
///
/// # Which promise answers for an invocation the table does not name
///
/// The `modes` table pairs each mode the producer **declared** with the bound it
/// declared there, and an invocation's own mode need not be one of those: the reference
/// relation declares the all-free mode alone and serves every access pattern of its
/// arity through it. The modes that answer for an invocation are exactly the declared
/// ones that *serve* it — `declared.subsumes(invoked)`, the same lattice rule
/// [`place`](crate::matching::place) admits the invocation by and the evaluator resolves
/// the call with — and a producer serving an invocation through one of them emits at
/// most that mode's declared rows, because the extra bindings only ever filter. Each
/// such mode is therefore a valid bound and the **tightest** of them is the promise the
/// producer actually made about this read, so that is the one taken. The answer is the
/// invoked mode's own number only where no coarser declared mode bounds it tighter —
/// the invoked mode subsumes itself, so it is one of the candidates, but it is not
/// privileged among them.
///
/// A coarser declared mode promising *less* than a finer one is the producer
/// **over-declaring the finer one**, and the `min` is the only sound reading of that
/// pair rather than a tie-break between two opinions. The coarser mode demands strictly
/// fewer bindings, so every tuple the finer mode can emit is one the coarser mode can
/// emit too — the extra bindings only filter — and a promise of three rows with the
/// needle free cannot be kept beside a hundred with it bound. So `fb = 100, ff = 3`
/// bounds an `fb` invocation at three, and a producer that then returns four is refused
/// against a number it did register: the refusal names the mode the three was read at
/// ([`BoundMode`]) precisely because that mode is not the one the call was made under.
///
/// `invoked` is `None` for a stratum the plan binds no producer to. There is no
/// invocation there and so no mode to read a promise at, and the widest bound the
/// producer declares under any mode is the most such a depth could ever describe —
/// which is the only number that can refuse nothing the registry did not speak against.
/// Nothing is emitted for such a stratum, so no read is judged by it.
///
/// Ties are broken by the mode rather than by declaration order, so the mode this
/// reports is a function of what the registry declares and not of the order it declared
/// it in: among modes promising the same count the invoked one wins where it is present,
/// then the lowest [`BindingPattern`] in its own total order. The number is the same
/// either way; the tie-break decides only which mode the diagnostic names.
pub(crate) fn declared_row_bound(
    descriptor: &PfDescriptor,
    invoked: Option<BindingPattern>,
) -> RowBound {
    let declared = descriptor.modes.iter().map(|declared| {
        (
            declared.rows_per_invocation,
            BindingPattern::from_code(&declared.code),
        )
    });
    match invoked {
        Some(invoked) => declared
            .filter(|(_, mode)| mode.subsumes(invoked))
            .min_by_key(|&(rows, mode)| (rows, mode != invoked, mode)),
        None => declared.max_by_key(|&(rows, mode)| (rows, mode)),
    }
    .map_or(RowBound::Undeclared, |(rows, mode)| RowBound::Declared {
        rows,
        mode,
    })
}

/// Which of the plan's request terms a producer's declaration accepts, as
/// indices into [`Plan::request_terms`], ascending.
///
/// This is the set a mandatory producer is held to. The rule is
/// [`pattern_matches`] — the planner's own matching function, called here rather
/// than restated — because admission re-derives the planner's decision and two
/// spellings of "does this pattern accept this term" would be a divergence no
/// test on either side could see.
///
/// A descriptor that is absent, or present with no ranked declaration, accepts
/// nothing: a relation that declares nothing is read back as nothing, and the
/// binding loop further down refuses a plan that binds such a producer anyway.
fn accepted_request_terms(plan: &Plan, descriptor: Option<&PfDescriptor>) -> Vec<u32> {
    let Some(declaration) = descriptor.and_then(|descriptor| descriptor.ranked.as_ref()) else {
        return Vec::new();
    };
    plan.request_terms
        .iter()
        .enumerate()
        .filter(|(_, term)| {
            declaration
                .accepted_terms
                .iter()
                .any(|accepted| pattern_matches(&accepted.pattern, term))
        })
        .map(|(index, _)| u32::try_from(index).unwrap_or(u32::MAX))
        .collect()
}

/// Which of the plan's request terms a producer's declaration can actually
/// *receive*: accepted, and placed somewhere by the alternative that accepted
/// them.
///
/// This is the set a mandatory producer's binding count is held to, and it is
/// narrower than [`accepted_request_terms`] by exactly the terms whose matching
/// alternative declares no placement. Holding a producer to those would demand a
/// binding no planner can honestly write: the term's content reaches no argument
/// position, so binding it would claim a service the emitted text does not
/// perform. A promise cannot extend past what the thing promising can do, which
/// is the same reasoning that made the promise quantify over accepted shapes
/// rather than over the whole request.
///
/// The rule is [`carries_content`] — the planner's own, called here rather than
/// restated.
fn carried_request_terms(plan: &Plan, descriptor: Option<&PfDescriptor>) -> Vec<u32> {
    let Some(declaration) = descriptor.and_then(|descriptor| descriptor.ranked.as_ref()) else {
        return Vec::new();
    };
    plan.request_terms
        .iter()
        .enumerate()
        .filter(|(_, term)| carries_content(declaration, term))
        .map(|(index, _)| u32::try_from(index).unwrap_or(u32::MAX))
        .collect()
}

/// A producer that cannot be invoked for the terms the plan gives it.
///
/// Raised where the placement runs, which is this waist: the mode an invocation has is
/// what decides the row bound its depth is judged against, so a binding that cannot be
/// placed at all is refused before any depth is compared to a number derived from it.
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

/// Parse a registry IRI, mapping a refusal to a malformed-plan error.
///
/// A registry predicate IRI was validated when registered, so this cannot fail in
/// practice; carrying the failure as a `MalformedPlan` rather than panicking keeps
/// a hostile registry declaration from aborting the caller.
fn registry_iri(text: &str) -> Result<Box<Iri>, AdmissionError> {
    Iri::parse(text)
        .map(Box::new)
        .map_err(|error| AdmissionError::MalformedPlan {
            reason: format!("registry declares invalid IRI {text:?}: {error}"),
        })
}

/// Read the registry's declarations, refusing a registry that cannot describe
/// itself.
fn describe(registry: &PropertyFunctionRegistry) -> Result<Vec<PfDescriptor>, AdmissionError> {
    registry
        .describe()
        .map_err(|error| AdmissionError::MalformedPlan {
            reason: format!("registry declarations could not be read: {error}"),
        })
}

/// Admit `plan` against `env`, returning the registry view `compile` emits from.
///
/// Performs every semantic check in the order a caller sees the dimensions:
/// version, registry content fingerprint, live instance identity, statistics
/// revision, mandatory producers and their bindings, then per-stratum depth
/// bounds. The first violated dimension is returned; admission does not
/// accumulate refusals.
pub(crate) fn admit_plan<'a>(
    plan: &'a Plan,
    env: &AdmissionEnvironment<'_>,
) -> Result<AdmittedRegistry<'a>, AdmissionError> {
    // 1. The plan's layout version must be one this build writes. A plan decoded
    //    through serde bypasses `from_canonical_bytes`'s own gate, so it is
    //    re-checked here rather than assumed.
    if plan.version != PLAN_VERSION {
        return Err(AdmissionError::InvalidPlanVersion {
            version: plan.version,
        });
    }

    // 2. The durable content fingerprint is checked first so a registry whose
    //    declared shape moved reports that dimension even when its live instance
    //    also differs.
    let fingerprint =
        env.registry
            .content_fingerprint()
            .map_err(|error| AdmissionError::MalformedPlan {
                reason: format!("registry content fingerprint could not be read: {error}"),
            })?;
    if plan.registry_content_fingerprint != fingerprint {
        return Err(AdmissionError::RegistryFingerprintMismatch {
            expected: plan.registry_content_fingerprint.clone(),
            got: fingerprint,
        });
    }

    // 3. The live instance identity, for the plans that carry a meaningful one.
    //    Two registries that declare byte-identical contents can still answer
    //    differently, and only this value can see that — but only within the
    //    process that minted it. A plan that crossed a process boundary records
    //    a counter this process never minted, so it is held to the durable
    //    fingerprint step 2 already matched and nothing weaker.
    let instance_id = env.registry.instance_id();
    match plan.origin {
        PlanOrigin::SameProcess => {
            if plan.registry_instance_id != instance_id {
                return Err(AdmissionError::RegistryMismatch {
                    expected_instance: plan.registry_instance_id,
                    got_instance: instance_id,
                });
            }
        }
        PlanOrigin::Deserialized => {}
    }

    // 4. Statistics staleness: the plan records the revision it was planned
    //    against, and a replay against a moved revision is a detectable refusal
    //    rather than a silent replan.
    let current_revision = env.statistics.revision();
    if plan.statistics_snapshot.revision != current_revision {
        return Err(AdmissionError::StaleStatistics {
            plan_revision: plan.statistics_snapshot.revision.clone(),
            current_revision: current_revision.to_owned(),
        });
    }

    // 5. Read the registry's declarations once. Admission and emission share this
    //    single read so they cannot disagree about what the registry declares.
    let described = describe(env.registry)?;
    let mut descriptors: BTreeMap<String, PfDescriptor> = BTreeMap::new();
    let mut strata: BTreeMap<Iri, String> = BTreeMap::new();
    let mut mandatory: Vec<(String, Iri)> = Vec::new();
    for descriptor in described {
        if let Some(stratum) = ranked_stratum(&descriptor) {
            // One entry, never a merge: the registry refuses a stratum a second
            // producer declares, so this key is fresh every time. What is recorded is
            // the producer, not a row bound: the bound depends on the mode the plan's
            // own binding will be invoked under, which is not known until placement has
            // run further down.
            strata.insert(stratum.clone(), descriptor.iri.clone());
            // Read, not derived: the host's own `mandatory` flag and nothing
            // else. A producer whose patterns happen to accept everything is
            // still droppable unless the registry said otherwise.
            if descriptor
                .ranked
                .as_ref()
                .is_some_and(|declaration| declaration.mandatory)
            {
                mandatory.push((descriptor.iri.clone(), stratum));
            }
        }
        descriptors.insert(descriptor.iri.clone(), descriptor);
    }

    // 6. Producers the registry declares mandatory are present, agree on their
    //    stratum, and receive every request term **their own declaration
    //    accepts**. A shortfall here is a registry misconfiguration made
    //    visible, never a silently narrowed answer.
    for (producer, stratum) in &mandatory {
        // The accepted set is derived through `matching::pattern_matches` — the
        // planner's own rule, not a second copy of it — so "the terms it
        // accepts" means here exactly what it meant when the plan was built.
        let accepted = accepted_request_terms(plan, descriptors.get(producer));
        if accepted.is_empty() {
            // Nothing in this request is this producer's business, so there is
            // no term to drop it from and no coverage claim to falsify. The
            // quantifier is the same one the shortfall check below uses: a
            // producer answers for the shapes it accepts, and a request it
            // accepts nothing of is not a request it was promised to serve.
            continue;
        }
        let producer_iri = registry_iri(producer)?;
        let binding = plan
            .producer_bindings
            .iter()
            .find(|binding| &binding.producer == producer);
        let Some(binding) = binding else {
            return Err(AdmissionError::MissingMandatoryProducer {
                producer: producer_iri,
            });
        };
        if &binding.stratum != stratum {
            return Err(AdmissionError::MalformedPlan {
                reason: format!(
                    "producer {producer} is bound to stratum {}, but the registry ranks it under {stratum}",
                    binding.stratum
                ),
            });
        }
        // Held to the terms it can receive, not merely to the ones it matches:
        // an accepted shape with no placement reaches no argument position, so
        // there is no binding for the plan to be missing.
        let carried = carried_request_terms(plan, descriptors.get(producer));
        let required = carried.len();
        let provided = carried
            .iter()
            .filter(|index| binding.request_terms.contains(index))
            .count();
        if provided < required {
            return Err(AdmissionError::InsufficientBindings {
                producer: producer_iri,
                required,
                provided,
            });
        }
    }

    // Every recorded binding must name a ranked producer, must be bound to a
    // stratum the plan itself records a depth for, must be that stratum's ONLY
    // binding, and every request-term index it carries must address the plan's
    // own request.
    //
    // The depth entry is not bookkeeping. `compile` emits one unit per *depth*
    // entry and looks its bindings up from there, so a binding whose stratum has
    // no depth is never visited: the plan compiles to well-formed query text
    // that runs a producer fewer than the registry promised, with nothing
    // anywhere saying so. That is the silent narrowing this waist exists to
    // prevent, so the implication is enforced in both directions — the loop
    // below refuses a binding with no depth, and the depth loop further down
    // refuses a depth that exceeds what the registry declared.
    //
    // The single-binding rule is the plan-side face of the registry's own: the
    // seam refuses a stratum a second producer declares, and the reason —
    // a rank means nothing outside the list that assigned it, so two lists in
    // one stratum can only be concatenated — is a fact about ranks, not about
    // registration. A plan is editable, so it can carry two bindings under one
    // stratum where the registry carries one producer (the same producer twice,
    // with two term sets), and that is the identical concatenation with the
    // identical distortion. It is refused here rather than emitted.
    //
    // This loop is also where each binding is *placed* — the terms rendered into the
    // producer's argument positions, and the access mode the resulting call has. That
    // used to happen at emission, one stage later, and it moved here because the depth
    // check below is decided against the declaration read at exactly that mode: the
    // bound a depth is held to is a function of the mode, so the mode has to be derived
    // before the depth can be judged. It is derivable this early because
    // [`place`](crate::matching::place) needs no depth — see its own header. The
    // consequence a caller sees is the order of two dimensions: a plan that is both
    // unplaceable and too deep now reports the placement, which is the narrower fact and
    // the one the depth's own refusal would otherwise be derived from.
    let mut stratum_bindings: BTreeMap<Iri, &ProducerBinding> = BTreeMap::new();
    let mut stratum_invocations: BTreeMap<Iri, Invocation> = BTreeMap::new();
    for binding in &plan.producer_bindings {
        let Some(descriptor) = descriptors.get(&binding.producer) else {
            return Err(AdmissionError::MalformedPlan {
                reason: format!(
                    "plan binds producer {} that the registry does not declare",
                    binding.producer
                ),
            });
        };
        // Read once, and it answers both of the next two questions: which
        // stratum the registry ranks this producer under, and which of the
        // plan's request terms its placements can actually receive. A relation
        // with no ranked declaration emits under no stratum, so it can match no
        // stratum a plan names and falls into the same refusal.
        let Some(declaration) = descriptor
            .ranked
            .as_ref()
            .filter(|ranked| ranked.stratum.as_str() == binding.stratum.as_str())
        else {
            return Err(AdmissionError::MalformedPlan {
                reason: format!(
                    "producer {} is bound to stratum {}, which the registry does not declare for it",
                    binding.producer, binding.stratum
                ),
            });
        };
        if !plan.stratum_depths.contains_key(&binding.stratum) {
            return Err(AdmissionError::MalformedPlan {
                reason: format!(
                    "producer {} is bound to stratum {}, but the plan records no depth for it, so the producer would never be compiled",
                    binding.producer, binding.stratum
                ),
            });
        }
        for index in &binding.request_terms {
            let Some(term) = plan.request_terms.get(*index as usize) else {
                return Err(AdmissionError::MalformedPlan {
                    reason: format!(
                        "producer {} references request term {index}, but the plan carries {} term(s)",
                        binding.producer,
                        plan.request_terms.len()
                    ),
                });
            };
            // A binding is the claim that the producer received the term, so the
            // index must name a term the declaration can receive. The planner
            // binds exactly the received set and the waist asks the same
            // question of the same function, because the plan between them is
            // editable: an index the planner would never have written compiles
            // to a call that mentions nothing of the term, while
            // `Plan::unserved_evidence` reads the binding back as having served
            // it — the term is reported answered by query text that never names
            // it. Emission cannot catch this. `place` iterates the alternative's
            // placements, so an alternative with none gives it nothing to do and
            // it succeeds; the missing placement is visible only to the rule
            // that asks about placements directly.
            //
            // `carries_content` asked per index is membership in
            // `carried_request_terms`, without materializing the set on the
            // admitting path.
            if !carries_content(declaration, term) {
                return Err(AdmissionError::HollowBinding {
                    producer: registry_iri(&binding.producer)?,
                    request_term: *index,
                });
            }
        }
        // The plan is untrusted input, so what `place` decided when the planner called
        // it is decided again here, by that same function and never by a second
        // approximation: every facet the declaration places renders into a SPARQL
        // constant, no two placements contend for one argument position, and some
        // declared access pattern serves the resulting invocation. The mode it derives
        // is carried to emission rather than re-derived there, so the number the depth
        // was admitted against and the number the emitted call is capped at cannot be
        // two readings of one declaration.
        //
        // What `place` does NOT re-derive is that the bound terms are carried at all.
        // It iterates the matching alternative's placements, so an alternative declaring
        // none gives it nothing to do and it returns success on a binding that
        // transports nothing. That is the `carries_content` rule just above, which is
        // why it is asked separately.
        let invocation = place(
            &binding.producer,
            descriptor,
            declaration,
            &plan.request_terms,
            &binding.request_terms,
        )
        .map_err(|error| unsatisfiable(binding, &error))?;
        stratum_invocations.insert(binding.stratum.clone(), invocation);
        if let Some(held) = stratum_bindings.insert(binding.stratum.clone(), binding) {
            return Err(AdmissionError::MalformedPlan {
                reason: format!(
                    "stratum {} carries two bindings, {} and {}, but one stratum carries one \
                     producer: their ranks are assigned by different lists and can only be \
                     concatenated, which ranks the second's best row below every row of the \
                     first. Merge producers that share a scoring law into one producer, or give \
                     producers that score differently a stratum each",
                    binding.stratum, held.producer, binding.producer
                ),
            });
        }
    }

    // Every recorded unserved term must address the plan's own request, and no
    // two entries may name one term. The list is per-term evidence a caller
    // reads back as "nothing answered this", so an index that names no term of
    // this request, or two entries disagreeing about one term, is a defect in
    // the value rather than a policy question.
    //
    // What is deliberately NOT enforced here is the converse — that every term
    // the bindings leave unserved appears in the list. Narrowing a producer the
    // registry does not declare mandatory is a legitimate edit (see this
    // module's header), and it strands terms without touching the recorded list;
    // refusing that would refuse the edit the waist explicitly admits. The
    // stranded term is not lost either: `Plan::unserved_evidence` derives it
    // from the bindings in hand, so the answer reports it regardless of what the
    // list says.
    let mut seen_unserved: BTreeSet<u32> = BTreeSet::new();
    for entry in &plan.unserved_terms {
        if entry.request_term as usize >= plan.request_terms.len() {
            return Err(AdmissionError::MalformedPlan {
                reason: format!(
                    "the plan records request term {} as unserved, but it carries {} term(s)",
                    entry.request_term,
                    plan.request_terms.len()
                ),
            });
        }
        if !seen_unserved.insert(entry.request_term) {
            return Err(AdmissionError::MalformedPlan {
                reason: format!(
                    "the plan records request term {} as unserved more than once",
                    entry.request_term
                ),
            });
        }
        if plan
            .producer_bindings
            .iter()
            .any(|binding| binding.request_terms.contains(&entry.request_term))
        {
            return Err(AdmissionError::MalformedPlan {
                reason: format!(
                    "the plan records request term {} as unserved, but binds it to a producer",
                    entry.request_term
                ),
            });
        }
    }

    // 7. Per-stratum depth bounds, from three sides. A recorded depth may be
    //    lower than the registry's declared row bound (statistics narrow a read)
    //    and never higher; it may never be zero, because a read of nothing is not
    //    a read; and it may never be so deep that the emitted bound cannot carry
    //    the probe row that says how the read ended, because a read whose ending
    //    nobody could observe must not be reported as an exhaustion.
    //
    //    The first is the registry's dimension and is decided here. The other two
    //    are properties of the depth alone — a zero reads nothing whatever the
    //    registry declared, and an unprobeable depth cannot report its ending
    //    whatever the registry declared — so they are `ProbedDepth::admit`'s, and
    //    the type it returns is what the compiler emits from.
    //
    //    The registry's dimension is decided against the declaration read at the mode
    //    the plan's own binding will be invoked under, which is the mode placement
    //    derived above. Read at the widest mode instead — which is what taking the
    //    maximum over the declared modes did — a depth of nine was admitted for an
    //    invocation whose mode declared three, because a second, larger mode existed on
    //    the same producer.
    let mut admitted_depths: BTreeMap<Iri, ProbedDepth> = BTreeMap::new();
    let mut stratum_row_bounds: BTreeMap<Iri, RowBound> = BTreeMap::new();
    for (stratum, depth) in &plan.stratum_depths {
        let depth = ProbedDepth::admit(*depth, stratum)?;
        // Recorded before the registry's own dimension is decided, because the
        // `Undeclared` arm below leaves this loop without reaching its end and the
        // compiler emits one unit per entry of this map: a stratum missing from it
        // is a producer missing from the emitted text, which is the silent
        // narrowing the waist exists to prevent. Nothing is admitted early by
        // recording it — a refusal below returns the whole `Result`, map and all.
        admitted_depths.insert(stratum.clone(), depth);
        // The bound this stratum's read is judged by, read once here and carried to
        // emission on the compiled unit. `invoked` is the mode the plan's own binding
        // will be called under, or `None` for a stratum the plan binds nothing to —
        // which emits no unit and so has no read for a mode to be a property of.
        let invoked = stratum_invocations.get(stratum).map(|held| held.mode);
        let bound = strata.get(stratum).map(|producer| {
            descriptors
                .get(producer)
                .map_or(RowBound::Undeclared, |descriptor| {
                    declared_row_bound(descriptor, invoked)
                })
        });
        if let Some(bound) = bound {
            stratum_row_bounds.insert(stratum.clone(), bound);
        }
        let declared = match bound {
            // Declared nothing, so it bounds nothing: there is no number here
            // for a depth to exceed, and inventing zero would refuse a plan the
            // registry never spoke against. This is a producer that declared no
            // access mode, which is not a producer that declared zero rows —
            // "declared nothing" is not "declared zero", and the two are decided
            // in different arms of this match.
            Some(RowBound::Undeclared) => continue,
            // `u64::MAX` is the seam's spelling of "genuinely unbounded", and it
            // admits every `u32` depth by arithmetic rather than by a rule.
            //
            // The `max(1)` is the one place a bound is widened, and it widens
            // exactly one declaration: a producer whose every access mode
            // promises zero rows. That producer described its data — an index
            // built before its documents land declares it — and it is planned at
            // the depth the planner floors at one, because that single row is the
            // probe that lets the producer report its own emptiness with a
            // receipt that means it. Admitting only the floor and nothing beyond
            // it, a depth of two over a declared zero is still refused below.
            // For every `declared >= 1` this changes nothing at all.
            Some(RowBound::Declared { rows, .. }) => rows.max(1),
            // No ranked producer emits under this stratum at all, so nothing can
            // rank there and no positive depth is fillable. This zero is **not**
            // floored: there is no producer here to hand a probing row to, and a
            // floor would turn "nobody ranks in this stratum" into a licence to
            // read one row from nobody. Every depth reaching here is positive, so
            // this bound of zero is reported as a `DepthBoundViolation` — the
            // zero-depth case above already returned.
            None => 0,
        };
        if u64::from(depth.get()) > declared {
            return Err(AdmissionError::DepthBoundViolation {
                stratum: Box::new(stratum.clone()),
                declared: u32::try_from(declared).unwrap_or(u32::MAX),
                requested: depth.get(),
                // The mode the refused number was read at, which for a producer
                // declaring several is routinely not the invoked one: a coarser mode
                // that declares less bounds the read tighter, and that is the number
                // above. Named rather than left out, because a depth refused against a
                // figure declared somewhere the call is not made sends its author
                // looking at the wrong declaration.
                mode: bound.map_or(BoundMode::Undeclared, |bound| bound.attributed(invoked)),
            });
        }
    }

    Ok(AdmittedRegistry {
        descriptors,
        stratum_bindings,
        stratum_invocations,
        stratum_row_bounds,
        stratum_depths: admitted_depths,
        fingerprint,
        instance_id,
    })
}
