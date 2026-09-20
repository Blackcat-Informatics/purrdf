// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The plan value's contract: equality, serialization, canonical stability,
//! digest sensitivity, decode round-trip and loud version refusal.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use pretty_assertions::assert_eq;
use purrdf_retrieval::{
    DepthInputs, Fixed, Iri, Metric, PLAN_VERSION, Plan, PlanError, PlanOrigin, ProducerBinding,
    ProducerDecision, ReadBound, RegistryId, RejectionReason, RequestTerm, StatisticsEntry,
    StatisticsSnapshot, Term, TopK, UnservedReason, UnservedTerm,
};
use purrdf_sparql_eval::{
    CandidateDomains, DomainTag, DuplicatePolicy, MemoryRelation, PropertyFunctionRegistry,
    RankedDeclaration,
};

fn iri(text: &str) -> Iri {
    Iri::parse(text).expect("fixture IRIs are valid")
}

fn stratum() -> Iri {
    iri("http://example.org/stratum/a")
}

fn baseline() -> Plan {
    let mut stratum_depths = HashMap::new();
    stratum_depths.insert(stratum(), 10);

    // The inputs derive the depth beside them, so the baseline is a plan that
    // certifies: a declaration of ten rows, no reported statistic, under a
    // licensed prefix of the request's own twenty-five. Building it any other way
    // would make every test that clones it start from a plan whose own evidence
    // contradicts it.
    let mut stratum_derivations = BTreeMap::new();
    stratum_derivations.insert(
        stratum(),
        DepthInputs {
            declared: 10,
            cardinality: None,
            selectivity_ppm: None,
            selectivity_terms: Vec::new(),
            licensed_prefix: Some(25),
        },
    );

    Plan {
        version: Plan::VERSION,
        read_bound: ReadBound::Bounded(TopK::new(25)),
        request_terms: vec![
            RequestTerm::Lexical {
                text: "quick brown".to_owned(),
                language: Some("en".to_owned()),
                predicate: Some(iri("http://example.org/p")),
            },
            RequestTerm::Vector {
                embedding: vec![0.25, -1.5],
                metric: Metric::Cosine,
                index_hint: Some("hint".to_owned()),
            },
            RequestTerm::Spatial {
                geometry: "POINT(0 0)".to_owned(),
                predicate: iri("http://example.org/geo"),
                max_distance: Some(Fixed::from_raw(5)),
            },
            RequestTerm::EntitySeed {
                entity: Term::new("<http://example.org/e>"),
            },
        ],
        producer_bindings: vec![ProducerBinding {
            producer: "http://example.org/pf/text".to_owned(),
            stratum: stratum(),
            request_terms: vec![0],
        }],
        producer_decisions: vec![
            ProducerDecision::Selected {
                producer: "http://example.org/pf/text".to_owned(),
                stratum: stratum(),
            },
            ProducerDecision::Rejected {
                producer: "http://example.org/pf/knn".to_owned(),
                reason: RejectionReason::NoAcceptedTerm,
            },
        ],
        // The baseline binds only term 0, so the other three reached nothing —
        // one of them because something accepted its shape and was then
        // rejected, which is a different fact from nothing accepting it at all.
        unserved_terms: vec![
            UnservedTerm {
                request_term: 1,
                reason: UnservedReason::NoProducerAccepts,
            },
            UnservedTerm {
                request_term: 2,
                reason: UnservedReason::EveryAcceptingProducerRejected,
            },
            UnservedTerm {
                request_term: 3,
                reason: UnservedReason::NoProducerAccepts,
            },
        ],
        stratum_depths,
        stratum_derivations,
        statistics_snapshot: StatisticsSnapshot {
            source: "example-statistics".to_owned(),
            revision: "r1".to_owned(),
            entries: vec![StatisticsEntry {
                subject: "http://example.org/p".to_owned(),
                cardinality: Some(42),
                selectivity_ppm: Some(1_000),
                selectivity_terms: vec![0],
            }],
        },
        registry_instance_id: RegistryId::from_raw(7),
        registry_content_fingerprint: "example-fingerprint".to_owned(),
        origin: PlanOrigin::SameProcess,
    }
}

#[test]
fn every_decode_path_records_a_deserialized_origin() {
    let plan = baseline();
    assert_eq!(
        plan.origin,
        PlanOrigin::SameProcess,
        "a plan built here names a registry identity this process can compare"
    );

    let decoded = Plan::from_canonical_bytes(&plan.canonical_bytes()).expect("canonical decode");
    assert_eq!(
        decoded.origin,
        PlanOrigin::Deserialized,
        "the canonical decoder records that the instance id it read is foreign"
    );

    let json = serde_json::to_string(&plan).expect("plan serializes");
    let round_tripped: Plan = serde_json::from_str(&json).expect("plan deserializes");
    assert_eq!(
        round_tripped.origin,
        PlanOrigin::Deserialized,
        "serde skips origin, so a decoded plan gets the default, which is the decoded case"
    );
    assert!(
        !json.contains("origin"),
        "origin is provenance, not content, so it is not part of the document: {json}"
    );
}

#[test]
fn origin_is_provenance_and_moves_neither_bytes_nor_identity() {
    // A plan and its own round trip differ in origin and in nothing else, so
    // they must stay one plan with one identity; otherwise pinning a plan and
    // reloading it would name a different plan.
    let planned = baseline();
    let decoded = Plan::from_canonical_bytes(&planned.canonical_bytes()).expect("canonical decode");
    assert_ne!(planned.origin, decoded.origin);
    assert_eq!(decoded, planned, "equality is over content");
    assert_eq!(decoded.canonical_bytes(), planned.canonical_bytes());
    assert_eq!(decoded.id(), planned.id());

    let mut flipped = baseline();
    flipped.origin = PlanOrigin::Deserialized;
    assert_eq!(flipped.id(), baseline().id(), "origin is not in the digest");
}

#[test]
fn plan_equality_is_field_equality() {
    assert_eq!(baseline(), baseline());

    let mut changed = baseline();
    changed.registry_content_fingerprint = "other".to_owned();
    assert_ne!(baseline(), changed);
}

#[test]
fn plans_differing_only_in_map_insertion_order_are_equal() {
    let mut left = baseline();
    left.stratum_depths
        .insert(iri("http://example.org/stratum/b"), 3);
    let mut right = baseline();
    right
        .stratum_depths
        .insert(iri("http://example.org/stratum/b"), 3);
    // `HashMap` has no insertion-order guarantee; equality (and the canonical
    // bytes below) must therefore not depend on it.
    assert_eq!(left, right);
    assert_eq!(left.canonical_bytes(), right.canonical_bytes());
    assert_eq!(left.id(), right.id());
}

#[test]
fn serde_round_trip_preserves_the_plan() {
    let plan = baseline();
    let json = serde_json::to_string(&plan).expect("plan serializes");
    let decoded: Plan = serde_json::from_str(&json).expect("plan deserializes");
    assert_eq!(decoded, plan);
    assert_eq!(decoded.id(), plan.id());
}

#[test]
fn canonical_bytes_and_decode_round_trip() {
    let plan = baseline();
    let bytes = plan.canonical_bytes();
    assert_eq!(bytes, plan.canonical_bytes(), "encoding is deterministic");
    let decoded = Plan::from_canonical_bytes(&bytes).expect("canonical decode");
    assert_eq!(decoded, plan);
    assert_eq!(decoded.canonical_bytes(), bytes);
}

/// Every rejection reason survives the canonical round trip under its own
/// discriminator byte.
///
/// A reason is the evidence a caller reads to find out why its answer is
/// narrower than it asked for, and the byte it is written as lands in the
/// identity of the plan. A reason that decoded as a *different* reason would be a plan that
/// reads back as blaming the wrong dimension while still carrying an identity
/// the caller recognises, so every variant is encoded and decoded here rather
/// than only the ones a fixture happens to produce.
#[test]
fn every_rejection_reason_round_trips_under_its_own_tag() {
    let reasons = [
        RejectionReason::NotRanked,
        RejectionReason::NoAcceptedTerm,
        RejectionReason::DepthExceeded,
        RejectionReason::UnsatisfiedConstraint,
    ];
    let mut ids = Vec::with_capacity(reasons.len());
    for reason in reasons {
        let mut plan = baseline();
        plan.producer_decisions = vec![ProducerDecision::Rejected {
            producer: "http://example.org/pf/knn".to_owned(),
            reason,
        }];
        let bytes = plan.canonical_bytes();
        let decoded = Plan::from_canonical_bytes(&bytes).expect("canonical decode");
        assert_eq!(
            decoded.producer_decisions, plan.producer_decisions,
            "{reason:?} decoded as something else"
        );
        assert_eq!(decoded.canonical_bytes(), bytes);
        ids.push(plan.id());
    }
    let distinct: std::collections::BTreeSet<_> = ids.iter().collect();
    assert_eq!(
        distinct.len(),
        ids.len(),
        "each reason takes its own byte, so each plan takes its own identity"
    );
}

/// The plan carrying `reason` as its one rejection decision.
fn plan_rejecting_with(reason: RejectionReason) -> Plan {
    let mut plan = baseline();
    plan.producer_decisions = vec![ProducerDecision::Rejected {
        producer: "http://example.org/pf/knn".to_owned(),
        reason,
    }];
    plan
}

/// A rejection-reason byte no variant is written as is refused, not substituted.
///
/// The vocabulary is closed and its bytes are dense, so the first byte past the
/// last variant is the one a plan written by a differently-versioned peer — or by
/// hand — would carry. Decoding it as *some* reason would hand a caller a verdict
/// about its own query that nothing in this process ever decided: the point of
/// the reason is that it is evidence.
///
/// The tag's position is located rather than hard-coded, by encoding two plans
/// that differ in nothing but the reason and taking the one byte that moved.
#[test]
fn a_rejection_reason_byte_no_variant_is_written_as_is_refused() {
    let depth = plan_rejecting_with(RejectionReason::DepthExceeded).canonical_bytes();
    let mut constraint =
        plan_rejecting_with(RejectionReason::UnsatisfiedConstraint).canonical_bytes();
    assert_eq!(
        depth.len(),
        constraint.len(),
        "two reasons are one byte each, so the encodings are the same length"
    );
    let moved: Vec<usize> = depth
        .iter()
        .zip(&constraint)
        .enumerate()
        .filter(|(_, (left, right))| left != right)
        .map(|(index, _)| index)
        .collect();
    assert_eq!(
        moved.len(),
        1,
        "the reason is the only thing that differs, so exactly one byte moved"
    );
    let tag = moved[0];
    assert_eq!(
        (depth[tag], constraint[tag]),
        (2, 3),
        "and the bytes at that position are the two reasons' own dense tags"
    );

    // One past the last variant: the byte a peer that still wrote a reason this
    // vocabulary no longer has would put here.
    constraint[tag] = 4;
    let error = Plan::from_canonical_bytes(&constraint)
        .expect_err("an unknown rejection reason is refused");
    match error {
        PlanError::InvalidTag { what, tag: byte } => {
            assert_eq!(what, "rejection reason");
            assert_eq!(byte, 4);
        }
        other => panic!("expected InvalidTag, got {other:?}"),
    }

    // The neighbour that must still decode: the same bytes with the real tag back
    // in place. A decoder that refused every reason would pass the assertion
    // above and be useless.
    constraint[tag] = 3;
    let decoded = Plan::from_canonical_bytes(&constraint).expect("the real tag still decodes");
    assert_eq!(
        decoded.producer_decisions,
        plan_rejecting_with(RejectionReason::UnsatisfiedConstraint).producer_decisions
    );
}

/// Every interval shape the two new arms admit, so the canonical encoding's new
/// tags are exercised in both directions and the arms' hand-written equality is
/// held to the same standard the existing ones are.
fn interval_terms() -> Vec<RequestTerm> {
    vec![
        RequestTerm::Temporal {
            predicate: iri("http://example.org/observed"),
            lower: Some("2026-01-01T00:00:00Z".to_owned()),
            upper: Some("2026-02-01T00:00:00Z".to_owned()),
        },
        RequestTerm::Temporal {
            predicate: iri("http://example.org/observed"),
            lower: None,
            upper: Some("2026-02-01T00:00:00Z".to_owned()),
        },
        RequestTerm::NumericRange {
            predicate: iri("http://example.org/price"),
            lower: Some(Fixed::from_raw(-1_500_000_000_000)),
            upper: Some(Fixed::from_raw(2_250_000_000_000)),
        },
        RequestTerm::NumericRange {
            predicate: iri("http://example.org/price"),
            lower: Some(Fixed::ONE),
            upper: None,
        },
    ]
}

#[test]
fn interval_terms_round_trip_through_the_canonical_encoding() {
    let mut plan = baseline();
    plan.request_terms.extend(interval_terms());
    let bytes = plan.canonical_bytes();
    assert_eq!(bytes, plan.canonical_bytes(), "encoding is deterministic");
    let decoded = Plan::from_canonical_bytes(&bytes).expect("canonical decode");
    assert_eq!(decoded.request_terms, plan.request_terms);
    assert_eq!(decoded.canonical_bytes(), bytes);
    assert_eq!(decoded.id(), plan.id());
}

#[test]
fn interval_terms_round_trip_through_serde() {
    let mut plan = baseline();
    plan.request_terms.extend(interval_terms());
    let json = serde_json::to_string(&plan).expect("a plan serializes");
    let decoded: Plan = serde_json::from_str(&json).expect("a plan deserializes");
    assert_eq!(decoded.request_terms, plan.request_terms);
    assert_eq!(decoded.id(), plan.id());
}

#[test]
fn the_digest_separates_every_interval_field() {
    // Each field of each new arm is in the identity, and no two distinct
    // intervals share one. The empty-interval case is deliberately absent: the
    // planner refuses it, so no admitted plan can carry one.
    let mut base = baseline();
    base.request_terms = interval_terms();
    let base_id = base.id();
    for (label, term) in [
        (
            "temporal predicate",
            RequestTerm::Temporal {
                predicate: iri("http://example.org/elsewhere"),
                lower: Some("2026-01-01T00:00:00Z".to_owned()),
                upper: Some("2026-02-01T00:00:00Z".to_owned()),
            },
        ),
        (
            "temporal lower endpoint",
            RequestTerm::Temporal {
                predicate: iri("http://example.org/observed"),
                lower: Some("2026-01-02T00:00:00Z".to_owned()),
                upper: Some("2026-02-01T00:00:00Z".to_owned()),
            },
        ),
        (
            "temporal upper endpoint absent",
            RequestTerm::Temporal {
                predicate: iri("http://example.org/observed"),
                lower: Some("2026-01-01T00:00:00Z".to_owned()),
                upper: None,
            },
        ),
    ] {
        let mut changed = base.clone();
        changed.request_terms[0] = term;
        assert_ne!(changed.id(), base_id, "{label}");
    }
    for (label, term) in [
        (
            "numeric predicate",
            RequestTerm::NumericRange {
                predicate: iri("http://example.org/cost"),
                lower: Some(Fixed::from_raw(-1_500_000_000_000)),
                upper: Some(Fixed::from_raw(2_250_000_000_000)),
            },
        ),
        (
            "numeric lower endpoint",
            RequestTerm::NumericRange {
                predicate: iri("http://example.org/price"),
                lower: Some(Fixed::from_raw(-1_500_000_000_001)),
                upper: Some(Fixed::from_raw(2_250_000_000_000)),
            },
        ),
        (
            "numeric upper endpoint absent",
            RequestTerm::NumericRange {
                predicate: iri("http://example.org/price"),
                lower: Some(Fixed::from_raw(-1_500_000_000_000)),
                upper: None,
            },
        ),
    ] {
        let mut changed = base.clone();
        changed.request_terms[2] = term;
        assert_ne!(changed.id(), base_id, "{label}");
    }
    // And a term that differs only in which arm it is: a temporal interval and a
    // numeric range carrying the same predicate are not one term.
    let mut swapped = base.clone();
    swapped.request_terms[0] = RequestTerm::NumericRange {
        predicate: iri("http://example.org/observed"),
        lower: None,
        upper: Some(Fixed::ONE),
    };
    assert_ne!(swapped.id(), base_id, "the arm itself is in the identity");
    assert_ne!(
        swapped.request_terms[0], base.request_terms[0],
        "and in equality"
    );
}

#[test]
fn canonical_bytes_are_stable_across_map_order() {
    let mut plan = baseline();
    plan.stratum_depths
        .insert(iri("http://example.org/stratum/z"), 20);
    plan.stratum_depths
        .insert(iri("http://example.org/stratum/m"), 30);
    let mut shuffled = plan.clone();
    shuffled.stratum_depths.clear();
    for key in [
        "http://example.org/stratum/z",
        "http://example.org/stratum/a",
        "http://example.org/stratum/m",
    ] {
        shuffled
            .stratum_depths
            .insert(iri(key), plan.stratum_depths[&iri(key)]);
    }
    assert_eq!(plan.canonical_bytes(), shuffled.canonical_bytes());
}

#[test]
// Each mutation starts from a fresh clone of the baseline on purpose; the final
// clone is flagged only because it happens to be the last use of `base`.
#[allow(clippy::redundant_clone)]
// Every *content* field. `origin` is provenance rather than content and is
// deliberately outside the digest; `origin_is_provenance_and_moves_neither_bytes_nor_identity`
// pins that exclusion from the other side.
fn digest_is_sensitive_to_every_field() {
    let base = baseline();
    let base_id = base.id();

    let mut changed = base.clone();
    changed.version = base.version.wrapping_add(1);
    assert_ne!(changed.id(), base_id, "version");

    let mut changed = base.clone();
    changed.request_terms[0] = RequestTerm::Lexical {
        text: "different".to_owned(),
        language: Some("en".to_owned()),
        predicate: Some(iri("http://example.org/p")),
    };
    assert_ne!(changed.id(), base_id, "request text");

    let mut changed = base.clone();
    if let RequestTerm::Vector { embedding, .. } = &mut changed.request_terms[1] {
        embedding[0] = 0.5;
    }
    assert_ne!(changed.id(), base_id, "embedding");

    let mut changed = base.clone();
    changed.producer_bindings[0].request_terms.push(1);
    assert_ne!(changed.id(), base_id, "producer binding");

    let mut changed = base.clone();
    changed.producer_decisions.push(ProducerDecision::Rejected {
        producer: "http://example.org/pf/geo".to_owned(),
        reason: RejectionReason::DepthExceeded,
    });
    assert_ne!(changed.id(), base_id, "producer decision");

    let mut changed = base.clone();
    changed.unserved_terms[0].reason = UnservedReason::Unbound;
    assert_ne!(changed.id(), base_id, "unserved term reason");

    let mut changed = base.clone();
    changed.unserved_terms.truncate(2);
    assert_ne!(changed.id(), base_id, "unserved term list");

    let mut changed = base.clone();
    changed.stratum_depths.insert(stratum(), 11);
    assert_ne!(changed.id(), base_id, "stratum depth");

    let mut changed = base.clone();
    changed.statistics_snapshot.revision = "r2".to_owned();
    assert_ne!(changed.id(), base_id, "statistics revision");

    let mut changed = base.clone();
    changed.statistics_snapshot.entries[0].cardinality = Some(43);
    assert_ne!(changed.id(), base_id, "statistics entry");

    // A statistic the plan was built against that the plan does not carry into
    // its identity is a statistic a replay cannot be held to, so every field of
    // the derivation record moves the id — including the term domain of the
    // selectivity, whose aggregate alone would be identical under a provider that
    // moved the same number to a different term.
    let mut changed = base.clone();
    changed.statistics_snapshot.entries[0].selectivity_terms = vec![1];
    assert_ne!(changed.id(), base_id, "statistics selectivity term domain");

    for (label, mutate) in [
        ("derivation declared", 0usize),
        ("derivation cardinality", 1),
        ("derivation selectivity", 2),
        ("derivation selectivity terms", 3),
        ("derivation licensed prefix", 4),
    ] {
        let mut changed = base.clone();
        let inputs = changed
            .stratum_derivations
            .get_mut(&stratum())
            .expect("the baseline records the stratum's derivation");
        match mutate {
            0 => inputs.declared = 11,
            1 => inputs.cardinality = Some(9),
            2 => inputs.selectivity_ppm = Some(500_000),
            3 => inputs.selectivity_terms = vec![2],
            _ => inputs.licensed_prefix = None,
        }
        assert_ne!(changed.id(), base_id, "{label}");
    }

    let mut changed = base.clone();
    changed.registry_instance_id = RegistryId::from_raw(8);
    assert_ne!(changed.id(), base_id, "registry instance id");

    let mut changed = base.clone();
    changed.registry_content_fingerprint = "other-fingerprint".to_owned();
    assert_ne!(changed.id(), base_id, "registry content fingerprint");
}

/// The evidence a plan supports is read off the plan in hand, so a recorded
/// list that has drifted from the bindings cannot make the answer lie in either
/// direction.
#[test]
fn unserved_evidence_reports_what_this_plan_supports_not_what_it_recorded() {
    // The ordinary case: the recorded list and the bindings agree, so the
    // evidence is the recorded list with its recorded reasons.
    let plan = baseline();
    assert_eq!(
        plan.unserved_evidence(),
        vec![
            UnservedTerm {
                request_term: 1,
                reason: UnservedReason::NoProducerAccepts,
            },
            UnservedTerm {
                request_term: 2,
                reason: UnservedReason::EveryAcceptingProducerRejected,
            },
            UnservedTerm {
                request_term: 3,
                reason: UnservedReason::NoProducerAccepts,
            },
        ],
        "an unedited plan reports exactly the terms it recorded, ascending"
    );

    // A term the plan does bind is not reported unserved, whatever a stale or
    // forged list claims: the plan itself falsifies the alarm.
    let mut stale = baseline();
    stale.unserved_terms.push(UnservedTerm {
        request_term: 0,
        reason: UnservedReason::NoProducerAccepts,
    });
    assert!(
        stale
            .unserved_evidence()
            .iter()
            .all(|entry| entry.request_term != 0),
        "term 0 is bound, so no list entry can report it as unanswered"
    );

    // And a term stranded by an edit is reported even though the list never
    // mentioned it — the quiet omission this evidence exists to prevent.
    let mut narrowed = baseline();
    narrowed.unserved_terms.clear();
    assert_eq!(
        narrowed.unserved_evidence(),
        vec![
            UnservedTerm {
                request_term: 1,
                reason: UnservedReason::Unbound,
            },
            UnservedTerm {
                request_term: 2,
                reason: UnservedReason::Unbound,
            },
            UnservedTerm {
                request_term: 3,
                reason: UnservedReason::Unbound,
            },
        ],
        "a term no binding carries is named, with no reason claimed for it"
    );

    // The valid neighbour: a plan whose bindings cover every term reports
    // nothing at all.
    let mut complete = baseline();
    complete.unserved_terms.clear();
    complete.producer_bindings[0].request_terms = vec![0, 1, 2, 3];
    assert!(
        complete.unserved_evidence().is_empty(),
        "a request every term of which reached a producer has no evidence to report, got {:?}",
        complete.unserved_evidence()
    );
}

#[test]
fn version_mismatch_on_decode_refuses_loudly() {
    let mut bytes = baseline().canonical_bytes();
    let mismatched = (PLAN_VERSION + 1).to_le_bytes();
    bytes[0] = mismatched[0];
    bytes[1] = mismatched[1];
    match Plan::from_canonical_bytes(&bytes) {
        Err(PlanError::VersionMismatch { found, expected }) => {
            assert_eq!(found, PLAN_VERSION + 1);
            assert_eq!(expected, PLAN_VERSION);
        }
        other => panic!("expected a loud version mismatch, got {other:?}"),
    }
}

/// Version 1 carried a per-stratum weight map between the depths and the
/// statistics snapshot. Removing it moved every byte after the depths, so a
/// version-1 encoding read under this layout would take the old weight count
/// for the statistics source's length — a plan nobody wrote, under an identity a
/// caller still recognises. The version therefore moved with the layout, and the
/// old one is refused by name.
#[test]
fn the_superseded_layout_is_refused_by_name_rather_than_misread() {
    let mut bytes = baseline().canonical_bytes();
    bytes[..2].copy_from_slice(&1_u16.to_le_bytes());
    match Plan::from_canonical_bytes(&bytes) {
        Err(PlanError::VersionMismatch { found, expected }) => {
            assert_eq!(found, 1, "the refusal names the layout it was handed");
            assert_eq!(expected, PLAN_VERSION);
        }
        other => panic!("expected a loud version mismatch, got {other:?}"),
    }

    // THE NEIGHBOURING CASE: the same plan, written by this build, still
    // decodes. What is refused is the old layout, not the plan.
    let plan = baseline();
    assert_eq!(
        Plan::from_canonical_bytes(&plan.canonical_bytes()).expect("this build's layout decodes"),
        plan
    );
}

#[test]
fn truncated_canonical_bytes_are_refused() {
    let mut bytes = baseline().canonical_bytes();
    bytes.truncate(bytes.len() - 1);
    assert!(matches!(
        Plan::from_canonical_bytes(&bytes),
        Err(PlanError::Truncated { .. })
    ));
}

#[test]
fn signed_zero_embeddings_are_distinct() {
    let positive = RequestTerm::Vector {
        embedding: vec![0.0, 1.0],
        metric: Metric::Cosine,
        index_hint: None,
    };
    let negative = RequestTerm::Vector {
        embedding: vec![-0.0, 1.0],
        metric: Metric::Cosine,
        index_hint: None,
    };
    assert_ne!(
        positive, negative,
        "0.0 and -0.0 have different bit patterns"
    );

    let mut plan_positive = baseline();
    plan_positive.request_terms = vec![positive];
    let mut plan_negative = baseline();
    plan_negative.request_terms = vec![negative];
    assert_ne!(
        plan_positive.id(),
        plan_negative.id(),
        "equality and canonical identity must agree"
    );
}

#[test]
fn nan_embeddings_are_reflexive() {
    let term = RequestTerm::Vector {
        embedding: vec![f32::NAN, 1.0],
        metric: Metric::Cosine,
        index_hint: None,
    };
    assert_eq!(term, term.clone(), "a NaN embedding equals its own clone");

    let mut plan = baseline();
    plan.request_terms = vec![term.clone()];
    let mut plan_clone = baseline();
    plan_clone.request_terms = vec![term];
    assert_eq!(plan.id(), plan_clone.id());
}

#[test]
fn identical_embeddings_are_equal() {
    let left = RequestTerm::Vector {
        embedding: vec![0.25, -1.5, 3.0],
        metric: Metric::Cosine,
        index_hint: Some("hint".to_owned()),
    };
    let right = RequestTerm::Vector {
        embedding: vec![0.25, -1.5, 3.0],
        metric: Metric::Cosine,
        index_hint: Some("hint".to_owned()),
    };
    assert_eq!(left, right, "ordinary finite embeddings remain equal");

    let mut plan_left = baseline();
    plan_left.request_terms = vec![left];
    let mut plan_right = baseline();
    plan_right.request_terms = vec![right];
    assert_eq!(plan_left.id(), plan_right.id());
}

#[test]
fn equal_lexical_plans_have_equal_canonical_bytes() {
    let mut left = baseline();
    left.request_terms = vec![RequestTerm::Lexical {
        text: "quick brown".to_owned(),
        language: Some("en".to_owned()),
        predicate: Some(iri("http://example.org/p")),
    }];
    let mut right = baseline();
    right.request_terms = vec![RequestTerm::Lexical {
        text: "quick brown".to_owned(),
        language: Some("en".to_owned()),
        predicate: Some(iri("http://example.org/p")),
    }];
    assert_eq!(left, right);
    assert_eq!(left.canonical_bytes(), right.canonical_bytes());
    assert_eq!(left.id(), right.id());
}

/// The grep gate: no function pointer may appear anywhere in this crate's
/// sources. A function pointer cannot be serialized, compared for equality, or
/// trusted to describe accepted request language, so the plan is pure data and
/// the capability declarations are closed enums of owned values.
#[test]
fn crate_sources_contain_no_function_pointers() {
    const SOURCES: &[(&str, &str)] = &[
        ("lib.rs", include_str!("../src/lib.rs")),
        ("plan.rs", include_str!("../src/plan.rs")),
        ("request.rs", include_str!("../src/request.rs")),
        ("id.rs", include_str!("../src/id.rs")),
        ("iri.rs", include_str!("../src/iri.rs")),
        ("error.rs", include_str!("../src/error.rs")),
        ("canonical.rs", include_str!("../src/canonical.rs")),
    ];
    for (name, source) in SOURCES {
        assert!(
            !source.contains("fn("),
            "{name} contains a function pointer; plans and capability declarations must be pure data"
        );
    }
}

// ---------------------------------------------------------------------------
// T6.6. A producer's candidate-domain declaration reaches the plan identity.
//
// A plan records the content fingerprint of the registry it was planned
// against, and the fingerprint folds in every declared field of every ranked
// declaration. The domain declaration is one of those fields, and it must be:
// two registries that differ only in what their producers may name FUSE
// DIFFERENTLY — one licenses a bounded read, the other does not — so a plan
// admitted against one must not be admissible against the other. The route is
// registry declaration → content fingerprint → plan bytes → plan id, and this
// test walks all of it rather than asserting the middle.
// ---------------------------------------------------------------------------

/// A one-producer registry whose declaration names `domains` and is otherwise
/// fixed, so the fingerprint difference below can come from nothing else.
fn registry_declaring(domains: CandidateDomains) -> PropertyFunctionRegistry {
    let mut registry = PropertyFunctionRegistry::new();
    registry.register_ranked(
        "http://example.org/ns#ranked",
        Arc::new(MemoryRelation::new(1, 1, Vec::new()).expect("an empty table is uniform")),
        RankedDeclaration {
            stratum: purrdf_core::parse_iri("http://example.org/stratum/a").expect("fixture IRI"),
            accepted_terms: Vec::new(),
            depth_placement: None,
            candidate_position: 0,
            duplicates: DuplicatePolicy::Unique,
            domains,
            block_position: None,
            mandatory: false,
        },
    );
    registry
}

/// The baseline plan, recording `registry`'s durable content fingerprint.
fn plan_against(registry: &PropertyFunctionRegistry) -> Plan {
    Plan {
        registry_content_fingerprint: registry
            .content_fingerprint()
            .expect("the fixture declarations are readable"),
        ..baseline()
    }
}

#[test]
fn a_declared_candidate_domain_moves_the_plan_identity() {
    let unrestricted = registry_declaring(CandidateDomains::Unrestricted);
    let restricted = registry_declaring(CandidateDomains::within([DomainTag::parse(
        "http://example.org/domain/documents",
    )
    .expect("fixture domain tag")]));

    assert_ne!(
        unrestricted.content_fingerprint().expect("readable"),
        restricted.content_fingerprint().expect("readable"),
        "the declaration reaches the registry's durable fingerprint"
    );
    assert_ne!(
        plan_against(&unrestricted).id(),
        plan_against(&restricted).id(),
        "and through it the plan's identity, so a plan admitted against one \
         wiring cannot run against a wiring that fuses differently"
    );

    // The valid neighbour, and the property that makes the identity usable: two
    // registries declaring the SAME domains plan to the same identity. An id
    // that moved between identical wirings would invalidate every cached plan
    // on every rebuild.
    let same = registry_declaring(CandidateDomains::within([DomainTag::parse(
        "http://example.org/domain/documents",
    )
    .expect("fixture domain tag")]));
    assert_eq!(
        plan_against(&restricted).id(),
        plan_against(&same).id(),
        "the identity is a function of what was declared, not of which registry \
         instance declared it"
    );

    // And two different restrictions are two different identities: the tags
    // themselves reach the digest, not merely the fact that a restriction
    // exists.
    let other_block = registry_declaring(CandidateDomains::within([DomainTag::parse(
        "http://example.org/domain/people",
    )
    .expect("fixture domain tag")]));
    assert_ne!(
        plan_against(&restricted).id(),
        plan_against(&other_block).id(),
        "the blocks are part of the declaration, so they are part of the identity"
    );
}

// ---------------------------------------------------------------------------
// The derivation record: round trip, identity sensitivity, and certification
// ---------------------------------------------------------------------------

/// An absent cardinality survives the encoding as an absence.
///
/// "The provider measured nothing" and "the provider measured zero" are
/// different facts about the data, and a decoder that read the first as the
/// second would turn silence into a claim that no row matches. Both are
/// executed, because a codec that collapsed them would still round-trip one of
/// them correctly.
#[test]
fn a_plan_round_trips_an_absent_cardinality() {
    for (label, recorded) in [
        ("absent", None),
        ("a measured zero", Some(0)),
        ("a count", Some(7)),
    ] {
        let mut plan = baseline();
        plan.statistics_snapshot.entries[0].cardinality = recorded;
        plan.stratum_derivations
            .get_mut(&stratum())
            .expect("the baseline records a derivation")
            .cardinality = recorded;

        let decoded =
            Plan::from_canonical_bytes(&plan.canonical_bytes()).expect("canonical decode");
        assert_eq!(
            decoded.statistics_snapshot.entries[0].cardinality, recorded,
            "{label} survives the entry encoding"
        );
        assert_eq!(
            decoded.stratum_derivations[&stratum()].cardinality,
            recorded,
            "{label} survives the derivation encoding"
        );
        assert_eq!(
            decoded.canonical_bytes(),
            plan.canonical_bytes(),
            "{label}: encode, decode and encode again is byte-identical"
        );
    }
}

/// An absent cardinality and a measured zero are different plans.
///
/// The distinction is only worth drawing if it reaches the identity: two plans
/// built against different evidence that shared one id would let a caller
/// comparing ids be told two different reads were the same one. This was not
/// expressible before the field could be absent.
#[test]
fn an_absent_cardinality_and_a_measured_zero_are_different_plans() {
    let mut absent = baseline();
    absent
        .stratum_derivations
        .get_mut(&stratum())
        .expect("derivation")
        .cardinality = None;

    let mut measured = absent.clone();
    measured
        .stratum_derivations
        .get_mut(&stratum())
        .expect("derivation")
        .cardinality = Some(0);

    assert_ne!(
        absent.id(),
        measured.id(),
        "a provider that measured nothing and one that measured no rows are \
         different evidence, so they are different plans"
    );
}

/// Two plans whose selectivity aggregates are equal but came from different
/// terms are different plans.
///
/// The aggregate is a sum, so the total alone cannot distinguish them — which
/// is the same silent-drift failure the derivation record exists to close, one
/// level down. Recording the domain is what makes the move detectable.
#[test]
fn equal_selectivity_sums_over_different_terms_are_different_plans() {
    let mut left = baseline();
    {
        let inputs = left
            .stratum_derivations
            .get_mut(&stratum())
            .expect("derivation");
        inputs.selectivity_ppm = Some(300_000);
        inputs.selectivity_terms = vec![0];
    }

    let mut right = left.clone();
    right
        .stratum_derivations
        .get_mut(&stratum())
        .expect("derivation")
        .selectivity_terms = vec![1];

    assert_eq!(
        left.stratum_derivations[&stratum()].selectivity_ppm,
        right.stratum_derivations[&stratum()].selectivity_ppm,
        "the aggregates are equal, which is the whole point"
    );
    assert_ne!(
        left.id(),
        right.id(),
        "a provider that moved the same total onto another term measured \
         something else, and the plan says so"
    );
}

/// The encoding is a pure function of the entries, not of the order a caller
/// built them in — and one subject twice is refused rather than sorted into an
/// arbitrary winner.
#[test]
fn statistics_entries_encode_by_subject_and_refuse_a_duplicate() {
    let second = StatisticsEntry {
        subject: "http://example.org/q".to_owned(),
        cardinality: Some(3),
        selectivity_ppm: None,
        selectivity_terms: Vec::new(),
    };

    let mut ascending = baseline();
    ascending.statistics_snapshot.entries.push(second.clone());
    let mut descending = baseline();
    descending.statistics_snapshot.entries.insert(0, second);

    assert_eq!(
        ascending.canonical_bytes(),
        descending.canonical_bytes(),
        "a hand-built snapshot must not give one plan many identities"
    );
    assert_eq!(ascending.id(), descending.id());

    // Two rows for one subject are two answers to one question, and sorting
    // cannot pick between them.
    let mut duplicated = baseline();
    let repeat = duplicated.statistics_snapshot.entries[0].clone();
    duplicated.statistics_snapshot.entries.push(repeat);
    let error = Plan::from_canonical_bytes(&duplicated.canonical_bytes())
        .expect_err("a repeated subject is refused");
    assert!(
        matches!(error, PlanError::DuplicateStatisticsSubject { .. }),
        "refused by name, not by a decode failure: {error:?}"
    );
}

/// A plan whose recorded depth does not follow from its recorded inputs is
/// refused, and one that does is admitted.
///
/// Both halves run. A checker that refused everything would pass the first
/// assertion alone, which is the mirror of the silent-drop bug: a refusal that
/// looks like strictness and rejects honest plans.
#[test]
fn certify_refuses_a_depth_its_inputs_do_not_derive() {
    baseline()
        .certify()
        .expect("the baseline's inputs derive its depth");

    let mut edited = baseline();
    edited.stratum_depths.insert(stratum(), 11);
    match edited.certify().expect_err("an edited depth is refused") {
        PlanError::DepthNotDerivable {
            recorded, derived, ..
        } => {
            assert_eq!(recorded, 11);
            assert_eq!(derived, 10, "the inputs still derive the honest depth");
        }
        other => panic!("refused by the wrong name: {other:?}"),
    }

    // A depth with no derivation, and a derivation with no depth, are different
    // edits and are refused separately.
    let mut orphan_depth = baseline();
    orphan_depth.stratum_derivations.clear();
    assert!(matches!(
        orphan_depth.certify(),
        Err(PlanError::DepthWithoutDerivation { .. })
    ));

    let mut orphan_derivation = baseline();
    orphan_derivation.stratum_depths.clear();
    assert!(matches!(
        orphan_derivation.certify(),
        Err(PlanError::DerivationWithoutDepth { .. })
    ));
}

/// A version-3 document is refused by name rather than reinterpreted under the
/// version-4 layout.
#[test]
fn a_version_three_document_is_refused_by_name() {
    let mut bytes = baseline().canonical_bytes();
    assert_eq!(
        &bytes[0..2],
        &PLAN_VERSION.to_le_bytes(),
        "the version is the encoding's first field, which is why a bump moves every plan"
    );
    bytes[0..2].copy_from_slice(&3u16.to_le_bytes());

    match Plan::from_canonical_bytes(&bytes).expect_err("version 3 is refused") {
        PlanError::VersionMismatch { found, expected } => {
            assert_eq!(found, 3);
            assert_eq!(expected, PLAN_VERSION);
        }
        other => panic!("a stale layout must be refused by name, not by a parse error: {other:?}"),
    }
}
