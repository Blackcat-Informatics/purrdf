// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The plan value's contract: equality, serialization, canonical stability,
//! digest sensitivity, decode round-trip and loud version refusal.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use pretty_assertions::assert_eq;
use purrdf_retrieval::{
    CanonicalSection, DepthInputs, Fixed, Iri, Metric, PLAN_VERSION, Plan, PlanError, PlanOrigin,
    ProducerBinding, ProducerDecision, RankFidelity, ReadBound, RegistryId, RejectionReason,
    RequestTerm, StatisticsDimension, StatisticsEntries, StatisticsEntry, StatisticsSnapshot, Term,
    TopK, UnservedReason, UnservedTerm,
};
use purrdf_sparql_eval::{
    CandidateDomains, DomainTag, DuplicatePolicy, ExclusionBasis, MemoryRelation,
    PropertyFunctionRegistry, RankedDeclaration,
};

fn iri(text: &str) -> Iri {
    Iri::parse(text).expect("fixture IRIs are valid")
}

/// Rewrite a plan's snapshot rows through the entries' own construction law.
///
/// [`StatisticsEntries`] offers no mutable view, deliberately: an edit that
/// moved one row's subject would leave the sequence unordered or doubled, which
/// is the state the type exists to make unrepresentable. So an edit here is a
/// re-construction, and every fixture below is held to the same law a caller is.
fn edit_rows(plan: &mut Plan, edit: impl FnOnce(&mut Vec<StatisticsEntry>)) {
    let mut rows = plan.statistics_snapshot.entries.to_vec();
    edit(&mut rows);
    plan.statistics_snapshot.entries =
        StatisticsEntries::new(rows).expect("the edited rows name each subject once");
}

/// The canonical bytes of `plan` with one length-framed keyed section written in
/// `order` — including orders and repetitions the plan's own containers cannot
/// hold.
///
/// [`Plan::canonical_bytes`] cannot produce these, which is exactly why the
/// decoder's refusals have to be driven by bytes rather than by a value: a
/// `BTreeMap` and a [`StatisticsEntries`] both establish their order and refuse
/// a repeated key, so a forged document is the only remaining way in — and it is
/// a way in, because the decoder reads whatever a caller hands it.
///
/// The splice is layout-agnostic rather than a hand-written encoding. The
/// section is located by encoding the same plan with no items and with one: the
/// two agree up to the framed count and differ there, which names the count's
/// offset and hence the suffix. Each item's block is then cut out of a one-item
/// encoding of the same plan. Nothing here knows the plan's field order, so a
/// field added anywhere leaves it correct.
fn forge_section<T: Clone>(plan: &Plan, set: impl Fn(&mut Plan, Vec<T>), order: &[T]) -> Vec<u8> {
    let encode_with = |items: Vec<T>| {
        let mut forged = plan.clone();
        set(&mut forged, items);
        forged.canonical_bytes()
    };
    let empty = encode_with(Vec::new());
    let single = encode_with(vec![order.first().expect("an order names an item").clone()]);
    let count_offset = empty
        .iter()
        .zip(&single)
        .position(|(left, right)| left != right)
        .expect("the framed item count differs between no items and one");
    let suffix = &empty[count_offset + 8..];

    let mut bytes = empty[..count_offset].to_vec();
    bytes.extend_from_slice(
        &u64::try_from(order.len())
            .expect("a fixture order fits a u64")
            .to_le_bytes(),
    );
    for item in order {
        let block = encode_with(vec![item.clone()]);
        bytes.extend_from_slice(&block[count_offset + 8..block.len() - suffix.len()]);
    }
    bytes.extend_from_slice(suffix);
    bytes
}

/// [`forge_section`] over the statistics snapshot's entries.
fn forge_statistics_entries(plan: &Plan, order: &[StatisticsEntry]) -> Vec<u8> {
    forge_section(
        plan,
        |forged, rows| {
            forged.statistics_snapshot.entries = StatisticsEntries::new(rows)
                .expect("each spliced encoding carries at most one row");
        },
        order,
    )
}

/// [`forge_section`] over the per-stratum depths.
fn forge_stratum_depths(plan: &Plan, order: &[(Iri, u32)]) -> Vec<u8> {
    forge_section(
        plan,
        |forged, rows| forged.stratum_depths = rows.into_iter().collect(),
        order,
    )
}

/// [`forge_section`] over the per-stratum depth derivations.
fn forge_stratum_derivations(plan: &Plan, order: &[(Iri, DepthInputs)]) -> Vec<u8> {
    forge_section(
        plan,
        |forged, rows| forged.stratum_derivations = rows.into_iter().collect(),
        order,
    )
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
        // The snapshot names every subject planning consults: the request's own
        // predicate, and the stratum a depth was derived for. The stratum's row
        // is the projection of the derivation above — absent cardinality, absent
        // selectivity — and the predicate's row carries different values in every
        // field, so a test that mixed the two up fails on the number rather than
        // on the shape.
        statistics_snapshot: StatisticsSnapshot {
            source: "example-statistics".to_owned(),
            revision: "r1".to_owned(),
            entries: StatisticsEntries::new(vec![
                StatisticsEntry {
                    subject: "http://example.org/p".to_owned(),
                    cardinality: Some(42),
                    selectivity_ppm: Some(1_000),
                    selectivity_terms: vec![0],
                },
                StatisticsEntry {
                    subject: stratum().as_str().to_owned(),
                    cardinality: None,
                    selectivity_ppm: None,
                    selectivity_terms: Vec::new(),
                },
            ])
            .expect("the baseline names each subject once"),
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
    edit_rows(&mut changed, |rows| rows[0].cardinality = Some(43));
    assert_ne!(changed.id(), base_id, "statistics entry");

    // A statistic the plan was built against that the plan does not carry into
    // its identity is a statistic a replay cannot be held to, so every field of
    // the derivation record moves the id — including the term domain of the
    // selectivity, whose aggregate alone would be identical under a provider that
    // moved the same number to a different term.
    let mut changed = base.clone();
    edit_rows(&mut changed, |rows| rows[0].selectivity_terms = vec![1]);
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
            fidelity: RankFidelity::EXACT,
            domains,
            block_position: None,
            exclusion: ExclusionBasis::Unavailable,
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
        edit_rows(&mut plan, |rows| rows[0].cardinality = recorded);
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

/// A snapshot's entries are ordered by the **value**, so two plans built from
/// the same rows in different orders are one plan: equal bytes, equal id, and
/// equal to each other.
///
/// The last of those three is the assertion an encoder-side sort could not
/// support, and its absence was the biconditional [`Plan`] documents being false
/// in the middle — equal bytes and equal ids over values that compared unequal.
#[test]
fn statistics_entries_are_ordered_by_the_value_not_by_the_encoder() {
    // A row that sorts BETWEEN the baseline's two, so a construction that merely
    // reversed or appended would land it somewhere the ascending order does not
    // put it.
    let between = StatisticsEntry {
        subject: "http://example.org/q".to_owned(),
        cardinality: Some(3),
        selectivity_ppm: None,
        selectivity_terms: Vec::new(),
    };
    let rows = baseline().statistics_snapshot.entries.to_vec();

    let mut ascending = baseline();
    edit_rows(&mut ascending, |rows| rows.push(between.clone()));

    let mut descending = baseline();
    edit_rows(&mut descending, |rows| {
        rows.push(between.clone());
        rows.sort_by(|left, right| right.subject.cmp(&left.subject));
    });

    assert_eq!(
        ascending.canonical_bytes(),
        descending.canonical_bytes(),
        "a hand-built snapshot must not give one plan many identities"
    );
    assert_eq!(ascending.id(), descending.id());
    assert_eq!(
        ascending, descending,
        "and the two values are equal, not merely identical in their bytes: the \
         order is the type's law, so there is no order left for equality to see"
    );

    // The order is ascending rather than merely agreed-upon, and every row
    // survived it.
    let subjects: Vec<&str> = ascending
        .statistics_snapshot
        .entries
        .iter()
        .map(|entry| entry.subject.as_str())
        .collect();
    assert_eq!(
        subjects,
        vec![
            "http://example.org/p",
            "http://example.org/q",
            stratum().as_str(),
        ]
    );
    assert_eq!(
        ascending.statistics_snapshot.entries.len(),
        rows.len() + 1,
        "establishing the order is not an excuse to lose a row"
    );
}

/// One subject twice is refused at construction, by name — and a snapshot that
/// merely grows by a **distinct** subject is not.
///
/// Both halves run. A constructor that refused every second row would pass the
/// first assertion alone, and the second row below carries the first row's own
/// measurements under another subject, so a construction that dropped or merged
/// it fails on the number rather than on the shape.
#[test]
fn a_repeated_subject_is_refused_at_construction_and_a_distinct_one_is_not() {
    let rows = baseline().statistics_snapshot.entries.to_vec();

    let mut repeated = rows.clone();
    repeated.push(rows[0].clone());
    match StatisticsEntries::new(repeated).expect_err("a repeated subject is refused") {
        PlanError::DuplicateStatisticsSubject { subject } => {
            assert_eq!(subject, rows[0].subject, "the refusal names the subject");
        }
        other => panic!("refused by the wrong name: {other:?}"),
    }

    // The neighbour: the identical row under a subject nothing else names.
    let mut twin = rows[0].clone();
    twin.subject = "http://example.org/q".to_owned();
    let mut distinct = rows.clone();
    distinct.push(twin);
    let entries = StatisticsEntries::new(distinct).expect("distinct subjects are admitted");
    assert_eq!(entries.len(), rows.len() + 1);
    assert_eq!(
        entries.get_subject("http://example.org/q"),
        Some(&StatisticsEntry {
            subject: "http://example.org/q".to_owned(),
            cardinality: rows[0].cardinality,
            selectivity_ppm: rows[0].selectivity_ppm,
            selectivity_terms: rows[0].selectivity_terms.clone(),
        }),
        "and the admitted row keeps every measurement it arrived with"
    );
    assert_eq!(
        entries.get_subject("http://example.org/absent"),
        None,
        "a subject the snapshot does not name is absent rather than nearest"
    );
}

/// A canonical document that names one subject twice is refused by name.
///
/// No [`StatisticsEntries`] can hold such a snapshot, so the bytes are forged:
/// the decoder reads a document a caller controls, and "the value type cannot
/// express it" is not a check the decoder is entitled to skip.
#[test]
fn a_canonical_document_naming_one_subject_twice_is_refused() {
    let plan = baseline();
    let rows = plan.statistics_snapshot.entries.to_vec();

    let forged = forge_statistics_entries(&plan, &[rows[0].clone(), rows[0].clone()]);
    match Plan::from_canonical_bytes(&forged).expect_err("a repeated subject is refused") {
        PlanError::DuplicateStatisticsSubject { subject } => {
            assert_eq!(subject, rows[0].subject);
        }
        other => panic!("refused by the wrong name: {other:?}"),
    }

    // The neighbour: the same splice, with the plan's own two distinct rows in
    // ascending order, decodes to the plan the splice was cut from.
    let spliced = forge_statistics_entries(&plan, &rows);
    assert_eq!(
        spliced,
        plan.canonical_bytes(),
        "the splice reproduces the encoder's own bytes, so the refusal above is \
         about the duplicate rather than about the forging"
    );
    let decoded = Plan::from_canonical_bytes(&spliced).expect("an honest document decodes");
    assert_eq!(decoded, plan);
}

/// The baseline widened to two strata, every recorded number distinct.
///
/// Distinct declarations, distinct depths and distinct snapshot rows, so a
/// decoder that dropped one record, kept the wrong one of two, or mapped a value
/// onto the wrong stratum fails on a number rather than on a shape. It certifies
/// before it is returned, so every fixture below starts from a plan whose own
/// evidence supports it.
fn two_strata() -> Plan {
    let mut plan = baseline();
    let second = iri("http://example.org/stratum/b");
    plan.stratum_derivations.insert(
        second.clone(),
        DepthInputs {
            declared: 4,
            cardinality: Some(4),
            selectivity_ppm: None,
            selectivity_terms: Vec::new(),
            licensed_prefix: Some(25),
        },
    );
    plan.stratum_depths.insert(second.clone(), 4);
    edit_rows(&mut plan, |rows| {
        rows.push(StatisticsEntry {
            subject: second.as_str().to_owned(),
            cardinality: Some(4),
            selectivity_ppm: None,
            selectivity_terms: Vec::new(),
        });
    });
    plan.certify()
        .expect("the widened baseline is honest about both of its strata");
    plan
}

/// A canonical document whose subjects are not ascending is refused, and the
/// same splice in ascending order is the encoder's own bytes.
///
/// Both halves run. A decoder that refused every spliced document would pass the
/// first assertion alone — so the neighbour asserts byte equality with
/// [`Plan::canonical_bytes`], which proves the refusal is about the order rather
/// than about the forging.
#[test]
fn a_canonical_document_listing_subjects_out_of_order_is_refused() {
    let plan = two_strata();
    let rows = plan.statistics_snapshot.entries.to_vec();
    let mut reversed = rows.clone();
    reversed.reverse();

    match Plan::from_canonical_bytes(&forge_statistics_entries(&plan, &reversed))
        .expect_err("an unordered snapshot is refused")
    {
        PlanError::NonAscendingCanonicalKeys {
            section,
            previous,
            key,
        } => {
            assert_eq!(section, CanonicalSection::StatisticsEntries);
            assert_eq!(previous, rows[2].subject, "the key read before it");
            assert_eq!(key, rows[1].subject, "and the key that did not follow it");
        }
        other => panic!("refused by the wrong name: {other:?}"),
    }

    let honest = forge_statistics_entries(&plan, &rows);
    assert_eq!(
        honest,
        plan.canonical_bytes(),
        "the ascending splice is the encoder's own document"
    );
    let decoded = Plan::from_canonical_bytes(&honest).expect("an ascending document decodes");
    assert_eq!(decoded.statistics_snapshot.entries.to_vec(), rows);
}

/// A canonical document that repeats or reorders a stratum derivation is
/// refused, and the same splice in ascending order decodes to the plan it was
/// cut from.
#[test]
fn a_canonical_document_repeating_or_reordering_a_derivation_is_refused() {
    let plan = two_strata();
    let ordered: Vec<(Iri, DepthInputs)> = plan
        .stratum_derivations
        .iter()
        .map(|(stratum, inputs)| (stratum.clone(), inputs.clone()))
        .collect();
    let mut reversed = ordered.clone();
    reversed.reverse();

    match Plan::from_canonical_bytes(&forge_stratum_derivations(&plan, &reversed))
        .expect_err("unordered derivations are refused")
    {
        PlanError::NonAscendingCanonicalKeys {
            section,
            previous,
            key,
        } => {
            assert_eq!(section, CanonicalSection::StratumDerivations);
            assert_eq!(previous, ordered[1].0.as_str());
            assert_eq!(key, ordered[0].0.as_str());
        }
        other => panic!("refused by the wrong name: {other:?}"),
    }

    let repeated = [ordered[0].clone(), ordered[0].clone()];
    match Plan::from_canonical_bytes(&forge_stratum_derivations(&plan, &repeated))
        .expect_err("two derivations of one stratum are refused")
    {
        PlanError::DuplicateStratumDerivation { stratum } => {
            assert_eq!(stratum, ordered[0].0.as_str());
        }
        other => panic!("refused by the wrong name: {other:?}"),
    }

    // The dimensions are told apart. The same stratum repeated in the SNAPSHOT
    // is refused as a repeated subject, and only there — so a reader is sent to
    // the record that is actually doubled rather than to the plan's other record
    // of the same stratum, which is fine in both documents.
    let rows = plan.statistics_snapshot.entries.to_vec();
    let stratum_row = rows
        .iter()
        .find(|row| row.subject == ordered[0].0.as_str())
        .expect("the snapshot names the stratum a depth was derived for")
        .clone();
    match Plan::from_canonical_bytes(&forge_statistics_entries(
        &plan,
        &[stratum_row.clone(), stratum_row],
    ))
    .expect_err("two snapshot rows for one subject are refused")
    {
        PlanError::DuplicateStatisticsSubject { subject } => {
            assert_eq!(subject, ordered[0].0.as_str());
        }
        other => panic!("refused by the wrong name: {other:?}"),
    }

    // The neighbour: two derivations for two strata, ascending. Their
    // declarations differ, so a decoder that kept one twice or mapped one onto
    // the other fails on the number.
    let honest = forge_stratum_derivations(&plan, &ordered);
    assert_eq!(honest, plan.canonical_bytes());
    let decoded = Plan::from_canonical_bytes(&honest).expect("ascending derivations decode");
    assert_eq!(decoded.stratum_derivations, plan.stratum_derivations);
    assert_eq!(decoded.stratum_derivations[&ordered[0].0].declared, 10);
    assert_eq!(decoded.stratum_derivations[&ordered[1].0].declared, 4);
}

/// A canonical document that repeats or reorders a stratum depth is refused, and
/// the same splice in ascending order decodes to the plan it was cut from.
///
/// The repeat is the sharper of the two: the `HashMap` the decoder fills would
/// have kept whichever depth arrived last, so a forged document could have
/// chosen how deep a stratum is read while the plan beside it said otherwise.
#[test]
fn a_canonical_document_repeating_or_reordering_a_depth_is_refused() {
    let plan = two_strata();
    let mut ordered: Vec<(Iri, u32)> = plan
        .stratum_depths
        .iter()
        .map(|(stratum, depth)| (stratum.clone(), *depth))
        .collect();
    ordered.sort_by(|left, right| left.0.cmp(&right.0));
    let mut reversed = ordered.clone();
    reversed.reverse();

    match Plan::from_canonical_bytes(&forge_stratum_depths(&plan, &reversed))
        .expect_err("unordered depths are refused")
    {
        PlanError::NonAscendingCanonicalKeys {
            section,
            previous,
            key,
        } => {
            assert_eq!(section, CanonicalSection::StratumDepths);
            assert_eq!(previous, ordered[1].0.as_str());
            assert_eq!(key, ordered[0].0.as_str());
        }
        other => panic!("refused by the wrong name: {other:?}"),
    }

    // Two depths for one stratum, and they differ — so the map would have
    // silently answered with the second.
    let repeated = [
        (ordered[0].0.clone(), ordered[0].1),
        (ordered[0].0.clone(), ordered[0].1 + 5),
    ];
    match Plan::from_canonical_bytes(&forge_stratum_depths(&plan, &repeated))
        .expect_err("two depths for one stratum are refused")
    {
        PlanError::DuplicateStratumDepth { stratum } => {
            assert_eq!(stratum, ordered[0].0.as_str());
        }
        other => panic!("refused by the wrong name: {other:?}"),
    }

    let honest = forge_stratum_depths(&plan, &ordered);
    assert_eq!(honest, plan.canonical_bytes());
    let decoded = Plan::from_canonical_bytes(&honest).expect("ascending depths decode");
    assert_eq!(decoded.stratum_depths, plan.stratum_depths);
    assert_eq!(decoded.stratum_depths[&ordered[0].0], 10);
    assert_eq!(decoded.stratum_depths[&ordered[1].0], 4);
}

/// [`baseline`] with `terms` as the stratum's recorded selectivity domain, in
/// **both** of the plan's records of it.
///
/// Both, because they are compared to each other: a fixture that moved one would
/// be refused for the disagreement rather than for the run, and every assertion
/// below would be reading the wrong refusal. The baseline's stratum records no
/// selectivity, so no run of indices changes the depth its inputs derive — which
/// is what makes an in-range run a plan that still certifies.
fn baseline_with_terms(terms: Vec<u32>) -> Plan {
    let mut plan = baseline();
    plan.stratum_derivations
        .get_mut(&stratum())
        .expect("the baseline derives a depth for its stratum")
        .selectivity_terms
        .clone_from(&terms);
    let row = row_of(&plan, stratum().as_str());
    edit_rows(&mut plan, |rows| rows[row].selectivity_terms = terms);
    plan
}

/// A selectivity-term run that does not strictly ascend is not an encoding of
/// any plan, and the same indices ascending are.
///
/// The run is the domain of a **sum**, so its order says nothing about the data
/// and `[1, 0]` would otherwise be a second encoding of the plan `[0, 1]` is the
/// encoding of — two documents digesting to one id. The repeat is the other
/// clause and is refused by its own name: one term counted twice into a total
/// the arithmetic reached once.
///
/// Both of the plan's records of a run are exercised, because they are written
/// by two different encoders: the derivation's run, and a snapshot row that
/// derives no depth at all. The valid neighbour is the same two indices in
/// order, asserted to arrive as `[0, 1]` rather than merely to arrive — a
/// decoder that silently sorted or dropped the run would pass a bare `is_ok`.
#[test]
fn a_canonical_document_whose_selectivity_terms_do_not_ascend_is_refused() {
    match Plan::from_canonical_bytes(&baseline_with_terms(vec![1, 0]).canonical_bytes())
        .expect_err("a descending selectivity-term run is refused")
    {
        PlanError::NonAscendingSelectivityTerms {
            section,
            subject,
            previous,
            request_term,
        } => {
            assert_eq!(section, CanonicalSection::StratumDerivations);
            assert_eq!(subject, stratum().as_str(), "the record that carries it");
            assert_eq!(previous, 1, "the index read before it");
            assert_eq!(request_term, 0, "and the index that did not follow it");
        }
        other => panic!("refused by the wrong name: {other:?}"),
    }

    match Plan::from_canonical_bytes(&baseline_with_terms(vec![0, 0]).canonical_bytes())
        .expect_err("one term counted twice is refused")
    {
        PlanError::DuplicateSelectivityTerm {
            section,
            subject,
            request_term,
        } => {
            assert_eq!(section, CanonicalSection::StratumDerivations);
            assert_eq!(subject, stratum().as_str());
            assert_eq!(request_term, 0);
        }
        other => panic!("refused by the wrong name: {other:?}"),
    }

    // The snapshot's own encoder, over a row that derives no depth — so this
    // document is refused for the run rather than for a disagreement with a
    // derivation it has none of.
    let predicate_row = row_of(&baseline(), "http://example.org/p");
    let mut ancillary = baseline();
    edit_rows(&mut ancillary, |rows| {
        rows[predicate_row].selectivity_terms = vec![2, 1];
    });
    match Plan::from_canonical_bytes(&ancillary.canonical_bytes())
        .expect_err("a descending run on a snapshot row is refused")
    {
        PlanError::NonAscendingSelectivityTerms {
            section,
            subject,
            previous,
            request_term,
        } => {
            assert_eq!(section, CanonicalSection::StatisticsEntries);
            assert_eq!(subject, "http://example.org/p");
            assert_eq!((previous, request_term), (2, 1));
        }
        other => panic!("refused by the wrong name: {other:?}"),
    }

    // The neighbour: the same two indices, ascending. It decodes, it arrives
    // unchanged in both records, and it certifies.
    let ascending = baseline_with_terms(vec![0, 1]);
    let decoded = Plan::from_canonical_bytes(&ascending.canonical_bytes())
        .expect("an ascending run is an encoding of a plan");
    assert_eq!(
        decoded.stratum_derivations[&stratum()].selectivity_terms,
        vec![0, 1],
        "the run arrives as it was written, rather than sorted or emptied"
    );
    assert_eq!(
        decoded.statistics_snapshot.entries[row_of(&ascending, stratum().as_str())]
            .selectivity_terms,
        vec![0, 1]
    );
    assert_eq!(decoded, ascending, "and the document round-trips whole");
    decoded
        .certify()
        .expect("an ascending, addressable run certifies");
}

/// A recorded selectivity term that addresses no term of the plan's own request
/// is refused by name, and the last index that does address one is not.
///
/// The run is written as indices into the plan's request, so an index past the
/// end names nothing and the aggregate's domain becomes unreadable. The check
/// lives on [`Plan::certify`] rather than at the decoder because it is the
/// relation between two of the plan's fields, not a property of one section's
/// bytes.
///
/// The over-refusal guard is the boundary itself: four terms, so index 3 is
/// legal and index 4 is not, and both are executed over the same fixture. An
/// **empty** run is legal in both, which is the case a subject whose selectivity
/// nothing contributed to records — refusing it would refuse the plan the
/// planner writes for every silent provider.
#[test]
fn certify_refuses_a_selectivity_term_that_addresses_no_request_term() {
    assert_eq!(
        baseline().request_terms.len(),
        4,
        "the boundary the two halves below straddle is this number"
    );

    for forged in [4, 99] {
        match baseline_with_terms(vec![forged])
            .certify()
            .expect_err("a run addressing no term of the request is refused")
        {
            PlanError::SelectivityTermOutOfRange {
                subject,
                request_term,
                request_terms,
            } => {
                assert_eq!(subject, stratum().as_str());
                assert_eq!(request_term, forged);
                assert_eq!(request_terms, 4, "and the message says what it counted");
            }
            other => panic!("refused by the wrong name: {other:?}"),
        }
    }

    // The neighbour, one index below the first refused one: it addresses the
    // request's last term, so it certifies.
    baseline_with_terms(vec![3])
        .certify()
        .expect("the last index that names a term is not a forgery");
    baseline_with_terms(vec![0, 3])
        .certify()
        .expect("nor is a run of two of them");
    // And the empty run, which is what every subject nothing contributed a
    // selectivity to records.
    baseline_with_terms(Vec::new())
        .certify()
        .expect("an empty run addresses nothing and is the planner's own common case");

    // A snapshot row that derives no depth is held to the same law, and named by
    // its own subject.
    let predicate_row = row_of(&baseline(), "http://example.org/p");
    let mut forged_row = baseline();
    edit_rows(&mut forged_row, |rows| {
        rows[predicate_row].selectivity_terms = vec![4];
    });
    match forged_row
        .certify()
        .expect_err("a snapshot row addressing no term is refused")
    {
        PlanError::SelectivityTermOutOfRange {
            subject,
            request_term,
            request_terms,
        } => {
            assert_eq!(subject, "http://example.org/p");
            assert_eq!((request_term, request_terms), (4, 4));
        }
        other => panic!("refused by the wrong name: {other:?}"),
    }
    let mut ancillary = baseline();
    edit_rows(&mut ancillary, |rows| {
        rows[predicate_row].selectivity_terms = vec![3];
    });
    ancillary
        .certify()
        .expect("the same row one index lower addresses the request's last term");

    // A request carrying no terms at all: every run is empty, every empty run
    // addresses nothing, and the plan certifies. A rule that refused a run field
    // rather than an unaddressable index would fail here. The request predicate's
    // row goes with the request — with no terms there is no predicate, so that
    // row would name a subject nothing consulted.
    let mut termless = baseline_with_terms(Vec::new());
    termless.request_terms.clear();
    let predicate_row = row_of(&termless, "http://example.org/p");
    edit_rows(&mut termless, |rows| {
        rows.remove(predicate_row);
    });
    termless
        .certify()
        .expect("a request with no terms records empty runs, which address nothing");
}

/// A hand-written serde document is held to the entries' construction law: a
/// repeated subject is refused, and an unordered one is ordered.
///
/// The two halves are the law's two clauses. Order carries no information the
/// snapshot did not already have, so an unordered document is admitted and
/// canonicalised — and it must land on the *same identity* as the ordered one,
/// or serde would be a way to mint a second id for one plan. A duplicate is
/// information the value cannot hold at all, so it is refused wherever it
/// arrives.
#[test]
fn serde_holds_a_hand_written_snapshot_to_the_entries_law() {
    let plan = baseline();
    let mut document: serde_json::Value =
        serde_json::to_value(&plan).expect("a plan serializes to a document");

    let entries = document["statistics_snapshot"]["entries"]
        .as_array()
        .expect("the snapshot's entries are a JSON array")
        .clone();
    assert_eq!(entries.len(), 2, "the baseline names two subjects");

    let mut reversed = entries.clone();
    reversed.reverse();
    document["statistics_snapshot"]["entries"] = serde_json::Value::Array(reversed.clone());
    let decoded: Plan =
        serde_json::from_value(document.clone()).expect("an unordered document is ordered");
    assert_eq!(
        decoded.statistics_snapshot.entries,
        plan.statistics_snapshot.entries
    );
    assert_eq!(
        decoded.id(),
        plan.id(),
        "serde is not a second way to mint an identity for one plan"
    );

    let mut duplicated = entries.clone();
    duplicated.push(entries[0].clone());
    document["statistics_snapshot"]["entries"] = serde_json::Value::Array(duplicated);
    let error = serde_json::from_value::<Plan>(document)
        .expect_err("a document naming one subject twice is refused");
    assert!(
        error.to_string().contains("more than once"),
        "the serde failure carries the construction law's own refusal: {error}"
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

/// The index of the baseline snapshot row for a subject, so an edit names the
/// row it means rather than a position that moves when a row is added.
fn row_of(plan: &Plan, subject: &str) -> usize {
    plan.statistics_snapshot
        .entries
        .iter()
        .position(|entry| entry.subject == subject)
        .unwrap_or_else(|| panic!("the baseline names {subject}"))
}

/// A stratum whose snapshot row contradicts its derivation is refused, naming
/// the dimension that disagreed and both values — and the same edit to a row
/// that explains no depth changes nothing.
///
/// Both halves run on every dimension. The refusal alone would pass for a
/// checker that rejected any snapshot at all, and a request predicate's row is
/// exactly the case such a checker would wrongly reject: it derives no depth, so
/// there is nothing for it to contradict, and a host is free to record whatever
/// the provider told it there.
#[test]
fn certify_refuses_a_snapshot_row_that_contradicts_its_derivation() {
    let stratum_row = row_of(&baseline(), stratum().as_str());
    let predicate_row = row_of(&baseline(), "http://example.org/p");

    // Each edit moves ONE dimension, to a value the derivation does not carry.
    // The derivation records `None`, `None` and `[]`, so every replacement below
    // is a different statement about what the provider said.
    let mut wrong_cardinality = baseline();
    edit_rows(&mut wrong_cardinality, |rows| {
        rows[stratum_row].cardinality = Some(7);
    });
    let mut wrong_selectivity = baseline();
    edit_rows(&mut wrong_selectivity, |rows| {
        rows[stratum_row].selectivity_ppm = Some(250_000);
    });
    let mut wrong_terms = baseline();
    edit_rows(&mut wrong_terms, |rows| {
        rows[stratum_row].selectivity_terms = vec![0, 2];
    });

    // The neighbours: the identical three edits, to the request predicate's row.
    let mut ancillary_cardinality = baseline();
    edit_rows(&mut ancillary_cardinality, |rows| {
        rows[predicate_row].cardinality = Some(7);
    });
    let mut ancillary_selectivity = baseline();
    edit_rows(&mut ancillary_selectivity, |rows| {
        rows[predicate_row].selectivity_ppm = Some(250_000);
    });
    let mut ancillary_terms = baseline();
    edit_rows(&mut ancillary_terms, |rows| {
        rows[predicate_row].selectivity_terms = vec![0, 2];
    });

    for (forged, ancillary, dimension, expected_snapshot, expected_derivation) in [
        (
            wrong_cardinality,
            ancillary_cardinality,
            StatisticsDimension::Cardinality,
            "7",
            "absent",
        ),
        (
            wrong_selectivity,
            ancillary_selectivity,
            StatisticsDimension::SelectivityPpm,
            "250000",
            "absent",
        ),
        (
            wrong_terms,
            ancillary_terms,
            StatisticsDimension::SelectivityTerms,
            "[0, 2]",
            "[]",
        ),
    ] {
        match forged
            .certify()
            .expect_err("a snapshot row that contradicts its derivation is refused")
        {
            PlanError::StatisticsEntryContradictsDerivation {
                stratum: named,
                dimension: reported,
                snapshot,
                derivation,
            } => {
                assert_eq!(named, stratum().as_str());
                assert_eq!(
                    reported, dimension,
                    "the refusal names the failed dimension"
                );
                assert_eq!(snapshot, expected_snapshot);
                assert_eq!(
                    derivation, expected_derivation,
                    "and carries both values, so the message says what disagreed"
                );
            }
            other => panic!("refused by the wrong name: {other:?}"),
        }

        // The neighbour, carrying the identical edit on the request predicate's
        // row. It explains no depth, so there is no second record for it to
        // contradict and the plan still certifies.
        ancillary
            .certify()
            .expect("a row that derives no depth contradicts nothing");
    }

    // And the plan whose two records agree — which is what the planner writes —
    // certifies untouched, so the three refusals above are decisions rather than
    // the absence of one.
    baseline()
        .certify()
        .expect("the baseline's two records of its stratum say the same thing");
}

/// A stratum the snapshot does not name at all is refused, and dropping a row
/// that explains no depth is not.
///
/// The two are different facts: one is a snapshot that forgot the subject a
/// depth was derived for, the other a host recording less ancillary context than
/// the planner would have. Only the first makes a recorded depth unexplainable.
#[test]
fn certify_refuses_a_stratum_the_snapshot_does_not_name() {
    let stratum_row = row_of(&baseline(), stratum().as_str());
    let mut missing = baseline();
    edit_rows(&mut missing, |rows| {
        rows.remove(stratum_row);
    });
    match missing
        .certify()
        .expect_err("a derivation with no snapshot row is refused")
    {
        PlanError::DerivationWithoutStatisticsEntry { stratum: named } => {
            assert_eq!(named, stratum().as_str());
        }
        other => panic!("refused by the wrong name: {other:?}"),
    }

    // The neighbour: drop the request predicate's row instead. Nothing was
    // derived from it, so nothing is left unexplained and the plan certifies.
    let predicate_row = row_of(&baseline(), "http://example.org/p");
    let mut thinner = baseline();
    edit_rows(&mut thinner, |rows| {
        rows.remove(predicate_row);
    });
    thinner
        .certify()
        .expect("a snapshot without an ancillary row still explains every depth");

    // And an empty snapshot is refused for the stratum, not waved through as
    // "nothing to compare against".
    let mut empty = baseline();
    edit_rows(&mut empty, Vec::clear);
    assert!(matches!(
        empty.certify(),
        Err(PlanError::DerivationWithoutStatisticsEntry { .. })
    ));
}

/// A snapshot row for a subject nothing consulted is refused, and a row for a
/// request predicate the baseline does not already carry is not.
///
/// The mirror of the test above, and the direction a forger would use: a row can
/// be ADDED as easily as removed, and the plan then reads back as evidence about
/// a consultation that never happened. The snapshot's own doc says a subject
/// nothing consulted is absent rather than recorded as empty; this is what makes
/// that a checked claim.
///
/// The over-refusal guard is the whole legitimate set, exercised row by row. A
/// stratum's row and the request's first predicate are already in the baseline,
/// so the neighbour here is the request's *other* predicate — the spatial term's
/// — added with values no existing row carries. A rule tighter than "every
/// stratum, plus every request-term predicate" rejects that row, which is a row
/// the planner itself writes.
#[test]
fn certify_refuses_a_snapshot_row_for_a_subject_nothing_consulted() {
    let predicate_row = row_of(&baseline(), "http://example.org/p");

    // The forge: the request predicate's row renamed to a subject of equal
    // length that no term of this request names. The order is re-established on
    // construction, so the document is refused for the subject rather than for a
    // sequence the rename disturbed.
    let mut ghost = baseline();
    edit_rows(&mut ghost, |rows| {
        rows[predicate_row].subject = "http://example.org/q".to_owned();
    });
    match ghost
        .certify()
        .expect_err("a row naming a subject nothing consulted is refused")
    {
        PlanError::UnconsultedStatisticsSubject { subject } => {
            assert_eq!(subject, "http://example.org/q");
        }
        other => panic!("refused by the wrong name: {other:?}"),
    }

    // A term that names no predicate contributes no subject. The baseline's
    // fourth term is an entity seed over `http://example.org/e`, and a row for
    // that IRI is a row for something nothing was ever asked about.
    let mut seeded = baseline();
    edit_rows(&mut seeded, |rows| {
        rows[predicate_row].subject = "http://example.org/e".to_owned();
    });
    match seeded
        .certify()
        .expect_err("an entity seed is not a subject a provider was consulted for")
    {
        PlanError::UnconsultedStatisticsSubject { subject } => {
            assert_eq!(subject, "http://example.org/e");
        }
        other => panic!("refused by the wrong name: {other:?}"),
    }

    // The neighbour: the request's OTHER predicate, which the baseline's snapshot
    // does not name. It is a legitimate consultation, so adding it certifies —
    // and its values differ from both existing rows, so a check that had waved
    // through a row it recognised by its numbers could not pass this.
    let mut widened = baseline();
    edit_rows(&mut widened, |rows| {
        rows.push(StatisticsEntry {
            subject: "http://example.org/geo".to_owned(),
            cardinality: Some(11),
            selectivity_ppm: Some(3),
            selectivity_terms: vec![2],
        });
    });
    widened
        .certify()
        .expect("a predicate this request names is a subject planning consulted");
    assert_eq!(
        widened.statistics_snapshot.entries.len(),
        3,
        "the row really was added rather than collapsed into one of the others"
    );

    // And the baseline itself, whose two rows are the stratum and the request's
    // first predicate — one of each legitimate kind.
    baseline()
        .certify()
        .expect("a stratum's row and a request predicate's row are both consultations");
}

/// Which stratum a refusal names is a function of the plan, never of hash order.
///
/// `stratum_depths` is a `HashMap`, so a checker walking it in its own iteration
/// order picks an arbitrary one of a plan's several disagreements, and two
/// processes refusing one forged plan could name two different strata. Thirty-two
/// strata all disagree here and the refusal must name the lexicographically first.
///
/// # Why the plan is rebuilt every round
///
/// One `HashMap` walked once proves nothing: a single map's order is fixed, and
/// it can perfectly well begin at the key the assertion expects — asserting over
/// one is a test that passes against the very bug it names, which is exactly how
/// a laundered oracle looks. The standard library seeds each `HashMap` from a
/// per-thread counter, so a fresh map of the same thirty-two keys is a fresh
/// order. Sixty-four rounds of an order-dependent walk agreeing on one key out of
/// thirty-two is a coincidence of about one in `32^64`; the sorted walk agrees
/// every time by construction.
///
/// The valid neighbour is the same thirty-two-stratum plan with honest depths,
/// which certifies: the ordering fix must not turn a wide plan into a refusal.
#[test]
fn certify_names_the_first_disagreement_in_stratum_order() {
    let wide = |honest: bool| {
        let mut plan = baseline();
        plan.stratum_depths.clear();
        plan.stratum_derivations.clear();
        let mut rows: Vec<StatisticsEntry> = Vec::new();
        for index in 0..32_u32 {
            let stratum = iri(&format!("http://example.org/stratum/{index:02}"));
            // Distinct declarations, so each stratum's honest depth is its own
            // number and a record read off the wrong stratum fails on the value.
            let declared = u64::from(index) + 1;
            plan.stratum_derivations.insert(
                stratum.clone(),
                DepthInputs {
                    declared,
                    cardinality: None,
                    selectivity_ppm: None,
                    selectivity_terms: Vec::new(),
                    licensed_prefix: None,
                },
            );
            let depth = u32::try_from(declared).expect("the fixture declarations fit a rank");
            plan.stratum_depths
                .insert(stratum.clone(), if honest { depth } else { depth + 1 });
            rows.push(StatisticsEntry {
                subject: stratum.as_str().to_owned(),
                cardinality: None,
                selectivity_ppm: None,
                selectivity_terms: Vec::new(),
            });
        }
        edit_rows(&mut plan, |existing| *existing = rows);
        plan
    };

    for round in 0..64 {
        match wide(false)
            .certify()
            .expect_err("every one of the thirty-two depths is wrong")
        {
            PlanError::DepthNotDerivable {
                stratum,
                recorded,
                derived,
            } => {
                assert_eq!(
                    stratum, "http://example.org/stratum/00",
                    "round {round} named another stratum, so which disagreement is \
                     reported depends on hash order rather than on the plan"
                );
                assert_eq!(
                    (recorded, derived),
                    (2, 1),
                    "and the values are that stratum's own, not another's"
                );
            }
            other => panic!("refused by the wrong name: {other:?}"),
        }
    }

    wide(true)
        .certify()
        .expect("thirty-two honest strata certify; the ordering rule refuses nothing");
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

/// Every refusal names itself, and no two refusals name the same thing.
///
/// [`PlanError::refusal`] is what a caller branches on — a host receiving plans
/// from somewhere it does not control, a binding rendering them into its own
/// vocabulary — so two variants sharing a name would make two different facts
/// indistinguishable at exactly the moment the caller is trying to tell them
/// apart, and a copy-pasted arm is how that happens. The exhaustive match in
/// `refusal` already makes an *unnamed* variant a compile error; this is the
/// other half.
#[test]
fn every_plan_refusal_has_its_own_pinned_name() {
    let stratum = || "https://example.org/stratum/text".to_owned();
    let refusals = [
        PlanError::VersionMismatch {
            found: 3,
            expected: PLAN_VERSION,
        },
        PlanError::NoApplicableProducers,
        PlanError::InvalidRequestTerm {
            term: Box::new(RequestTerm::Lexical {
                text: "quick".to_owned(),
                language: None,
                predicate: None,
            }),
            reason: "empty".to_owned(),
        },
        PlanError::ReadBoundBeyondDepthRange {
            requested: 1,
            ceiling: 2,
        },
        PlanError::RegistryDeclaration {
            message: "panicked".to_owned(),
        },
        PlanError::Truncated { offset: 0 },
        PlanError::InvalidTag {
            what: "metric",
            tag: 9,
        },
        PlanError::InvalidUtf8 { what: "producer" },
        Iri::parse("not an iri").expect_err("a malformed IRI is refused"),
        PlanError::TrailingBytes { extra: 1 },
        PlanError::UndeclaredRowBound {
            stratum: stratum(),
            producer: "https://example.org/pf/text".to_owned(),
        },
        PlanError::DuplicateStatisticsSubject { subject: stratum() },
        PlanError::DuplicateStratumDerivation { stratum: stratum() },
        PlanError::DuplicateStratumDepth { stratum: stratum() },
        PlanError::NonAscendingCanonicalKeys {
            section: CanonicalSection::StratumDepths,
            previous: stratum(),
            key: stratum(),
        },
        PlanError::NonAscendingSelectivityTerms {
            section: CanonicalSection::StratumDerivations,
            subject: stratum(),
            previous: 1,
            request_term: 0,
        },
        PlanError::DuplicateSelectivityTerm {
            section: CanonicalSection::StatisticsEntries,
            subject: stratum(),
            request_term: 0,
        },
        PlanError::SelectivityTermOutOfRange {
            subject: stratum(),
            request_term: 4,
            request_terms: 4,
        },
        PlanError::UnconsultedStatisticsSubject { subject: stratum() },
        PlanError::DepthNotDerivable {
            stratum: stratum(),
            recorded: 2,
            derived: 1,
        },
        PlanError::DepthWithoutDerivation { stratum: stratum() },
        PlanError::DerivationWithoutDepth { stratum: stratum() },
        PlanError::DerivationWithoutStatisticsEntry { stratum: stratum() },
        PlanError::StatisticsEntryContradictsDerivation {
            stratum: stratum(),
            dimension: StatisticsDimension::Cardinality,
            snapshot: "2".to_owned(),
            derivation: "3".to_owned(),
        },
    ];

    let mut seen: BTreeMap<&'static str, String> = BTreeMap::new();
    for refusal in &refusals {
        let name = refusal.refusal();
        assert!(
            !name.is_empty()
                && name
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-'),
            "a refusal name is pinned, machine-readable and kebab-case; {name:?} is not"
        );
        if let Some(previous) = seen.insert(name, format!("{refusal}")) {
            panic!(
                "two refusals both name themselves {name:?}: {previous} — and {refusal}; a caller \
                 branching on the name cannot tell them apart"
            );
        }
    }
    assert_eq!(
        seen.len(),
        refusals.len(),
        "every refusal above contributes its own name"
    );
}

// ---------------------------------------------------------------------------
// The bytes themselves, written out by hand
// ---------------------------------------------------------------------------

/// A complete plan, small enough that every byte of its encoding is spelled out
/// below.
///
/// Every number in it is distinct, so a byte read off the wrong field fails on
/// the value rather than passing under a coincidence: the declaration is eight
/// rows, the measured cardinality four, the licensed prefix three, the recorded
/// depth two, the request's bound three, the registry counter seven. The
/// selectivity is half a part in two — `500_000` parts per million, `0x0007_a120`
/// — which is the one multi-byte value here and so the one that would expose a
/// byte order written the wrong way round.
///
/// It carries, deliberately, every shape the pinned literal has to cover: a
/// stratum whose cardinality is **present**, a second statistics subject whose
/// cardinality is **absent**, a present selectivity with a non-empty
/// `selectivity_terms` run beside it, an absent selectivity with an empty run,
/// and one stratum derivation. It certifies: the depth of two is what
/// `depth_from` derives from the inputs recorded beside it, and the stratum's
/// snapshot row is the projection of that derivation.
fn pinned_plan() -> Plan {
    let subject = iri("http://example.org/s");
    let mut stratum_depths = HashMap::new();
    stratum_depths.insert(subject.clone(), 2);
    let mut stratum_derivations = BTreeMap::new();
    stratum_derivations.insert(
        subject.clone(),
        DepthInputs {
            declared: 8,
            cardinality: Some(4),
            selectivity_ppm: Some(500_000),
            selectivity_terms: vec![0],
            licensed_prefix: Some(3),
        },
    );

    Plan {
        version: Plan::VERSION,
        request_terms: vec![RequestTerm::Lexical {
            text: "q".to_owned(),
            language: None,
            predicate: Some(iri("http://example.org/p")),
        }],
        read_bound: ReadBound::Bounded(TopK::new(3)),
        producer_bindings: vec![ProducerBinding {
            producer: "http://example.org/pf".to_owned(),
            stratum: subject.clone(),
            request_terms: vec![0],
        }],
        producer_decisions: vec![ProducerDecision::Selected {
            producer: "http://example.org/pf".to_owned(),
            stratum: subject,
        }],
        unserved_terms: Vec::new(),
        stratum_depths,
        stratum_derivations,
        statistics_snapshot: StatisticsSnapshot {
            source: "host".to_owned(),
            revision: "r1".to_owned(),
            entries: StatisticsEntries::new(vec![
                // The request predicate: consulted, and the provider said
                // nothing about it in either dimension.
                StatisticsEntry {
                    subject: "http://example.org/p".to_owned(),
                    cardinality: None,
                    selectivity_ppm: None,
                    selectivity_terms: Vec::new(),
                },
                // The stratum: the projection of the derivation above.
                StatisticsEntry {
                    subject: "http://example.org/s".to_owned(),
                    cardinality: Some(4),
                    selectivity_ppm: Some(500_000),
                    selectivity_terms: vec![0],
                },
            ])
            .expect("the pinned plan names each subject once"),
        },
        registry_instance_id: RegistryId::from_raw(7),
        registry_content_fingerprint: "f".to_owned(),
        origin: PlanOrigin::SameProcess,
    }
}

/// [`pinned_plan`]'s canonical encoding, byte for byte, written by hand.
///
/// # Why this is written out rather than compared against itself
///
/// A plan's identity is the digest of these bytes, so the bytes are the
/// artefact: two builds that disagree about them mint two identities for one
/// plan, and a caller comparing identities across the disagreement is told two
/// identical reads are different ones. Nothing else in this file can catch that.
/// The encode/decode round trip above cannot: a codec that writes a field wrongly
/// and reads it back the same wrong way round-trips perfectly. The version header
/// pin cannot: it covers two bytes out of four hundred and seventy-six.
///
/// So the literal is transcribed, annotated, and asserted against — never
/// regenerated. A helper that re-derived it from the encoder would be comparing
/// the encoder with itself, which is the property that is already free. The
/// price is that a deliberate layout change has to be re-transcribed by hand,
/// and that price is the point: it is the moment a reader is made to see how
/// much of the encoding moved, and to move `PLAN_VERSION` with it.
///
/// # What moved when the layout became version 4
///
/// Four things, each marked `NEW IN v4` below, and nothing else:
///
/// 1. the version value itself, `3` to `4`, in the first two bytes;
/// 2. the whole **derivations** section, inserted between the depths and the
///    statistics;
/// 3. a **presence tag** before every statistics entry's cardinality, which in
///    version 3 was a bare `u64` that could not say "absent";
/// 4. a length-framed **`selectivity_terms`** run after every statistics entry's
///    selectivity.
///
/// Every other segment is version 3's, carried forward unchanged and marked
/// `v3` — which is the second half of what a reader needs: not only that the
/// layout moved, but that it moved exactly this far.
const PINNED_CANONICAL_BYTES: &[u8] = &[
    // ===== header =====
    // NEW IN v4: `PLAN_VERSION`, u16 LE. Version 3 wrote `03 00` here. It is the
    // FIRST field, so no version can share an encoding with another — which is
    // why "the same plan serializes identically across a version bump" is not a
    // property any layout can have, and why these bytes are pinned instead.
    0x04, 0x00, // layout version: 4
    // ===== request terms (v3, unchanged) =====
    0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // count: 1 term
    0x00, // term 0 tag: TERM_LEXICAL
    0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // text length: 1
    0x71, // text: "q"
    0x00, // language: absent
    0x01, // predicate: present
    0x14, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // predicate length: 20
    0x68, 0x74, 0x74, 0x70, 0x3a, 0x2f, 0x2f, // "http://"
    0x65, 0x78, 0x61, 0x6d, 0x70, 0x6c, 0x65, 0x2e, 0x6f, 0x72, 0x67, // "example.org"
    0x2f, 0x70, // "/p"
    // ===== producer bindings (v3, unchanged) =====
    0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // count: 1 binding
    0x15, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // producer length: 21
    0x68, 0x74, 0x74, 0x70, 0x3a, 0x2f, 0x2f, // "http://"
    0x65, 0x78, 0x61, 0x6d, 0x70, 0x6c, 0x65, 0x2e, 0x6f, 0x72, 0x67, // "example.org"
    0x2f, 0x70, 0x66, // "/pf"
    0x14, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // stratum length: 20
    0x68, 0x74, 0x74, 0x70, 0x3a, 0x2f, 0x2f, // "http://"
    0x65, 0x78, 0x61, 0x6d, 0x70, 0x6c, 0x65, 0x2e, 0x6f, 0x72, 0x67, // "example.org"
    0x2f, 0x73, // "/s"
    0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // bound request terms: 1
    0x00, 0x00, 0x00, 0x00, // request term index 0, u32 LE
    // ===== producer decisions (v3, unchanged) =====
    0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // count: 1 decision
    0x00, // decision tag: DECISION_SELECTED
    0x15, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // producer length: 21
    0x68, 0x74, 0x74, 0x70, 0x3a, 0x2f, 0x2f, // "http://"
    0x65, 0x78, 0x61, 0x6d, 0x70, 0x6c, 0x65, 0x2e, 0x6f, 0x72, 0x67, // "example.org"
    0x2f, 0x70, 0x66, // "/pf"
    0x14, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // stratum length: 20
    0x68, 0x74, 0x74, 0x70, 0x3a, 0x2f, 0x2f, // "http://"
    0x65, 0x78, 0x61, 0x6d, 0x70, 0x6c, 0x65, 0x2e, 0x6f, 0x72, 0x67, // "example.org"
    0x2f, 0x73, // "/s"
    // ===== stratum depths (v3, unchanged) =====
    0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // count: 1 stratum
    0x14, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // stratum length: 20
    0x68, 0x74, 0x74, 0x70, 0x3a, 0x2f, 0x2f, // "http://"
    0x65, 0x78, 0x61, 0x6d, 0x70, 0x6c, 0x65, 0x2e, 0x6f, 0x72, 0x67, // "example.org"
    0x2f, 0x73, // "/s"
    0x02, 0x00, 0x00, 0x00, // depth: 2, u32 LE
    // ===== stratum derivations: NEW IN v4, the whole section =====
    // Version 3 wrote nothing here at all. It sits BETWEEN the depths above and
    // the statistics below, so a version-3 document read under this layout would
    // take the statistics source's length frame for a derivation count — which
    // is why the section's arrival moved the version rather than appending a tag.
    0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // count: 1 derivation
    0x14, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // stratum length: 20
    0x68, 0x74, 0x74, 0x70, 0x3a, 0x2f, 0x2f, // "http://"
    0x65, 0x78, 0x61, 0x6d, 0x70, 0x6c, 0x65, 0x2e, 0x6f, 0x72, 0x67, // "example.org"
    0x2f, 0x73, // "/s"
    0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // declared: 8 rows, u64 LE
    0x01, // cardinality: PRESENT
    0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // cardinality: 4, u64 LE
    0x01, // selectivity: PRESENT
    0x20, 0xa1, 0x07, 0x00, 0x00, 0x00, 0x00, 0x00, // selectivity: 500_000 ppm
    0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // selectivity terms: 1 index
    0x00, 0x00, 0x00, 0x00, // request term index 0, u32 LE
    0x01, // licensed prefix: PRESENT
    0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // licensed prefix: 3 rows
    // ===== statistics snapshot (v3 frame, v4 entry fields) =====
    0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // source length: 4
    0x68, 0x6f, 0x73, 0x74, // "host"
    0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // revision length: 2
    0x72, 0x31, // "r1"
    0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // count: 2 entries, ascending
    // --- entry 1: the request predicate, measured in neither dimension ---
    0x14, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // subject length: 20
    0x68, 0x74, 0x74, 0x70, 0x3a, 0x2f, 0x2f, // "http://"
    0x65, 0x78, 0x61, 0x6d, 0x70, 0x6c, 0x65, 0x2e, 0x6f, 0x72, 0x67, // "example.org"
    0x2f, 0x70, // "/p"
    // NEW IN v4: the presence tag. Version 3 wrote a bare `u64` here, so it had
    // no spelling for "the provider reported nothing" and the absence below
    // could only have gone out as a zero — a measurement the provider never
    // made. This ONE byte is the whole of that fix.
    0x00, // cardinality: ABSENT — and no value follows it
    0x00, // selectivity: ABSENT (v3 already spelled this one)
    // NEW IN v4: the selectivity's term domain, length-framed. Empty here,
    // because an absent selectivity has no contributing terms.
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // selectivity terms: 0 indices
    // --- entry 2: the stratum, the projection of the derivation above ---
    0x14, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // subject length: 20
    0x68, 0x74, 0x74, 0x70, 0x3a, 0x2f, 0x2f, // "http://"
    0x65, 0x78, 0x61, 0x6d, 0x70, 0x6c, 0x65, 0x2e, 0x6f, 0x72, 0x67, // "example.org"
    0x2f, 0x73, // "/s"
    0x01, // NEW IN v4: cardinality PRESENT
    0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // cardinality: 4, u64 LE
    0x01, // selectivity: PRESENT
    0x20, 0xa1, 0x07, 0x00, 0x00, 0x00, 0x00, 0x00, // selectivity: 500_000 ppm
    0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // NEW IN v4: 1 index
    0x00, 0x00, 0x00, 0x00, // request term index 0, u32 LE
    // ===== registry identities (v3, unchanged) =====
    0x07, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // instance counter: 7, u64 LE
    0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // fingerprint length: 1
    0x66, // "f"
    // ===== unserved terms (v3, unchanged) =====
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // count: 0 — every term served
    // ===== read bound (v3, unchanged) =====
    0x01, // bound tag: BOUND_BOUNDED
    0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // top-k: 3 rows, u64 LE
];

/// The encoder writes [`PINNED_CANONICAL_BYTES`] and nothing else.
///
/// The comparison is byte by byte before it is whole-slice, so a failure names
/// the offset that moved rather than printing four hundred and seventy-six bytes
/// twice and leaving a reader to diff them.
#[test]
fn the_canonical_encoding_is_the_bytes_written_out_by_hand() {
    let plan = pinned_plan();
    plan.certify()
        .expect("the pinned plan's depth follows from its own recorded inputs");
    let encoded = plan.canonical_bytes();

    for (offset, (written, pinned)) in encoded.iter().zip(PINNED_CANONICAL_BYTES).enumerate() {
        assert_eq!(
            written, pinned,
            "byte {offset}: the encoder wrote {written:#04x} where the pinned \
             literal spells {pinned:#04x}"
        );
    }
    assert_eq!(
        encoded.len(),
        PINNED_CANONICAL_BYTES.len(),
        "the encoding changed length, so a field was added, dropped or reframed"
    );
    assert_eq!(encoded, PINNED_CANONICAL_BYTES);

    // The literal is a plan, not just a string of bytes: it decodes to the plan
    // it was transcribed from. Without this a transcription error that happened
    // to keep the length would be indistinguishable from an encoder change.
    let decoded =
        Plan::from_canonical_bytes(PINNED_CANONICAL_BYTES).expect("the pinned literal decodes");
    assert_eq!(decoded, plan);
    assert_eq!(decoded.id(), plan.id());
}

/// The version this build writes is the version the pinned literal begins with.
///
/// Stated separately because the two are different claims. The literal above
/// pins what *these* bytes are; this pins that they are the bytes of the version
/// this build declares — so a `PLAN_VERSION` moved without re-transcribing the
/// literal fails here, by name, instead of failing as an anonymous byte-0
/// mismatch.
#[test]
fn the_pinned_literal_begins_with_this_builds_plan_version() {
    assert_eq!(
        PINNED_CANONICAL_BYTES[..2],
        PLAN_VERSION.to_le_bytes(),
        "the literal was transcribed under a different layout version"
    );
    assert_eq!(PLAN_VERSION, 4);
}

// ---------------------------------------------------------------------------
// The presence tag, on its own
// ---------------------------------------------------------------------------

/// A plan carrying nothing but a two-row statistics snapshot: one subject the
/// provider answered for with `measured`, one it was silent about.
///
/// Every other section is empty, deliberately. The snapshot is what this pair of
/// tests is about, and emptying everything ahead of it puts the section at an
/// offset a reader can verify by adding up six numbers rather than by trusting a
/// search.
fn statistics_only_plan(measured: Option<u64>) -> Plan {
    Plan {
        version: Plan::VERSION,
        request_terms: Vec::new(),
        read_bound: ReadBound::Complete,
        producer_bindings: Vec::new(),
        producer_decisions: Vec::new(),
        unserved_terms: Vec::new(),
        stratum_depths: HashMap::new(),
        stratum_derivations: BTreeMap::new(),
        statistics_snapshot: StatisticsSnapshot {
            source: "host".to_owned(),
            revision: "r1".to_owned(),
            entries: StatisticsEntries::new(vec![
                StatisticsEntry {
                    subject: "http://example.org/m".to_owned(),
                    cardinality: measured,
                    selectivity_ppm: None,
                    selectivity_terms: Vec::new(),
                },
                StatisticsEntry {
                    subject: "http://example.org/s".to_owned(),
                    cardinality: None,
                    selectivity_ppm: None,
                    selectivity_terms: Vec::new(),
                },
            ])
            .expect("the fixture names each subject once"),
        },
        registry_instance_id: RegistryId::from_raw(7),
        registry_content_fingerprint: "f".to_owned(),
        origin: PlanOrigin::SameProcess,
    }
}

/// Where [`statistics_only_plan`]'s snapshot begins in its canonical bytes.
///
/// Two bytes of version, then five empty length-framed sections — request terms,
/// producer bindings, producer decisions, stratum depths, stratum derivations —
/// each of which writes its eight-byte count of zero and nothing else. So
/// `2 + 5 * 8`, and bytes `0..42` are the version followed by forty zeroes, which
/// the test below asserts rather than assumes.
const STATISTICS_SECTION_OFFSET: usize = 42;

/// Where the measured subject's cardinality presence tag sits.
///
/// [`STATISTICS_SECTION_OFFSET`], then the framed source (`8 + 4`), the framed
/// revision (`8 + 2`), the entry count (`8`) and the first subject (`8 + 20`):
/// `42 + 12 + 10 + 8 + 28`.
const MEASURED_CARDINALITY_OFFSET: usize = 100;

/// The statistics section of `statistics_only_plan(Some(42))`, byte for byte,
/// written by hand.
///
/// Hand-written for the reason the whole-plan literal above is, and separate
/// from it for a narrower one: this is the section the layout version moved for,
/// and a reader who wants to see what "absent is not zero" costs in bytes should
/// not have to find it inside four hundred and seventy-six of them.
const PINNED_STATISTICS_SECTION: &[u8] = &[
    0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // source length: 4
    0x68, 0x6f, 0x73, 0x74, // "host"
    0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // revision length: 2
    0x72, 0x31, // "r1"
    0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // count: 2 entries, ascending
    // --- the subject the provider answered for ---
    0x14, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // subject length: 20
    0x68, 0x74, 0x74, 0x70, 0x3a, 0x2f, 0x2f, // "http://"
    0x65, 0x78, 0x61, 0x6d, 0x70, 0x6c, 0x65, 0x2e, 0x6f, 0x72, 0x67, // "example.org"
    0x2f, 0x6d, // "/m"
    0x01, // cardinality: PRESENT  <- MEASURED_CARDINALITY_OFFSET
    0x2a, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // cardinality: 42, u64 LE
    0x00, // selectivity: ABSENT
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // selectivity terms: 0 indices
    // --- the subject the provider was silent about ---
    0x14, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // subject length: 20
    0x68, 0x74, 0x74, 0x70, 0x3a, 0x2f, 0x2f, // "http://"
    0x65, 0x78, 0x61, 0x6d, 0x70, 0x6c, 0x65, 0x2e, 0x6f, 0x72, 0x67, // "example.org"
    0x2f, 0x73, // "/s"
    0x00, // cardinality: ABSENT — and no value follows it
    0x00, // selectivity: ABSENT
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // selectivity terms: 0 indices
];

/// The nine bytes a measured cardinality of zero occupies.
const MEASURED_ZERO: &[u8] = &[
    0x01, // PRESENT
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // the measurement: 0, u64 LE
];

/// The one byte an unmeasured cardinality occupies.
const MEASURED_NOTHING: &[u8] = &[
    0x00, // ABSENT — the value is not written at all
];

/// A measured cardinality and a silent one are spelled out, side by side, in the
/// section the layout version moved for.
#[test]
fn the_statistics_section_spells_a_measured_cardinality_beside_a_silent_one() {
    let plan = statistics_only_plan(Some(42));
    // A byte-layout specimen rather than a plan any planner would emit: its
    // request is empty, so both subjects its snapshot names are subjects nothing
    // consulted, and `certify` says exactly that. The depth side is coherent —
    // there are no depths to contradict — which is what makes the refusal the
    // snapshot's own. The bytes below are the subject here, and they are the
    // bytes of this value either way.
    assert!(
        matches!(
            plan.certify(),
            Err(PlanError::UnconsultedStatisticsSubject { .. })
        ),
        "the fixture's empty request consults nothing, so its rows are evidence \
         about no consultation"
    );
    let encoded = plan.canonical_bytes();

    // The offset, verified rather than trusted: the version, then five empty
    // sections' worth of zeroed counts.
    assert_eq!(&encoded[..2], &PLAN_VERSION.to_le_bytes());
    assert_eq!(
        &encoded[2..STATISTICS_SECTION_OFFSET],
        &[0u8; STATISTICS_SECTION_OFFSET - 2],
        "five empty sections precede the snapshot, each an eight-byte count of zero"
    );

    let section = &encoded[STATISTICS_SECTION_OFFSET..][..PINNED_STATISTICS_SECTION.len()];
    for (index, (written, pinned)) in section.iter().zip(PINNED_STATISTICS_SECTION).enumerate() {
        assert_eq!(
            written,
            pinned,
            "byte {} of the plan (byte {index} of the statistics section): the \
             encoder wrote {written:#04x} where the pinned literal spells {pinned:#04x}",
            STATISTICS_SECTION_OFFSET + index
        );
    }
    assert_eq!(section, PINNED_STATISTICS_SECTION);

    // And the section really is the whole of what follows it up to the registry
    // identities, so the literal above is not pinning a prefix of a longer one.
    assert_eq!(
        encoded.len(),
        STATISTICS_SECTION_OFFSET + PINNED_STATISTICS_SECTION.len() + 8 + 9 + 8 + 1,
        "the snapshot is followed by the instance counter, the framed \
         one-character fingerprint, an empty unserved list and the complete bound"
    );
}

/// "The provider measured zero" and "the provider measured nothing" are one byte
/// apart, and that byte reaches the plan's identity.
///
/// This is the distinction the whole record exists for, and the one a sentinel
/// encoding would have destroyed: every `u64` is a legal measurement, so a
/// spelling that reserved one of them for absence would make the reserved
/// measurement unrepresentable — and zero, the value a provider reports when it
/// counted and found no rows, is exactly the measurement such a scheme would
/// most likely reserve.
///
/// The oracle is the byte AND the identity. Bytes alone would pass for an
/// encoding that distinguished the two and a digest that did not read the
/// distinguishing byte; identity alone would pass for an encoder that moved
/// something else entirely.
#[test]
fn a_measured_zero_and_a_silent_subject_differ_in_a_byte_and_in_identity() {
    let measured_zero = statistics_only_plan(Some(0));
    let silent = statistics_only_plan(None);

    let zero_bytes = measured_zero.canonical_bytes();
    let silent_bytes = silent.canonical_bytes();

    // The one byte: the presence tag. It is `01` for the measurement of zero and
    // `00` for the silence, at the same offset, and the eight bytes of the
    // measurement itself follow only the first.
    assert_eq!(
        &zero_bytes[MEASURED_CARDINALITY_OFFSET..][..MEASURED_ZERO.len()],
        MEASURED_ZERO,
        "a measured zero is a present tag followed by eight zero bytes"
    );
    assert_eq!(
        &silent_bytes[MEASURED_CARDINALITY_OFFSET..][..MEASURED_NOTHING.len()],
        MEASURED_NOTHING,
        "and a silence is the absent tag, with no measurement after it"
    );
    assert_eq!(
        zero_bytes
            .iter()
            .zip(&silent_bytes)
            .position(|(left, right)| left != right),
        Some(MEASURED_CARDINALITY_OFFSET),
        "the two encodings agree up to the presence tag and part company there"
    );
    assert_eq!(
        zero_bytes.len() - silent_bytes.len(),
        8,
        "the measurement's own eight bytes are written only when there is one"
    );

    assert_ne!(zero_bytes, silent_bytes);
    assert_ne!(
        measured_zero.id(),
        silent.id(),
        "a plan planned against a counted zero and one planned against a silence \
         must not share an identity, or a caller comparing identities would be \
         told two different measurements were one"
    );
    assert_ne!(measured_zero, silent);

    // The control: the difference above is the cardinality's and nothing else's.
    // Two independent builds of the same measurement agree byte for byte, so a
    // fixture that varied in some other way could not have produced it.
    assert_eq!(
        statistics_only_plan(Some(0)).canonical_bytes(),
        zero_bytes,
        "the encoding is a pure function of the plan"
    );
    assert_eq!(statistics_only_plan(Some(0)).id(), measured_zero.id());

    // The valid neighbours: zero is an ordinary measurement, not a special one.
    // It is as distinct from one as it is from silence, and its round trip
    // reads back as a measurement rather than as an absence.
    for other in [Some(1), Some(42)] {
        let neighbour = statistics_only_plan(other);
        assert_ne!(neighbour.id(), measured_zero.id(), "{other:?} is not zero");
        assert_ne!(neighbour.id(), silent.id(), "{other:?} is not silence");
    }
    for measured in [Some(0), Some(1), Some(42), None] {
        let plan = statistics_only_plan(measured);
        let decoded =
            Plan::from_canonical_bytes(&plan.canonical_bytes()).expect("canonical decode");
        assert_eq!(
            decoded.statistics_snapshot.entries[0].cardinality, measured,
            "a decoded {measured:?} is the value that was written, never the other one"
        );
        assert_eq!(decoded.id(), plan.id());
    }
}

// ---------------------------------------------------------------------------
// The derivations section does not remember how it was filled
// ---------------------------------------------------------------------------

/// How many strata the insertion-order fixture carries.
const ORDERED_STRATA: usize = 5;

/// The `index`-th permutation of `0..ORDERED_STRATA`, in factorial-base order.
///
/// Written out rather than drawn at random: a random order is unreproducible, so
/// a failure could not be re-run, and randomness is not what this test needs
/// anyway. It needs EVERY order, which is what enumerating them gives.
fn permutation(mut index: usize) -> Vec<usize> {
    let mut pool: Vec<usize> = (0..ORDERED_STRATA).collect();
    let mut order = Vec::with_capacity(ORDERED_STRATA);
    let mut block: usize = (1..=ORDERED_STRATA).product();
    for remaining in (1..=ORDERED_STRATA).rev() {
        block /= remaining;
        order.push(pool.remove(index / block));
        index %= block;
    }
    order
}

/// A plan over [`ORDERED_STRATA`] strata whose derivations, depths and snapshot
/// rows were all inserted in `order`.
///
/// Each stratum declares its own row count, so the encoded section differs
/// position by position: an encoder that emitted the entries in the order they
/// arrived would write different bytes for different orders rather than the same
/// bytes in a different arrangement.
fn plan_filled_in(order: &[usize]) -> Plan {
    let mut plan = baseline();
    plan.stratum_depths.clear();
    plan.stratum_derivations.clear();
    let mut rows: Vec<StatisticsEntry> = Vec::new();
    for &index in order {
        let stratum = iri(&format!("http://example.org/stratum/{index}"));
        // Distinct declarations AND distinct measurements, so every field of
        // every record is that stratum's own.
        let declared = 100 + index as u64;
        let cardinality = 40 + index as u64;
        plan.stratum_derivations.insert(
            stratum.clone(),
            DepthInputs {
                declared,
                cardinality: Some(cardinality),
                selectivity_ppm: None,
                selectivity_terms: Vec::new(),
                licensed_prefix: Some(25),
            },
        );
        plan.stratum_depths.insert(
            stratum.clone(),
            u32::try_from(cardinality.min(25)).expect("the fixture depths fit a rank"),
        );
        rows.push(StatisticsEntry {
            subject: stratum.as_str().to_owned(),
            cardinality: Some(cardinality),
            selectivity_ppm: None,
            selectivity_terms: Vec::new(),
        });
    }
    edit_rows(&mut plan, |existing| *existing = rows);
    plan
}

/// The derivations section is a function of the derivations, not of the order
/// they were inserted in.
///
/// # Why one construction would prove nothing
///
/// The sibling section next door is a `HashMap`, and the argument that its
/// encoding is order-independent is a sort the encoder performs; the argument
/// here is that a [`BTreeMap`](std::collections::BTreeMap) has no insertion
/// order to leak. Both are arguments from the type, and this test exists
/// because an argument from the type is not an executed case — the section was
/// added with that reasoning and no adversarial run behind it.
///
/// A single pair of constructions could not close that. Rust seeds each
/// `HashMap` from a per-thread counter, so iteration order varies between maps
/// but is FIXED within one: two plans built once might happen to agree, and the
/// test would be green on a coincidence it could not detect. So every order is
/// enumerated — all one hundred and twenty of them — and the whole enumeration
/// is repeated fifty times, which is six thousand freshly seeded structures
/// rather than two.
///
/// The oracle is the bytes, the identity and the value, all three. Bytes alone
/// would pass for a digest that read a prefix; identity alone would pass for an
/// encoder that moved a field the digest ignores; equality alone is the
/// `BTreeMap`'s own, which is the thing under suspicion.
#[test]
fn plans_differing_only_in_derivation_insertion_order_are_one_plan() {
    let orders: usize = (1..=ORDERED_STRATA).product();
    let reference = plan_filled_in(&permutation(0));
    reference
        .certify()
        .expect("every recorded depth follows from the inputs beside it");
    let expected_bytes = reference.canonical_bytes();
    let expected_id = reference.id();
    assert_eq!(
        reference.stratum_derivations.len(),
        ORDERED_STRATA,
        "the fixture really carries every stratum, so the section under test is \
         not empty"
    );

    let mut rebuilds = 0_usize;
    for round in 0..50 {
        for index in 0..orders {
            let order = permutation(index);
            let plan = plan_filled_in(&order);
            rebuilds += 1;
            assert_eq!(
                plan.canonical_bytes(),
                expected_bytes,
                "round {round}, order {order:?}: the derivations section \
                 remembered how it was filled"
            );
            assert_eq!(plan.id(), expected_id, "round {round}, order {order:?}");
            assert_eq!(plan, reference, "round {round}, order {order:?}");
            plan.certify().expect("and each rebuild is still coherent");
        }
    }
    assert_eq!(
        rebuilds, 6_000,
        "six thousand freshly seeded structures, not two"
    );

    // The control: the section is not order-independent because it is constant.
    // Change one stratum's declaration and the bytes, the identity and the value
    // all move — so the equalities above are observations and not vacuities.
    let mut moved = plan_filled_in(&permutation(0));
    let stratum = iri("http://example.org/stratum/3");
    moved
        .stratum_derivations
        .get_mut(&stratum)
        .expect("the fixture carries this stratum")
        .declared += 1;
    assert_ne!(moved.canonical_bytes(), expected_bytes);
    assert_ne!(moved.id(), expected_id);
    assert_ne!(moved, reference);
}

/// Every order the fixture enumerates is a distinct permutation, and together
/// they are all of them.
///
/// The enumeration is the test above's whole coverage claim, so it is checked
/// rather than assumed: an off-by-one in the factorial-base decode would quietly
/// repeat one order a hundred and twenty times and leave the adversarial case
/// unrun while every assertion still passed.
#[test]
fn the_insertion_orders_enumerated_are_every_permutation_exactly_once() {
    let orders: usize = (1..=ORDERED_STRATA).product();
    let mut seen: std::collections::BTreeSet<Vec<usize>> = std::collections::BTreeSet::new();
    for index in 0..orders {
        let order = permutation(index);
        assert_eq!(
            order.len(),
            ORDERED_STRATA,
            "order {index} is not a whole permutation: {order:?}"
        );
        assert_eq!(
            order
                .iter()
                .copied()
                .collect::<std::collections::BTreeSet<_>>(),
            (0..ORDERED_STRATA).collect::<std::collections::BTreeSet<_>>(),
            "order {index} names a stratum twice or not at all: {order:?}"
        );
        assert!(
            seen.insert(order.clone()),
            "order {index} repeats {order:?}"
        );
    }
    assert_eq!(seen.len(), orders, "all {orders} orders, each once");
    assert_eq!(
        permutation(0),
        vec![0, 1, 2, 3, 4],
        "the first is ascending"
    );
    assert_eq!(
        permutation(orders - 1),
        vec![4, 3, 2, 1, 0],
        "and the last is descending, which is the adversarial one"
    );
}
