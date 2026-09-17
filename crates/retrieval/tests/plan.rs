// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The plan value's contract: equality, serialization, canonical stability,
//! digest sensitivity, decode round-trip and loud version refusal.

use std::collections::HashMap;

use pretty_assertions::assert_eq;
use purrdf_retrieval::{
    Fixed, Iri, Metric, PLAN_VERSION, Plan, PlanError, PlanOrigin, ProducerBinding,
    ProducerDecision, RegistryId, RejectionReason, RequestTerm, StatisticsEntry,
    StatisticsSnapshot, Term, UnservedReason, UnservedTerm,
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

    Plan {
        version: Plan::VERSION,
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
        statistics_snapshot: StatisticsSnapshot {
            source: "example-statistics".to_owned(),
            revision: "r1".to_owned(),
            entries: vec![StatisticsEntry {
                subject: "http://example.org/p".to_owned(),
                cardinality: 42,
                selectivity_ppm: Some(1_000),
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
    changed.statistics_snapshot.entries[0].cardinality = 43;
    assert_ne!(changed.id(), base_id, "statistics entry");

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
