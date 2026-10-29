// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Borrowed foundation owners used by retrieval and its native failures.

use purrdf_hash::error::{ErrorValue, StructuredError};
use purrdf_retrieval::fixture::PRECISION_12;
use purrdf_testkit::error_capture::Capture;

#[global_allocator]
static GLOBAL: purrdf_alloc_probe::CountingAllocator = purrdf_alloc_probe::CountingAllocator;

#[derive(Debug)]
struct SpawnCause(u64);

impl core::fmt::Display for SpawnCause {
    fn fmt(&self, _: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        panic!("inspecting the original spawn cause must not render it")
    }
}

impl std::error::Error for SpawnCause {}

impl StructuredError for SpawnCause {
    purrdf_hash::error_visit! {
        @tuple self (count) => "spawn-cause-control" {
            "count" => Unsigned(u128::from(*count)),
        }
    }
}

fn stack_refusals_borrow_original_native_causes_and_full_width_counts() {
    let error = purrdf_stack::StackError::Spawn(
        purrdf_hash::error::io_error(std::io::ErrorKind::Other, SpawnCause(u64::MAX)).into(),
    );
    let captured = Capture::read(&error);
    assert_eq!(captured.code, "stack-spawn");
    let native = captured.child("cause").child("cause").child("cause");
    assert_eq!(native.code, "spawn-cause-control");
    assert!(
        matches!(native.field("count"), ErrorValue::Unsigned(value) if value == u128::from(u64::MAX))
    );
    let mut cause = std::error::Error::source(&error);
    loop {
        let current = cause.expect("the original native source must remain reachable");
        if let Some(original) = current.downcast_ref::<SpawnCause>() {
            assert_eq!(original.0, u64::MAX);
            break;
        }
        cause = current.source();
    }
    let floor = purrdf_stack::StackError::ExceedsFloor {
        requested: usize::MAX,
        available: 0,
    };
    let captured = Capture::read(&floor);
    assert!(
        matches!(captured.field("requested"), ErrorValue::ByteCount(value) if value == usize::MAX as u128)
    );
    assert!(matches!(
        captured.field("available"),
        ErrorValue::ByteCount(0)
    ));
}

fn scoped_bases_retain_the_original_iri_and_directive_provenance() {
    let input = String::from("https://example.org/original/base/");
    let base = purrdf_iri::ScopedBase::new(
        purrdf_iri::BaseIri::parse(&input).unwrap(),
        purrdf_iri::BaseOrigin::Directive {
            line: u64::MAX,
            column: u32::MAX,
        },
    );
    drop(input);
    let copied = base.clone();
    drop(base);
    let captured = Capture::read(&copied);
    assert!(matches!(
        captured.child("iri").field("iri"),
        ErrorValue::Iri("https://example.org/original/base/")
    ));
    let origin = captured.child("origin");
    assert!(
        matches!(origin.field("line"), ErrorValue::Unsigned(value) if value == u128::from(u64::MAX))
    );
    assert!(
        matches!(origin.field("column"), ErrorValue::Unsigned(value) if value == u128::from(u32::MAX))
    );
}

fn fixed_values_binding_masks_and_registry_ids_keep_their_exact_native_bits() {
    for raw in [0_u128, 1, u128::MAX] {
        let fixed = purrdf_retrieval::Score::from_raw(raw, PRECISION_12);
        let captured = Capture::read(&fixed);
        assert!(matches!(captured.field("raw"), ErrorValue::Unsigned(value) if value == raw));
        assert!(matches!(
            captured.field("scale_digits"),
            ErrorValue::Unsigned(12)
        ));
    }
    let pattern = purrdf_sparql_eval::BindingPattern::from_bound_positions(64, 0..64);
    let captured = Capture::read(&pattern);
    assert!(matches!(captured.field("arity"), ErrorValue::Unsigned(64)));
    assert!(
        matches!(captured.field("bound"), ErrorValue::Unsigned(value) if value == u128::from(u64::MAX))
    );
    let registry = purrdf_sparql_eval::RegistryId::from_raw(u64::MAX);
    let captured = Capture::read(&registry);
    assert!(
        matches!(captured.field("value"), ErrorValue::Unsigned(value) if value == u128::from(u64::MAX))
    );
    assert!(matches!(
        captured.field("scope"),
        ErrorValue::Code("process")
    ));
}

fn attestations_preserve_original_generation_and_caller_message_owners() {
    use purrdf_sparql_eval::{IndexGeneration, PfAttestation, ServiceLevel};

    let generation = String::from("original-generation-λ");
    let reason = String::from("caller-owned unavailable shard β");
    let attestation = PfAttestation {
        generation: IndexGeneration::declared(generation.clone()),
        service: ServiceLevel::incomplete(&reason),
    };
    drop((generation, reason));
    let copied = attestation.clone();
    drop(attestation);
    let captured = Capture::read(&copied);
    assert!(matches!(
        captured.child("generation").field("value"),
        ErrorValue::Text("original-generation-λ")
    ));
    assert!(matches!(
        captured.child("service").field("reason"),
        ErrorValue::Message("caller-owned unavailable shard β")
    ));
}

fn evaluator_conversion_retains_the_original_parser_cause_after_input_disposal() {
    // The native cause must not widen composed Result errors past Clippy's
    // existing large-error boundary, including on the portable target.
    const { assert!(size_of::<purrdf_retrieval::SearchError>() < 128) };
    let input = String::from("SELECT WHERE {");
    let cause = purrdf_sparql_algebra::SparqlParser::new()
        .parse_query(&input)
        .unwrap_err();
    drop(input);
    let expected = cause.clone();
    let failure = purrdf_sparql_eval::EvalError::from(cause);
    let copied = failure.clone();
    drop(failure);
    let source = std::error::Error::source(&copied).unwrap();
    assert_eq!(
        source.downcast_ref::<purrdf_sparql_algebra::ParseError>(),
        Some(&expected)
    );
    assert!(
        purrdf_sparql_algebra::SparqlParser::new()
            .parse_query("SELECT * WHERE {}")
            .is_ok()
    );
}

fn retrieval_actual_iri_failures_keep_original_nested_cause() {
    let input = String::from("not an iri λ");
    let original = purrdf_retrieval::Iri::parse(&input).unwrap_err();
    drop(input);
    let failure = purrdf_retrieval::SearchError::from(original);
    let events = Vec::with_capacity(4);
    let window = purrdf_alloc_probe::CurrentThreadWindow::open();
    let capture = Capture::read_with_buffer(&failure, events);
    assert_eq!(window.close().allocations, 0);
    assert_eq!(capture.code, "retrieval-search-plan");
    let plan = capture.child("cause");
    assert_eq!(plan.code, "retrieval-plan-invalid-iri");
    assert!(matches!(
        plan.field("text"),
        ErrorValue::Iri("not an iri λ")
    ));
    let native = std::error::Error::source(&failure)
        .unwrap()
        .source()
        .unwrap();
    assert!(native.downcast_ref::<purrdf_core::IriError>().is_some());
    assert_eq!(
        plan.child("source").code,
        Capture::read(native.downcast_ref::<purrdf_core::IriError>().unwrap()).code
    );
    assert!(purrdf_retrieval::Iri::parse("https://example.org/accepted").is_ok());
}

fn retrieval_protocol_lists_preserve_order_and_full_width_counts() {
    use purrdf_testkit::error_capture::Event;
    let stratum = "https://example.org/offending-stratum";
    let named_by = "https://example.org/witness-stratum";
    for error in [
        purrdf_retrieval::ProtocolError::OutsideDeclaredDomain {
            item: "original candidate".into(),
            stratum: stratum.into(),
            named_by: named_by.into(),
        },
        purrdf_retrieval::ProtocolError::CandidateInTwoBlocks {
            item: "original candidate".into(),
            stratum: stratum.into(),
            block: "offending block".into(),
            named_by: named_by.into(),
            named_by_block: "witness block".into(),
        },
    ] {
        let fields = Capture::read(&error);
        assert!(matches!(fields.field("stratum"), ErrorValue::Iri(value) if value == stratum));
        assert!(matches!(fields.field("named_by"), ErrorValue::Iri(value) if value == named_by));
    }
    let error = purrdf_retrieval::ProtocolError::UnbackedDomainDeclaration {
        stratum: "https://example.org/stratum".into(),
        declared: vec!["second λ".into(), "first β".into(), "second λ".into()],
        rank: u64::MAX,
    };
    let fields = Capture::read(&error);
    assert!(
        matches!(fields.field("rank"), ErrorValue::Unsigned(value) if value == u128::from(u64::MAX))
    );
    let entries: Vec<_> = fields
        .events
        .iter()
        .filter_map(|event| match event {
            Event::Value(None, ErrorValue::Text(text)) => Some(*text),
            _ => None,
        })
        .collect();
    assert_eq!(entries, ["second λ", "first β", "second λ"]);
    let error = purrdf_retrieval::FusionError::NonPositiveWeight {
        stratum: "https://example.org/stratum".into(),
        weight: purrdf_retrieval::Score::from_raw(0, PRECISION_12),
    };
    assert!(matches!(
        Capture::read(&error).child("weight").field("raw"),
        ErrorValue::Unsigned(0)
    ));
}

fn retrieval_request_views_keep_ieee_bits_and_optional_native_values() {
    use purrdf_retrieval::{Metric, RequestTerm};
    use purrdf_testkit::error_capture::Event;
    let bits = [0x8000_0000, 0x7fc0_4321, 0x7f80_0000];
    let request = RequestTerm::Vector {
        embedding: bits.map(f32::from_bits).to_vec(),
        metric: Metric::Dot,
        index_hint: None,
    };
    let fields = Capture::read(&request);
    let captured: Vec<_> = fields
        .events
        .iter()
        .filter_map(|event| match event {
            Event::Value(None, ErrorValue::Float32Bits(bits)) => Some(*bits),
            _ => None,
        })
        .collect();
    assert_eq!(captured, bits);
    assert!(matches!(fields.field("index_hint"), ErrorValue::Absent));
    assert_eq!(fields.child("metric").code, "retrieval-metric-dot");
    let request = RequestTerm::Lexical {
        text: "original λ".into(),
        language: Some(String::new()),
        predicate: None,
    };
    let fields = Capture::read(&request);
    assert!(matches!(fields.field("language"), ErrorValue::Text("")));
    assert!(matches!(fields.field("predicate"), ErrorValue::Absent));
}

fn retrieval_native_failures_preserve_panicking_caller_cause() {
    use purrdf_testkit::error_capture::CallerRefusal;
    let error = purrdf_retrieval::ExecutionError::NativeFailure(
        purrdf_sparql_eval::EvalError::FunctionAdmission(
            purrdf_sparql_eval::UserFunctionRefusal::new(CallerRefusal { limit: u128::MAX }),
        ),
    );
    let error = purrdf_retrieval::SearchError::from(error);
    let fields = Capture::read(&error);
    let leaf = fields
        .child("cause")
        .child("cause")
        .child("cause")
        .child("cause");
    assert_eq!(leaf.code, "caller-refused");
    assert!(matches!(
        leaf.field("limit"),
        ErrorValue::Unsigned(u128::MAX)
    ));
    let mut native = std::error::Error::source(&error).unwrap();
    while let Some(next) = native.source() {
        native = next;
    }
    assert_eq!(
        native.downcast_ref::<CallerRefusal>().unwrap().limit,
        u128::MAX
    );
}

fn canonical_utf8_refusals_keep_native_causes_after_input_disposal() {
    use purrdf_core::FastMap;
    use purrdf_retrieval::{
        Plan, PlanOrigin, ReadBound, RegistryId, RequestTerm, StatisticsSnapshot,
    };
    use std::collections::BTreeMap;

    let text = "prefix λ";
    let plan = Plan {
        version: Plan::VERSION,
        request_terms: vec![RequestTerm::Lexical {
            text: text.into(),
            language: None,
            predicate: None,
        }],
        constraints: Vec::new(),
        read_bound: ReadBound::Complete,
        producer_bindings: Vec::new(),
        producer_decisions: Vec::new(),
        unserved_terms: Vec::new(),
        stratum_depths: FastMap::default(),
        stratum_derivations: BTreeMap::default(),
        statistics_snapshot: StatisticsSnapshot::default(),
        registry_instance_id: RegistryId::from_raw(7),
        registry_content_fingerprint: String::new(),
        origin: PlanOrigin::Deserialized,
    };
    let original = plan.canonical_bytes();
    let decoded = Plan::from_canonical_bytes(&original).unwrap();
    assert_eq!(decoded, plan);
    assert_eq!(decoded.id(), plan.id());
    let start = original
        .windows(text.len())
        .position(|part| part == text.as_bytes())
        .unwrap();
    for (tail, valid_up_to, error_len) in [([0xce, 0xff], 7, Some(1)), ([b'a', 0xce], 8, None)] {
        let mut bytes = original.clone();
        bytes[start + 7..start + text.len()].copy_from_slice(&tail);
        let error = Plan::from_canonical_bytes(&bytes).unwrap_err();
        drop(bytes);
        let native = std::error::Error::source(&error)
            .unwrap()
            .downcast_ref::<core::str::Utf8Error>()
            .unwrap();
        assert_eq!(native.valid_up_to(), valid_up_to);
        assert_eq!(native.error_len(), error_len);
        assert_eq!(error.refusal(), "invalid-utf8");
        assert_eq!(
            error.to_string(),
            "plan canonical encoding carries invalid UTF-8 in lexical text"
        );
        let events = Vec::with_capacity(16);
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        let capture = Capture::read_with_buffer(&error, events);
        let measured = window.close();
        assert_eq!((measured.allocations, measured.requested_bytes), (0, 0));
        assert_eq!(capture.code, "retrieval-plan-invalid-utf8");
        assert!(matches!(
            capture.field("what"),
            ErrorValue::Code("lexical text")
        ));
        let source = capture.child("source");
        assert_eq!(source.code, "std-utf8");
        assert!(
            matches!(source.field("valid_up_to"), ErrorValue::ByteCount(value) if value == valid_up_to as u128)
        );
        match (source.field("error_len"), error_len) {
            (ErrorValue::ByteCount(actual), Some(expected)) => assert_eq!(actual, expected as u128),
            (ErrorValue::Absent, None) => {}
            other => panic!("the original UTF-8 sequence length was lost: {other:?}"),
        }
    }
}

fn profile_decoder_refusals_keep_original_causes_and_owners() {
    use purrdf_retrieval::{DecayRule, FusionProfile, Iri};
    use std::collections::BTreeMap;

    let spelling = "https://example.org/λ";
    let profile = FusionProfile::with_decay(
        BTreeMap::from([(
            Iri::parse(spelling).unwrap(),
            PRECISION_12.score(PRECISION_12.scale()),
        )]),
        DecayRule::ReciprocalRank { k: 60 },
        PRECISION_12,
    )
    .unwrap();
    let original = profile.canonical_bytes();
    let decoded = FusionProfile::from_canonical_bytes(&original).unwrap();
    assert_eq!(decoded, profile);
    assert_eq!(decoded.id(), profile.id());
    let start = original
        .windows(spelling.len())
        .position(|part| part == spelling.as_bytes())
        .unwrap();
    for (tail, valid_up_to, error_len) in [
        ([0xce, 0xff], spelling.len() - 2, Some(1)),
        ([b'a', 0xce], spelling.len() - 1, None),
    ] {
        let mut bytes = original.clone();
        bytes[start + spelling.len() - 2..start + spelling.len()].copy_from_slice(&tail);
        let error = FusionProfile::from_canonical_bytes(&bytes).unwrap_err();
        drop(bytes);
        let native = std::error::Error::source(&error)
            .unwrap()
            .source()
            .unwrap()
            .source()
            .unwrap()
            .downcast_ref::<core::str::Utf8Error>()
            .unwrap();
        assert_eq!(native.valid_up_to(), valid_up_to);
        assert_eq!(native.error_len(), error_len);
        assert_eq!(
            error.to_string(),
            "malformed fusion profile: plan canonical encoding carries invalid UTF-8 in fusion profile stratum"
        );
        let events = Vec::with_capacity(24);
        let reason_events = Vec::with_capacity(8);
        let plan_events = Vec::with_capacity(8);
        let utf8_events = Vec::with_capacity(8);
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        let capture = Capture::read_with_buffer(&error, events);
        let ErrorValue::Cause(reason) = capture.field("reason") else {
            panic!("the profile reason must remain a borrowed cause");
        };
        let reason = Capture::read_with_buffer(reason, reason_events);
        let ErrorValue::Cause(plan) = reason.field("source") else {
            panic!("the profile must retain its original plan refusal");
        };
        let plan = Capture::read_with_buffer(plan, plan_events);
        let ErrorValue::Cause(utf8) = plan.field("source") else {
            panic!("the plan must retain its original UTF-8 refusal");
        };
        let utf8 = Capture::read_with_buffer(utf8, utf8_events);
        let measured = window.close();
        assert_eq!((measured.allocations, measured.requested_bytes), (0, 0));
        assert_eq!(capture.code, "fusion-malformed-profile");
        assert_eq!(reason.code, "fusion-profile-decode");
        assert!(matches!(
            plan.field("what"),
            ErrorValue::Code("fusion profile stratum")
        ));
        assert_eq!(utf8.code, "std-utf8");
        assert!(
            matches!(utf8.field("valid_up_to"), ErrorValue::ByteCount(value) if value == valid_up_to as u128)
        );
        match (utf8.field("error_len"), error_len) {
            (ErrorValue::ByteCount(actual), Some(expected)) => assert_eq!(actual, expected as u128),
            (ErrorValue::Absent, None) => {}
            other => panic!("the profile lost its original UTF-8 sequence length: {other:?}"),
        }
    }
    let mut bytes = original;
    bytes[start] = b' ';
    let error = FusionProfile::from_canonical_bytes(&bytes).unwrap_err();
    drop(bytes);
    let native = std::error::Error::source(&error)
        .unwrap()
        .source()
        .unwrap()
        .source()
        .unwrap()
        .downcast_ref::<purrdf_core::IriError>()
        .unwrap();
    let capture = Capture::read(&error);
    let plan = capture.child("reason").child("source");
    assert_eq!(plan.code, "retrieval-plan-invalid-iri");
    assert!(matches!(plan.field("text"), ErrorValue::Iri(text) if text == " ttps://example.org/λ"));
    assert_eq!(plan.child("source").code, Capture::read(native).code);
}

purrdf_testkit::harness_main!(
    profile_decoder_refusals_keep_original_causes_and_owners,
    canonical_utf8_refusals_keep_native_causes_after_input_disposal,
    retrieval_actual_iri_failures_keep_original_nested_cause,
    retrieval_protocol_lists_preserve_order_and_full_width_counts,
    retrieval_request_views_keep_ieee_bits_and_optional_native_values,
    retrieval_native_failures_preserve_panicking_caller_cause,
    evaluator_conversion_retains_the_original_parser_cause_after_input_disposal,
    stack_refusals_borrow_original_native_causes_and_full_width_counts,
    scoped_bases_retain_the_original_iri_and_directive_provenance,
    fixed_values_binding_masks_and_registry_ids_keep_their_exact_native_bits,
    attestations_preserve_original_generation_and_caller_message_owners,
);
