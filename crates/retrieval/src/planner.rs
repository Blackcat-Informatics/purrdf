// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The pure planner: one request, one registry, one statistics input, one plan.
//!
//! [`plan`] is a pure function. It opens no store, reads no file, consults no
//! clock and spawns nothing; every input it can see is a borrow, and the value
//! it returns is the whole of its output. That is what makes a plan
//! golden-testable as text in, text out, and reproducible across processes and
//! targets for the same request, registry and statistics.
//!
//! # Matching is a lookup, never inference
//!
//! A producer declares, as serializable data, which request-term shapes it
//! accepts — the registry's side table holds that declaration
//! ([`RankedDeclaration::accepted_terms`](purrdf_sparql_eval::RankedDeclaration::accepted_terms)),
//! supplied where the producer was registered. The planner
//! matches a request term against those declarations by a fixed lookup over
//! term kind, language and predicate — it never infers capability from a
//! producer's name, from a statistical signal, or from what other producers
//! accepted.
//!
//! A request term maps to the RDF term kind its retrieval targets:
//!
//! * a lexical term, a spatial term, and the endpoints of a temporal or numeric
//!   interval target a **literal**;
//! * an entity seed targets the kind its canonical lexical names (`<…>` an IRI,
//!   `_:…` a blank node, `"…"` a literal);
//! * a vector term targets **no** RDF term kind, so only a producer that
//!   declares [`TermKind::Any`] accepts it.
//!
//! A pattern's `datatype` constraint can never match, because no request term
//! carries a datatype; its `language` constraint matches only a lexical term
//! with that tag; its `predicate` constraint matches only a lexical, spatial,
//! temporal or numeric-range term carrying that predicate.
//!
//! # Matching is not enough: a producer must also be *invocable*
//!
//! Accepting a term's shape and being able to render that term into an argument
//! position are two different claims, and a producer can make the first without
//! the second: a query embedding has no SPARQL constant form, a blank-node seed
//! is a non-distinguished variable rather than a ground value, and a relation
//! that can only run with its depth bound cannot be called by one that leaves
//! the depth free. So after matching, the planner runs the compiler's own
//! [`place`](crate::matching::place) and records
//! [`RejectionReason::UnsatisfiedConstraint`] for a producer that cannot be
//! invoked, rather than binding it and emitting a call that silently drops the
//! facet it could not write. If that leaves nothing bound, the request reaches
//! nothing and [`PlanError::NoApplicableProducers`] is the answer.
//!
//! # A term that reaches nothing says so
//!
//! Both passes above decide per **producer**, and a caller asks per **term**. A
//! request can name a modality this registry has no producer for — the request
//! lattice deliberately carries shapes ahead of the producers that answer them —
//! and a plan that merely omitted such a term would be indistinguishable from
//! one that served it and found nothing. So every term no binding carries is
//! recorded in [`Plan::unserved_terms`](crate::Plan::unserved_terms) with a
//! typed [`UnservedReason`](crate::UnservedReason): nothing accepted its shape,
//! or something did and every acceptor was then rejected. The request is still
//! planned — the terms that *do* reach a producer are answered — because
//! refusing the whole request over one unserved term would discard every answer
//! the others can give.
//!
//! A producer the registry declares **mandatory** is not exempt. If placement
//! refuses one, the plan records the rejection and
//! [`compile`](crate::compile) refuses the plan with
//! [`MissingMandatoryProducer`](crate::AdmissionError::MissingMandatoryProducer):
//! the registry insists that producer serve every request, and this request
//! cannot be delivered to it. That is a visible, typed refusal at the admission
//! waist, which is the only place a coverage claim is enforced.
//!
//! # The registry is read, never duplicated
//!
//! Every declaration the planner consumes — a producer's IRI, its capability,
//! its access modes and their row bounds — is read from the registry's own
//! [`describe`](purrdf_sparql_eval::PropertyFunctionRegistry::describe)
//! machinery. The planner declares no parallel `volatility`, `arity` or
//! access-pattern fields of its own; the seam's declarations are the single
//! source, and [`Statistics`] supplies the cardinalities that bound them.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use purrdf_sparql_eval::{PfDescriptor, PropertyFunctionRegistry, RankedDeclaration};
use purrdf_text::Fixed;

use crate::error::PlanError;
use crate::iri::{Iri, Weight};
use crate::matching::{pattern_matches, place};
use crate::plan::{
    Plan, PlanOrigin, ProducerBinding, ProducerDecision, RejectionReason, StatisticsEntry,
    StatisticsSnapshot, UnservedReason, UnservedTerm,
};
use crate::request::{RequestTerm, RetrievalRequest};
use crate::statistics::Statistics;

/// The raw scale of the unit stratum weight a plan records (exactly one).
///
/// Planning happens before a fusion profile is chosen (the profile is
/// deliberately not a plan input), so the planner records the identity weight —
/// one — for every stratum it places. A caller that wants a different weighting
/// chooses it in the profile at fusion time; admission validates the plan's
/// weights against the declared strata rather than reading a policy into them.
const UNIT_WEIGHT_RAW: i128 = Fixed::ONE.into_raw();

/// Plan `request` against `registry`, consulting `statistics`.
///
/// The returned [`Plan`] records, as pure data: the request terms, one binding
/// per selected producer (the stratum and which request-term indices it
/// receives), every considered producer's decision — selected or rejected with
/// a reason — every request term that reached no producer at all with the reason
/// it did not, the per-stratum depth derived from the registry's row-bound
/// declarations capped by statistics, the unit weight map, the statistics
/// snapshot actually consulted, and both registry identities (the ephemeral
/// instance id and the durable content fingerprint).
///
/// # Errors
///
/// * [`PlanError::InvalidRequestTerm`] when a term is malformed (empty text or
///   geometry, an empty embedding, a non-finite embedding value, an empty seed).
/// * [`PlanError::RegistryDeclaration`] when a registered relation's declaration
///   panics under the seam's containment.
/// * [`PlanError::NoApplicableProducers`] when no registered producer accepts any
///   term of the request, or when every producer that does accept one cannot be
///   invoked for it.
/// * [`PlanError::StatisticsUnavailable`] when a selected producer declares an
///   unbounded row count and statistics supply no cardinality to bound it.
pub fn plan(
    request: &RetrievalRequest,
    registry: &PropertyFunctionRegistry,
    statistics: &impl Statistics,
) -> Result<Plan, PlanError> {
    // 1. A malformed term cannot be planned, and no later stage could repair
    //    it: refuse at the boundary, naming the term.
    for term in &request.terms {
        validate_term(term)?;
    }

    // 2. Read the registry's own declarations once. `describe` is IRI-sorted, so
    //    every derived vector below is a pure function of the registry's
    //    contents, not of registration order.
    let descriptors = registry
        .describe()
        .map_err(|error| PlanError::RegistryDeclaration {
            message: error.to_string(),
        })?;
    let content_fingerprint =
        registry
            .content_fingerprint()
            .map_err(|error| PlanError::RegistryDeclaration {
                message: error.to_string(),
            })?;

    // 3. Match request terms to producers by declared capability only. This pass
    //    settles term matching and nothing else; a producer that matches here is
    //    still only a candidate, because whether its declaration can actually
    //    *render* those terms into its argument positions is decided in step 5.
    let mut candidates: Vec<Candidate<'_>> = Vec::new();
    for descriptor in &descriptors {
        let producer = descriptor.iri.clone();
        // A relation registered without a ranked declaration declares nothing,
        // and nothing is what the planner reads back: it does not fuse.
        let Some(declaration) = descriptor.ranked.as_ref() else {
            candidates.push(Candidate::rejected(producer, RejectionReason::NotRanked));
            continue;
        };
        // The seam declares its stratum with the kernel IRI; a plan carries the
        // layer's validated, hashable, orderable wrapper.
        let stratum = Iri::from(declaration.stratum.clone());

        let matched: Vec<u32> = request
            .terms
            .iter()
            .enumerate()
            .filter(|(_, term)| {
                declaration
                    .accepted_terms
                    .iter()
                    .any(|accepted| pattern_matches(&accepted.pattern, term))
            })
            .map(|(index, _)| u32::try_from(index).unwrap_or(u32::MAX))
            .collect();

        if matched.is_empty() {
            candidates.push(Candidate::rejected(
                producer,
                RejectionReason::NoAcceptedTerm,
            ));
            continue;
        }

        candidates.push(Candidate {
            producer,
            outcome: Outcome::Matched {
                descriptor,
                declaration,
                stratum,
                matched,
            },
        });
    }

    // 4. The depth every candidate would be invoked at. `place` renders the
    //    per-stratum depth into an argument for a producer that declares a depth
    //    placement, so the bound has to exist before placement runs — over the
    //    term-matched set, which is the widest set placement can survive from.
    let provisional = depth_bounds(&candidates, statistics);

    // 5. Placement: can this producer's declaration actually render the terms it
    //    matched into its own argument positions, under a mode it declares? A
    //    producer that cannot is rejected here rather than emitted as a call that
    //    silently drops the facet it could not write.
    let mut bindings: Vec<ProducerBinding> = Vec::new();
    let mut decisions: Vec<ProducerDecision> = Vec::with_capacity(candidates.len());
    let mut selected: Vec<(Iri, u64)> = Vec::new();
    for candidate in &candidates {
        let (descriptor, declaration, stratum, matched) = match &candidate.outcome {
            Outcome::Rejected(reason) => {
                decisions.push(ProducerDecision::Rejected {
                    producer: candidate.producer.clone(),
                    reason: *reason,
                });
                continue;
            }
            Outcome::Matched {
                descriptor,
                declaration,
                stratum,
                matched,
            } => (*descriptor, *declaration, stratum, matched),
        };
        let depth = provisional.get(stratum).copied().unwrap_or(u32::MAX);
        if place(
            &candidate.producer,
            descriptor,
            declaration,
            &request.terms,
            matched,
            depth,
        )
        .is_err()
        {
            decisions.push(ProducerDecision::Rejected {
                producer: candidate.producer.clone(),
                reason: RejectionReason::UnsatisfiedConstraint,
            });
            continue;
        }
        decisions.push(ProducerDecision::Selected {
            producer: candidate.producer.clone(),
            stratum: stratum.clone(),
        });
        bindings.push(ProducerBinding {
            producer: candidate.producer.clone(),
            stratum: stratum.clone(),
            request_terms: matched.clone(),
        });
        selected.push((stratum.clone(), declared_row_bound(descriptor)));
    }

    if bindings.is_empty() {
        return Err(PlanError::NoApplicableProducers);
    }

    // 5b. Per-TERM evidence, which the per-producer decisions above cannot
    //     carry. A request term the bindings do not reach went unserved, and the
    //     two ways that happens are different facts about the registry: nothing
    //     declared a shape that accepts it at all, or something did and every
    //     acceptor was then rejected. Recorded for every such term, ascending,
    //     so an armed-but-unserved modality is visible as exactly that rather
    //     than as a term that quietly fell out of the plan.
    let unserved_terms = unserved_terms(&request.terms, &candidates, &bindings);

    // 6. Per-stratum depth over the SURVIVING set: a producer dropped in step 5
    //    can lower its stratum's worst-case bound, and recording the wider bound
    //    would license a depth no remaining producer can fill. The bound is the
    //    declared row count, capped by a measured cardinality when statistics
    //    offer one; an unbounded declaration with no statistic to bound it has no
    //    finite depth to record.
    let mut declared_bounds: BTreeMap<Iri, u64> = BTreeMap::new();
    for (stratum, bound) in selected {
        declared_bounds
            .entry(stratum)
            .and_modify(|current| *current = (*current).max(bound))
            .or_insert(bound);
    }
    let strata: BTreeSet<Iri> = declared_bounds.keys().cloned().collect();
    let mut stratum_depths: HashMap<Iri, u32> = HashMap::with_capacity(strata.len());
    let mut stratum_weights: HashMap<Iri, Weight> = HashMap::with_capacity(strata.len());
    for stratum in &strata {
        let declared = declared_bounds.get(stratum).copied().unwrap_or(0);
        let bound = capped(declared, stratum, statistics);
        if bound == u64::MAX {
            return Err(PlanError::StatisticsUnavailable {
                predicate: Box::new(stratum.clone()),
            });
        }
        stratum_depths.insert(stratum.clone(), u32::try_from(bound).unwrap_or(u32::MAX));
        stratum_weights.insert(stratum.clone(), Weight::from_raw(UNIT_WEIGHT_RAW));
    }

    // 5. Capture the statistics the planner actually consulted: the strata it
    //    placed and the predicates the request named.
    let statistics_snapshot = capture_statistics(request, &strata, statistics);

    // 6. Record both registry identities.
    Ok(Plan {
        version: Plan::VERSION,
        request_terms: request.terms.clone(),
        producer_bindings: bindings,
        producer_decisions: decisions,
        unserved_terms,
        stratum_depths,
        stratum_weights,
        statistics_snapshot,
        registry_instance_id: registry.instance_id(),
        registry_content_fingerprint: content_fingerprint,
        // Planned here, against a registry this process holds, so the instance
        // id above names something admission can still compare against.
        origin: PlanOrigin::SameProcess,
    })
}

/// One considered producer, between term matching and placement.
///
/// Planning is two passes over the registry's descriptions rather than one,
/// because placement needs a depth and the depth needs the set placement
/// survives. This value is what the first pass hands the second; it borrows the
/// registry's own description rather than copying any of it.
struct Candidate<'a> {
    /// The registered producer IRI, byte-exact.
    producer: String,
    /// What the first pass decided.
    outcome: Outcome<'a>,
}

impl Candidate<'_> {
    /// A producer the first pass already refused.
    const fn rejected(producer: String, reason: RejectionReason) -> Self {
        Self {
            producer,
            outcome: Outcome::Rejected(reason),
        }
    }
}

/// The first pass's decision for one producer.
enum Outcome<'a> {
    /// Refused on term matching alone; placement is never consulted.
    Rejected(RejectionReason),
    /// At least one request term matched a declared shape.
    Matched {
        /// The registry's own description of the relation.
        descriptor: &'a PfDescriptor,
        /// The ranked declaration supplied where it was registered.
        declaration: &'a RankedDeclaration,
        /// The stratum it ranks within.
        stratum: Iri,
        /// The request-term indices it matched, ascending.
        matched: Vec<u32>,
    },
}

/// The depth each stratum would carry over the term-matched candidates.
///
/// This is the provisional bound placement is run against, not the bound the
/// plan records: it is computed over the widest set (every producer whose terms
/// matched), so a producer can only ever be handed a depth at least as large as
/// the one its stratum finally records. An unbounded stratum with no statistic
/// has no finite depth here; it is carried as [`u32::MAX`] rather than refused,
/// because the refusal belongs to the surviving set and is raised there.
fn depth_bounds(candidates: &[Candidate<'_>], statistics: &impl Statistics) -> BTreeMap<Iri, u32> {
    let mut bounds: BTreeMap<Iri, u64> = BTreeMap::new();
    for candidate in candidates {
        let Outcome::Matched {
            descriptor,
            stratum,
            ..
        } = &candidate.outcome
        else {
            continue;
        };
        let bound = declared_row_bound(descriptor);
        bounds
            .entry(stratum.clone())
            .and_modify(|current| *current = (*current).max(bound))
            .or_insert(bound);
    }
    bounds
        .into_iter()
        .map(|(stratum, declared)| {
            let bound = capped(declared, &stratum, statistics);
            let depth = u32::try_from(bound).unwrap_or(u32::MAX);
            (stratum, depth)
        })
        .collect()
}

/// Every request term no binding carries, ascending, with the reason it went
/// unserved.
///
/// The two reasons are read off the passes that produced them and are not
/// interchangeable. A term no [`Outcome::Matched`] candidate lists was never
/// accepted by any declaration in the registry — nothing was a candidate for it,
/// which is [`UnservedReason::NoProducerAccepts`]. A term some candidate did
/// list, that nonetheless reaches no binding, was accepted and then left with
/// nothing to answer it when every acceptor was rejected at placement, which is
/// [`UnservedReason::EveryAcceptingProducerRejected`] and sends the reader to
/// that producer's own recorded dimension.
fn unserved_terms(
    terms: &[RequestTerm],
    candidates: &[Candidate<'_>],
    bindings: &[ProducerBinding],
) -> Vec<UnservedTerm> {
    let mut accepted = vec![false; terms.len()];
    for candidate in candidates {
        let Outcome::Matched { matched, .. } = &candidate.outcome else {
            continue;
        };
        for index in matched {
            if let Some(slot) = accepted.get_mut(*index as usize) {
                *slot = true;
            }
        }
    }
    let mut served = vec![false; terms.len()];
    for binding in bindings {
        for index in &binding.request_terms {
            if let Some(slot) = served.get_mut(*index as usize) {
                *slot = true;
            }
        }
    }
    served
        .iter()
        .enumerate()
        .filter(|(_, served)| !**served)
        .map(|(index, _)| UnservedTerm {
            request_term: u32::try_from(index).unwrap_or(u32::MAX),
            reason: if accepted[index] {
                UnservedReason::EveryAcceptingProducerRejected
            } else {
                UnservedReason::NoProducerAccepts
            },
        })
        .collect()
}

/// A declared row bound, lowered (never raised) by a measured cardinality.
fn capped(declared: u64, stratum: &Iri, statistics: &impl Statistics) -> u64 {
    match statistics.cardinality(stratum) {
        Some(cardinality) => declared.min(cardinality),
        None => declared,
    }
}

/// Refuse a request term that cannot name anything.
///
/// The two interval arms are refused on the two grounds an interval can be
/// unusable: it constrains nothing (neither endpoint), or it can match nothing
/// (a lower endpoint above its upper). The second is checked for a numeric range
/// and not for a temporal one, and the asymmetry is the point rather than an
/// omission: a numeric endpoint is an exact [`Fixed`] this layer can order, while
/// a temporal endpoint is the caller's own lexical form, which this layer does
/// not parse and therefore cannot order — comparing two calendar lexicals as
/// strings would refuse legitimate intervals whose encoding is not
/// lexicographically ordered. A degenerate range whose endpoints are equal is a
/// single point and is admitted.
fn validate_term(term: &RequestTerm) -> Result<(), PlanError> {
    let reason = match term {
        RequestTerm::Lexical { text, .. } if text.trim().is_empty() => "lexical text is empty",
        RequestTerm::Vector { embedding, .. } if embedding.is_empty() => {
            "vector embedding is empty"
        }
        RequestTerm::Vector { embedding, .. }
            if embedding.iter().any(|value| !value.is_finite()) =>
        {
            "vector embedding contains a non-finite value"
        }
        RequestTerm::Spatial { geometry, .. } if geometry.trim().is_empty() => {
            "spatial geometry is empty"
        }
        RequestTerm::EntitySeed { entity } if entity.as_str().trim().is_empty() => {
            "entity seed is empty"
        }
        RequestTerm::Temporal {
            lower: None,
            upper: None,
            ..
        } => "temporal interval carries neither endpoint",
        RequestTerm::Temporal { lower, upper, .. }
            if lower
                .iter()
                .chain(upper.iter())
                .any(|end| end.trim().is_empty()) =>
        {
            "temporal interval endpoint is empty"
        }
        RequestTerm::NumericRange {
            lower: None,
            upper: None,
            ..
        } => "numeric range carries neither endpoint",
        RequestTerm::NumericRange {
            lower: Some(lower),
            upper: Some(upper),
            ..
        } if lower > upper => "numeric range lower endpoint exceeds its upper endpoint",
        _ => return Ok(()),
    };
    Err(PlanError::InvalidRequestTerm {
        term: Box::new(term.clone()),
        reason: reason.to_owned(),
    })
}

/// A producer's worst-case declared row count across its access modes.
///
/// This is the registry's own cost declaration (`rows_per_invocation` per
/// declared mode), read from the descriptor — never a parallel field.
fn declared_row_bound(descriptor: &PfDescriptor) -> u64 {
    descriptor
        .modes
        .iter()
        .map(|mode| mode.rows_per_invocation)
        .max()
        .unwrap_or(0)
}

/// The predicate a request term names, when it names one.
fn term_predicate(term: &RequestTerm) -> Option<&Iri> {
    match term {
        RequestTerm::Lexical { predicate, .. } => predicate.as_ref(),
        RequestTerm::Spatial { predicate, .. }
        | RequestTerm::Temporal { predicate, .. }
        | RequestTerm::NumericRange { predicate, .. } => Some(predicate),
        RequestTerm::Vector { .. } | RequestTerm::EntitySeed { .. } => None,
    }
}

/// Record the statistics the planner consulted, in a deterministic order.
///
/// Only facts the provider actually reports are recorded: a subject with no
/// cardinality is omitted rather than recorded as zero, because zero is a
/// measurement and absence is not.
fn capture_statistics(
    request: &RetrievalRequest,
    strata: &BTreeSet<Iri>,
    statistics: &impl Statistics,
) -> StatisticsSnapshot {
    let mut subjects: BTreeSet<Iri> = strata.clone();
    for term in &request.terms {
        if let Some(predicate) = term_predicate(term) {
            subjects.insert(predicate.clone());
        }
    }

    let mut entries = Vec::new();
    for subject in subjects {
        let Some(cardinality) = statistics.cardinality(&subject) else {
            continue;
        };
        entries.push(StatisticsEntry {
            subject: subject.as_str().to_owned(),
            cardinality,
            selectivity_ppm: selectivity_ppm(&subject, request, statistics),
        });
    }

    StatisticsSnapshot {
        source: statistics.source().to_owned(),
        revision: statistics.revision().to_owned(),
        entries,
    }
}

/// The most selective reported selectivity for a subject's request terms, in
/// parts per million.
fn selectivity_ppm(
    subject: &Iri,
    request: &RetrievalRequest,
    statistics: &impl Statistics,
) -> Option<u64> {
    let mut most_selective: Option<f64> = None;
    for term in &request.terms {
        if term_predicate(term) != Some(subject) {
            continue;
        }
        if let Some(value) = statistics.selectivity(subject, term) {
            most_selective = Some(most_selective.map_or(value, |current| current.min(value)));
        }
    }
    most_selective.map(|value| {
        // The trait's contract is [0, 1]; clamping keeps a provider fault from
        // producing a nonsensical ratio while still recording a bounded fact.
        let ratio = if value.is_finite() {
            value.clamp(0.0, 1.0)
        } else {
            0.0
        };
        (ratio * 1_000_000.0).round() as u64
    })
}
