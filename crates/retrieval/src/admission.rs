// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

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
//!   the stratum the registry ranks it under, and receives every request term;
//! * every stratum depth respects the registry's declared row bound;
//! * every stratum weight keys a declared stratum and is strictly positive under
//!   §5 of the design record;
//! * every bound producer can actually be *invoked* for the request terms the
//!   plan gives it — every declared placement renders, no two contend for one
//!   position, and some declared access pattern serves the result;
//! * every producer the plan binds is bound to a stratum the plan records a
//!   depth for, so a compiler that emits one unit per depth entry cannot skip a
//!   bound producer;
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
//! Declaring a producer mandatory is therefore a claim with teeth, and it bites
//! in two distinguishable ways:
//!
//! * the producer is **not in the plan at all** — never planned, refused at
//!   placement, or edited out — which is
//!   [`MissingMandatoryProducer`](AdmissionError::MissingMandatoryProducer);
//! * the producer is present but the planner could bind it to only *some* of the
//!   request's terms, because its declared patterns accept only some of the
//!   request's shapes, which is
//!   [`InsufficientBindings`](AdmissionError::InsufficientBindings), naming how
//!   many terms were required and how many arrived.
//!
//! Both are refusals of a registry misconfiguration: the host declared that
//! every request must be served by a producer that cannot serve this one. They
//! are loud, typed and name the producer, which is the only useful thing to do
//! with a coverage promise that the request has just falsified. A producer the
//! registry does **not** declare mandatory and that placement refuses is simply
//! dropped from the plan, with its reason recorded in the plan's decisions.

use std::collections::{BTreeMap, BTreeSet};

use purrdf_sparql_eval::{PfDescriptor, PropertyFunctionRegistry, RegistryId};
use purrdf_text::Fixed;

use crate::id::PLAN_VERSION;
use crate::iri::Iri;
use crate::plan::{Plan, PlanOrigin};
use crate::statistics::Statistics;

/// Everything `compile` needs from outside the plan: the live registry and the
/// statistics provider the replay is admitted against.
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
}

impl core::fmt::Debug for AdmissionEnvironment<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("AdmissionEnvironment")
            .field("registry", self.registry)
            .field("statistics_source", &self.statistics.source())
            .field("statistics_revision", &self.statistics.revision())
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
    /// A producer the registry declares mandatory is absent from the plan
    /// entirely — never planned, refused at placement, or edited out.
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
    /// to every request term, because its declared patterns accept only some of
    /// the request's shapes.
    #[error(
        "the registry's mandatory producer {producer} must receive every request term but is bound to {provided} of {required}"
    )]
    InsufficientBindings {
        /// The registered producer IRI that was under-bound.
        producer: Box<Iri>,
        /// How many request terms the producer must receive.
        required: usize,
        /// How many of those it was actually bound to.
        provided: usize,
    },

    /// A stratum's recorded depth exceeds the registry's declared row bound.
    #[error(
        "stratum {stratum} declares depth {requested}, but the registry bounds it at {declared}"
    )]
    DepthBoundViolation {
        /// The stratum whose depth was raised.
        stratum: Box<Iri>,
        /// The registry's declared row bound for the stratum.
        declared: u32,
        /// The depth the plan requested.
        requested: u32,
    },

    /// A weight keys a stratum no registered producer emits under.
    #[error("stratum weight refers to undeclared stratum {stratum}")]
    UndeclaredStratumWeight {
        /// The undeclared stratum.
        stratum: Box<Iri>,
    },

    /// A stratum weight is not strictly positive, so it cannot participate in
    /// the §5 reciprocal-rank sum.
    #[error("stratum {stratum} carries invalid weight {weight:?}: {reason}")]
    InvalidWeight {
        /// The stratum whose weight was rejected.
        stratum: Box<Iri>,
        /// The rejected weight, exact.
        weight: Fixed,
        /// Why the weight cannot be admitted.
        reason: String,
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
            Self::DepthBoundViolation { .. } => "depth_bound_violation",
            Self::UndeclaredStratumWeight { .. } => "undeclared_stratum_weight",
            Self::InvalidWeight { .. } => "invalid_weight",
            Self::StaleStatistics { .. } => "stale_statistics",
            Self::RegistryMismatch { .. } => "registry_instance_mismatch",
            Self::RegistryFingerprintMismatch { .. } => "registry_fingerprint_mismatch",
            Self::InvalidPlanVersion { .. } => "invalid_plan_version",
            Self::UnsatisfiablePlacement { .. } => "unsatisfiable_placement",
            Self::MalformedPlan { .. } => "malformed_plan",
        }
    }
}

/// The registry facts admission reads once and `compile` reuses to emit units.
pub(crate) struct AdmittedRegistry {
    /// Every registered relation's self-description, keyed by its IRI.
    pub(crate) descriptors: BTreeMap<String, PfDescriptor>,
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
/// about a registry and must not share a representation: zero rows is a promise
/// that nothing ranks there, and it refuses every positive depth, while a
/// missing declaration is the registry saying nothing, which can refuse nothing.
/// Collapsing the two is how a producer that declares no access mode ends up
/// bounding its stratum at zero and failing every plan that records a depth for
/// it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RowBound {
    /// Every producer under the stratum that declared a worst case declared a
    /// finite one, and this is the largest of them.
    Declared(u64),
    /// No producer under the stratum declared a worst-case row count at all, so
    /// the registry set no bound here and admission enforces none.
    Undeclared,
}

impl RowBound {
    /// The worst case of two bounds under one stratum.
    ///
    /// [`Undeclared`](Self::Undeclared) is the identity, not the dominant value:
    /// a producer that declared nothing makes no claim, so it neither raises nor
    /// erases a claim another producer under the same stratum did make. That
    /// keeps the check exactly as strong as what the registry declared — the
    /// same "read, never inferred" rule this module applies to `mandatory`.
    const fn widen(self, other: Self) -> Self {
        match (self, other) {
            (Self::Declared(left), Self::Declared(right)) => {
                Self::Declared(if left > right { left } else { right })
            }
            (Self::Declared(bound), Self::Undeclared)
            | (Self::Undeclared, Self::Declared(bound)) => Self::Declared(bound),
            (Self::Undeclared, Self::Undeclared) => Self::Undeclared,
        }
    }
}

/// A descriptor's worst-case declared row count across its access modes, or
/// [`RowBound::Undeclared`] when it declares no access mode and therefore
/// declares no row count.
fn declared_row_bound(descriptor: &PfDescriptor) -> RowBound {
    descriptor
        .modes
        .iter()
        .map(|mode| mode.rows_per_invocation)
        .max()
        .map_or(RowBound::Undeclared, RowBound::Declared)
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
/// revision, mandatory producers and their bindings, per-stratum depth bounds,
/// then stratum weights. The first violated dimension is returned; admission does
/// not accumulate refusals.
pub(crate) fn admit_plan(
    plan: &Plan,
    env: &AdmissionEnvironment<'_>,
) -> Result<AdmittedRegistry, AdmissionError> {
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
    let mut strata: BTreeMap<Iri, RowBound> = BTreeMap::new();
    let mut mandatory: Vec<(String, Iri)> = Vec::new();
    for descriptor in described {
        if let Some(stratum) = ranked_stratum(&descriptor) {
            let bound = declared_row_bound(&descriptor);
            strata
                .entry(stratum.clone())
                .and_modify(|current| *current = current.widen(bound))
                .or_insert(bound);
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
    //    stratum, and receive every request term. A shortfall here is a registry
    //    misconfiguration made visible, never a silently narrowed answer.
    for (producer, stratum) in &mandatory {
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
        let required = plan.request_terms.len();
        let provided = (0..required)
            .filter(|index| {
                binding
                    .request_terms
                    .contains(&u32::try_from(*index).unwrap_or(u32::MAX))
            })
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
    // stratum the plan itself records a depth for, and every request-term index
    // it carries must address the plan's own request.
    //
    // The depth entry is not bookkeeping. `compile` emits one unit per *depth*
    // entry and looks its bindings up from there, so a binding whose stratum has
    // no depth is never visited: the plan compiles to well-formed query text
    // that runs a producer fewer than the registry promised, with nothing
    // anywhere saying so. That is the silent narrowing this waist exists to
    // prevent, so the implication is enforced in both directions — the loop
    // below refuses a binding with no depth, and the depth loop further down
    // refuses a depth that exceeds what the registry declared.
    for binding in &plan.producer_bindings {
        let Some(descriptor) = descriptors.get(&binding.producer) else {
            return Err(AdmissionError::MalformedPlan {
                reason: format!(
                    "plan binds producer {} that the registry does not declare",
                    binding.producer
                ),
            });
        };
        if ranked_stratum(descriptor).as_ref() != Some(&binding.stratum) {
            return Err(AdmissionError::MalformedPlan {
                reason: format!(
                    "producer {} is bound to stratum {}, which the registry does not declare for it",
                    binding.producer, binding.stratum
                ),
            });
        }
        if !plan.stratum_depths.contains_key(&binding.stratum) {
            return Err(AdmissionError::MalformedPlan {
                reason: format!(
                    "producer {} is bound to stratum {}, but the plan records no depth for it, so the producer would never be compiled",
                    binding.producer, binding.stratum
                ),
            });
        }
        for index in &binding.request_terms {
            if *index as usize >= plan.request_terms.len() {
                return Err(AdmissionError::MalformedPlan {
                    reason: format!(
                        "producer {} references request term {index}, but the plan carries {} term(s)",
                        binding.producer,
                        plan.request_terms.len()
                    ),
                });
            }
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

    // 7. Per-stratum depth bounds: the plan may lower a depth (statistics do not
    //    raise it), never raise it above the registry's declared row bound. A
    //    stratum no ranked producer emits under is bounded at zero — nothing
    //    ranks there, so no depth is fillable — but a stratum whose producers
    //    declared no row count at all is bounded by nothing, and is refused for
    //    nothing.
    for (stratum, depth) in &plan.stratum_depths {
        let declared = match strata.get(stratum) {
            // Declared nothing, so it bounds nothing: there is no number here
            // for a depth to exceed, and inventing zero would refuse a plan the
            // registry never spoke against.
            Some(RowBound::Undeclared) => continue,
            // `u64::MAX` is the seam's spelling of "genuinely unbounded", and it
            // admits every `u32` depth by arithmetic rather than by a rule.
            Some(&RowBound::Declared(bound)) => bound,
            // No ranked producer emits under this stratum at all, so nothing can
            // rank there and no positive depth is fillable.
            None => 0,
        };
        if u64::from(*depth) > declared {
            return Err(AdmissionError::DepthBoundViolation {
                stratum: Box::new(stratum.clone()),
                declared: u32::try_from(declared).unwrap_or(u32::MAX),
                requested: *depth,
            });
        }
    }

    // 8. Stratum weights must key a declared stratum and be strictly positive
    //    under §5; a non-positive weight cannot participate in the sum.
    for (stratum, weight) in &plan.stratum_weights {
        if !strata.contains_key(stratum) {
            return Err(AdmissionError::UndeclaredStratumWeight {
                stratum: Box::new(stratum.clone()),
            });
        }
        if !weight.is_positive() {
            return Err(AdmissionError::InvalidWeight {
                stratum: Box::new(stratum.clone()),
                weight: weight.fixed(),
                reason: "stratum weight must be strictly positive under §5".to_owned(),
            });
        }
    }

    Ok(AdmittedRegistry {
        descriptors,
        fingerprint,
        instance_id,
    })
}
