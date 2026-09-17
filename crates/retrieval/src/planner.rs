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
//! * a lexical term and a spatial term target a **literal**;
//! * an entity seed targets the kind its canonical lexical names (`<…>` an IRI,
//!   `_:…` a blank node, `"…"` a literal);
//! * a vector term targets **no** RDF term kind, so only a producer that
//!   declares [`TermKind::Any`] accepts it.
//!
//! A pattern's `datatype` constraint can never match, because no request term
//! carries a datatype; its `language` constraint matches only a lexical term
//! with that tag; its `predicate` constraint matches only a lexical or spatial
//! term carrying that predicate.
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

use purrdf_sparql_eval::{PfDescriptor, PropertyFunctionRegistry, TermKind, TermPattern};
use purrdf_text::Fixed;

use crate::error::PlanError;
use crate::iri::{Iri, Weight};
use crate::plan::{
    Plan, ProducerBinding, ProducerDecision, RejectionReason, StatisticsEntry, StatisticsSnapshot,
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
/// a reason — the per-stratum depth derived from the registry's row-bound
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
///   term of the request.
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

    // 3. Match request terms to producers by declared capability only.
    let mut bindings: Vec<ProducerBinding> = Vec::new();
    let mut decisions: Vec<ProducerDecision> = Vec::new();
    // Stratum -> worst-case declared row bound over its selected producers.
    let mut declared_bounds: BTreeMap<Iri, u64> = BTreeMap::new();

    for descriptor in &descriptors {
        let producer = descriptor.iri.clone();
        // A relation registered without a ranked declaration declares nothing,
        // and nothing is what the planner reads back: it does not fuse.
        let Some(declaration) = descriptor.ranked.as_ref() else {
            decisions.push(ProducerDecision::Rejected {
                producer,
                reason: RejectionReason::NotRanked,
            });
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
            decisions.push(ProducerDecision::Rejected {
                producer,
                reason: RejectionReason::NoAcceptedTerm,
            });
            continue;
        }

        decisions.push(ProducerDecision::Selected {
            producer: producer.clone(),
            stratum: stratum.clone(),
        });
        bindings.push(ProducerBinding {
            producer,
            stratum: stratum.clone(),
            request_terms: matched,
        });

        let bound = declared_row_bound(descriptor);
        declared_bounds
            .entry(stratum)
            .and_modify(|current| *current = (*current).max(bound))
            .or_insert(bound);
    }

    if bindings.is_empty() {
        return Err(PlanError::NoApplicableProducers);
    }

    // 4. Per-stratum depth: the declared row bound, capped by a measured
    //    cardinality when statistics offer one. An unbounded declaration with no
    //    statistic to bound it has no finite depth to record.
    let strata: BTreeSet<Iri> = declared_bounds.keys().cloned().collect();
    let mut stratum_depths: HashMap<Iri, u32> = HashMap::with_capacity(strata.len());
    let mut stratum_weights: HashMap<Iri, Weight> = HashMap::with_capacity(strata.len());
    for stratum in &strata {
        let declared = declared_bounds.get(stratum).copied().unwrap_or(0);
        let bound = match statistics.cardinality(stratum) {
            Some(cardinality) => declared.min(cardinality),
            None => declared,
        };
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
        stratum_depths,
        stratum_weights,
        statistics_snapshot,
        registry_instance_id: registry.instance_id(),
        registry_content_fingerprint: content_fingerprint,
    })
}

/// Refuse a request term that cannot name anything.
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

/// Whether the producer's declared pattern accepts the request term.
fn pattern_matches(pattern: &TermPattern, term: &RequestTerm) -> bool {
    if !kind_matches(pattern.kind, term) {
        return false;
    }
    // No request term carries a datatype, so a datatype constraint matches nothing.
    if pattern.datatype.is_some() {
        return false;
    }
    if let Some(language) = &pattern.language {
        match term {
            RequestTerm::Lexical {
                language: Some(term_language),
                ..
            } if term_language == language => {}
            _ => return false,
        }
    }
    if let Some(predicate) = &pattern.predicate {
        let matches = match term {
            RequestTerm::Lexical {
                predicate: Some(term_predicate),
                ..
            } => term_predicate.as_str() == predicate,
            RequestTerm::Spatial {
                predicate: term_predicate,
                ..
            } => term_predicate.as_str() == predicate,
            _ => false,
        };
        if !matches {
            return false;
        }
    }
    true
}

/// Whether a declared term kind accepts a request term.
fn kind_matches(kind: TermKind, term: &RequestTerm) -> bool {
    if kind == TermKind::Any {
        return true;
    }
    match term {
        RequestTerm::Lexical { .. } | RequestTerm::Spatial { .. } => kind == TermKind::Literal,
        RequestTerm::EntitySeed { entity } => seed_kind(entity.as_str()) == Some(kind),
        // A vector term targets no RDF term kind; only `Any` accepts it.
        RequestTerm::Vector { .. } => false,
    }
}

/// The RDF term kind a canonical seed lexical names, when it names one.
fn seed_kind(text: &str) -> Option<TermKind> {
    if text.starts_with('<') {
        Some(TermKind::Iri)
    } else if text.starts_with("_:") {
        Some(TermKind::Blank)
    } else if text.starts_with('"') {
        Some(TermKind::Literal)
    } else {
        None
    }
}

/// The predicate a request term names, when it names one.
fn term_predicate(term: &RequestTerm) -> Option<&Iri> {
    match term {
        RequestTerm::Lexical { predicate, .. } => predicate.as_ref(),
        RequestTerm::Spatial { predicate, .. } => Some(predicate),
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
