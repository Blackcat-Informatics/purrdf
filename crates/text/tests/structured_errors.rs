// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original public text failures inspected through the generic error protocol.

use purrdf_alloc_probe::{CountingAllocator, CurrentThreadWindow};
use purrdf_hash::error::ErrorValue;
use purrdf_sparql_eval::{EvalError, WorkspaceCapability};
use purrdf_testkit::error_capture::{Capture, Event};
use purrdf_text::phonetic::DoubleMetaphone;
use purrdf_text::{Analyzer, AnalyzerProfile, InputMode, SurfaceIndex, TextError};

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

fn phonetic_native_scalar_survives_evaluator_conversion() {
    let refusal = DoubleMetaphone::new(4).unwrap().encode("東京").unwrap_err();
    let error = TextError::from(refusal).into_eval_in(&WorkspaceCapability::resident());
    let clone = error.clone();
    let buffer = Vec::with_capacity(4);
    let window = CurrentThreadWindow::open();
    let capture = Capture::read_with_buffer(&error, buffer);
    assert_eq!(window.close().allocations, 0);
    let native = capture.child("cause");
    assert!(matches!(native.field("text"), ErrorValue::Absent));
    let original = native.child("cause").child("cause");
    assert_eq!(original.code, "text-phonetic-unsupported-scalar");
    assert!(matches!(
        original.field("scalar"),
        ErrorValue::Character('東')
    ));
    let source = std::error::Error::source(&error).unwrap().source().unwrap();
    let clone_source = std::error::Error::source(&clone).unwrap().source().unwrap();
    assert!(core::ptr::eq(source, clone_source));
    assert!(source.downcast_ref::<TextError>().is_some());
}

fn substring_native_refusal_preserves_counters_limits_and_generation() {
    let index = SurfaceIndex::from_texts(Analyzer::empty_lexicon(), [(0, "adamantine")]).unwrap();
    let refusal = index.substring("damant").unwrap_err();
    let error: EvalError = refusal.into();
    let original = Capture::read(&error)
        .child("cause")
        .child("cause")
        .child("cause");
    assert_eq!(original.child("reason").code, "text-substring-nonselective");
    assert!(matches!(
        original.field("distinct_spans"),
        ErrorValue::Unsigned(1)
    ));
    assert!(matches!(
        original.child("counters").field("candidate_spans"),
        ErrorValue::Unsigned(1)
    ));
    assert!(matches!(
        original.child("limits").field("candidate_spans"),
        ErrorValue::Unsigned(4096)
    ));
    assert!(
        matches!(original.field("generation"), ErrorValue::Bytes(bytes) if bytes == index.generation().as_slice())
    );
}

fn html_native_diagnostic_list_preserves_standard_codes_and_original_ranges() {
    let analyzer = Analyzer::with_profile(
        AnalyzerProfile::empty_lexicon().with_input_mode(InputMode::HtmlText),
    )
    .unwrap();
    let refusal = analyzer.projections("&bogus; &#xD800;").unwrap_err();
    let error: EvalError = refusal.into();
    let original = Capture::read(&error).child("cause").child("cause");
    assert_eq!(original.code, "text-html");
    assert!(matches!(
        original.events.first(),
        Some(Event::BeginList(Some("diagnostics"), 2))
    ));
    let diagnostics: Vec<_> = original
        .events
        .iter()
        .filter_map(|event| match event {
            Event::Value(None, ErrorValue::Record(value)) => Some(Capture::read(*value)),
            _ => None,
        })
        .collect();
    assert_eq!(diagnostics.len(), 2);
    assert_eq!(
        diagnostics[0].child("kind").code,
        "unknown-named-character-reference"
    );
    assert!(matches!(
        diagnostics[0].field("source_start"),
        ErrorValue::ByteCount(0)
    ));
    assert!(matches!(
        diagnostics[0].field("source_end"),
        ErrorValue::ByteCount(7)
    ));
    assert_eq!(
        diagnostics[1].child("kind").code,
        "surrogate-character-reference"
    );
    assert!(matches!(
        diagnostics[1].field("source_start"),
        ErrorValue::ByteCount(8)
    ));
    assert!(matches!(
        diagnostics[1].field("source_end"),
        ErrorValue::ByteCount(16)
    ));
}

fn wide_substring_fields_are_not_narrowed_by_native_cause_transport() {
    use purrdf_text::{SubstringLimits, SubstringRefusal, SubstringRefusalReason, SubstringWork};
    let error: EvalError = TextError::Substring(SubstringRefusal {
        reason: SubstringRefusalReason::VerificationBytes,
        counters: SubstringWork {
            posting_operations: u64::MAX,
            candidate_spans: u64::MAX,
            verification_bytes: u64::MAX,
        },
        limits: SubstringLimits {
            posting_operations: u64::MAX - 1,
            candidate_spans: u64::MAX - 1,
            verification_bytes: u64::MAX - 1,
        },
        distinct_spans: u64::MAX,
        generation: [0xa5; 32],
    })
    .into();
    let original = Capture::read(&error)
        .child("cause")
        .child("cause")
        .child("cause");
    assert!(
        matches!(original.field("distinct_spans"), ErrorValue::Unsigned(value) if value == u128::from(u64::MAX))
    );
    let work = original.child("counters");
    assert!(
        matches!(work.field("posting_operations"), ErrorValue::Unsigned(value) if value == u128::from(u64::MAX))
    );
    assert!(
        matches!(work.field("candidate_spans"), ErrorValue::Unsigned(value) if value == u128::from(u64::MAX))
    );
    assert!(
        matches!(work.field("verification_bytes"), ErrorValue::ByteCount(value) if value == u128::from(u64::MAX))
    );
    assert!(
        matches!(original.child("limits").field("verification_bytes"), ErrorValue::ByteCount(value) if value == u128::from(u64::MAX - 1))
    );
}

fn fixed_arithmetic_failures_keep_original_raw_operands_and_domain_values() {
    use purrdf_text::Fixed;
    let maximum = Fixed::from_raw(i128::MAX);
    let minimum = Fixed::from_raw(i128::MIN);
    for (result, code, left, right) in [
        (
            maximum.checked_add(Fixed::ONE),
            "text-fixed-add-overflow",
            i128::MAX,
            Fixed::ONE.into_raw(),
        ),
        (
            minimum.checked_sub(Fixed::ONE),
            "text-fixed-subtract-overflow",
            i128::MIN,
            Fixed::ONE.into_raw(),
        ),
        (
            maximum.checked_mul(maximum),
            "text-fixed-multiply-overflow",
            i128::MAX,
            i128::MAX,
        ),
        (
            maximum.checked_div(Fixed::from_raw(1)),
            "text-fixed-divide-overflow",
            i128::MAX,
            1,
        ),
    ] {
        let original = result.unwrap_err();
        assert!(matches!(&original, TextError::Overflow(_)));
        let error: EvalError = original.into();
        let cause = Capture::read(&error).child("cause").child("cause");
        assert_eq!(cause.code, code);
        assert!(matches!(cause.field("left_raw"), ErrorValue::Signed(value) if value == left));
        assert!(matches!(cause.field("right_raw"), ErrorValue::Signed(value) if value == right));
    }
    for (result, code, field, raw) in [
        (
            Fixed::ONE.checked_div(Fixed::ZERO),
            "text-fixed-division-by-zero",
            "divisor_raw",
            0,
        ),
        (minimum.ln(), "text-fixed-log-domain", "raw", i128::MIN),
    ] {
        let original = result.unwrap_err();
        assert!(matches!(&original, TextError::Domain(_)));
        let error: EvalError = original.into();
        let cause = Capture::read(&error).child("cause").child("cause");
        assert_eq!(cause.code, code);
        assert!(matches!(cause.field(field), ErrorValue::Signed(value) if value == raw));
    }
}

fn exact_integer_records_preserve_all_binary_limbs_without_rendering() {
    use purrdf_xsd::exact::Integer;
    // (2^127 - 1)^2 = 2^254 - 2^128 + 1, in base 2^64.
    let value = Integer::from_i128(i128::MAX)
        .try_mul(&Integer::from_i128(i128::MAX))
        .unwrap();
    let big = Capture::read(&value).child("value");
    assert_eq!(big.code, "xsd-binary-integer");
    assert!(matches!(big.field("negative"), ErrorValue::Boolean(false)));
    assert!(matches!(
        big.events.get(1),
        Some(Event::BeginList(Some("limbs"), 4))
    ));
    let limbs: Vec<_> = big
        .events
        .iter()
        .filter_map(|event| match event {
            Event::Value(None, ErrorValue::Unsigned(value)) => Some(*value),
            _ => None,
        })
        .collect();
    assert_eq!(
        limbs,
        [1, 0, u128::from(u64::MAX), u128::from((1_u64 << 62) - 1)]
    );
}

fn configuration_bounds_keep_the_requested_value_and_native_limit() {
    use purrdf_text::phonetic::PreparedDistance;
    let requested = usize::MAX;
    for (error, code, maximum) in [
        (
            AnalyzerProfile::new(requested).unwrap_err(),
            "text-config-token-bound",
            1024,
        ),
        (
            DoubleMetaphone::new(requested).unwrap_err(),
            "text-config-phonetic-code-length",
            64,
        ),
        (
            AnalyzerProfile::standard()
                .with_phonetics(4, requested)
                .unwrap_err(),
            "text-config-phonetic-distance",
            64,
        ),
        (
            PreparedDistance::new("a", requested).unwrap_err(),
            "text-config-edit-threshold",
            64,
        ),
    ] {
        assert!(matches!(&error, TextError::Config(_)));
        let error: EvalError = error.into();
        let cause = Capture::read(&error).child("cause").child("cause");
        assert_eq!(cause.code, code);
        assert!(
            matches!(cause.field("requested"), ErrorValue::Unsigned(value) if value == requested as u128)
        );
        assert!(matches!(cause.field("maximum"), ErrorValue::Unsigned(value) if value == maximum));
    }
}

fn configuration_ranking_keeps_signed_values_and_field_indices() {
    use purrdf_text::{Fixed, RankingField, RankingProfile};
    let error = RankingField::new("title", Fixed::from_raw(i128::MIN), Fixed::ZERO).unwrap_err();
    assert_eq!(
        error.to_string(),
        "invalid text-index configuration: a field needs a name and a nonnegative weight"
    );
    let error: EvalError = error.into();
    let cause = Capture::read(&error).child("cause").child("cause");
    assert_eq!(cause.code, "text-config-field-name-or-weight");
    assert!(matches!(cause.field("name"), ErrorValue::Text("title")));
    assert!(matches!(
        cause.field("weight_raw"),
        ErrorValue::Signed(i128::MIN)
    ));
    let field = RankingField::new("body", Fixed::ONE, Fixed::ZERO).unwrap();
    let error: EvalError = RankingProfile::new(vec![field], Vec::new(), Some(usize::MAX))
        .unwrap_err()
        .into();
    let cause = Capture::read(&error).child("cause").child("cause");
    assert_eq!(cause.code, "text-config-unclassified-field");
    assert!(
        matches!(cause.field("field"), ErrorValue::Unsigned(value) if value == usize::MAX as u128)
    );
    assert!(matches!(cause.field("fields"), ErrorValue::Unsigned(1)));
}

fn configuration_predicates_keep_terms_and_original_iri_causes() {
    use purrdf_core::TermValue;
    use purrdf_text::{GraphSelector, RankingProfile, TextIndexConfig};
    let error: EvalError = TextIndexConfig::new(
        vec![TermValue::iri("https://example.org/p")],
        GraphSelector::Named(TermValue::Blank {
            label: "graph".to_owned(),
            scope: purrdf_core::BlankScope(7),
        }),
        Analyzer::empty_lexicon(),
    )
    .unwrap_err()
    .into();
    let cause = Capture::read(&error).child("cause").child("cause");
    assert_eq!(cause.code, "text-config-named-graph");
    assert!(matches!(cause.field("graph"), ErrorValue::Term(_)));
    let error: EvalError = RankingProfile::single_field()
        .field_for(&TermValue::iri("https://example.org/a b"))
        .unwrap_err()
        .into();
    let cause = Capture::read(&error).child("cause").child("cause");
    assert_eq!(cause.code, "text-config-ranking-iri");
    assert!(matches!(
        cause.field("iri"),
        ErrorValue::Iri("https://example.org/a b")
    ));
    assert!(matches!(cause.field("cause"), ErrorValue::Cause(_)));
}

purrdf_testkit::harness_main!(
    phonetic_native_scalar_survives_evaluator_conversion,
    substring_native_refusal_preserves_counters_limits_and_generation,
    html_native_diagnostic_list_preserves_standard_codes_and_original_ranges,
    wide_substring_fields_are_not_narrowed_by_native_cause_transport,
    fixed_arithmetic_failures_keep_original_raw_operands_and_domain_values,
    exact_integer_records_preserve_all_binary_limbs_without_rendering,
    configuration_bounds_keep_the_requested_value_and_native_limit,
    configuration_ranking_keeps_signed_values_and_field_indices,
    configuration_predicates_keep_terms_and_original_iri_causes,
);
