// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Borrowed native TEXT refusals remain complete after their inputs disappear.

use purrdf_alloc_probe::CurrentThreadWindow;
use purrdf_hash::error::{ErrorValue, StructuredError};
use purrdf_testkit::error_capture::Capture;
use purrdf_text::{
    Analyzer, AnalyzerProfile, InputMode, SubstringLimits, SurfaceIndex, TextDeltaError, TextError,
};
use std::error::Error;

#[global_allocator]
static GLOBAL: purrdf_alloc_probe::CountingAllocator = purrdf_alloc_probe::CountingAllocator;

fn cause<'a>(fields: &Capture<'a>) -> &'a dyn StructuredError {
    let ErrorValue::Cause(cause) = fields.field("cause") else {
        panic!("the original refusal is a borrowed cause");
    };
    cause
}

#[test]
fn substring_first_excess_retains_generation_limits_and_work_after_index_drop() {
    let analyzer = Analyzer::with_profile(AnalyzerProfile::empty_lexicon().with_substring_limits(
        SubstringLimits {
            posting_operations: 3,
            candidate_spans: u64::MAX,
            verification_bytes: u64::MAX,
        },
    ))
    .unwrap();
    let index = SurfaceIndex::from_texts(analyzer, [(0, "abcd"), (1, "abcxbcd")]).unwrap();
    let generation = index.generation();
    let error = index.substring("abcd").unwrap_err();
    drop(index);
    let buffers = std::array::from_fn::<_, 5, _>(|_| Vec::with_capacity(5));
    let [root, refusal, work, limits, reason] = buffers;
    let window = CurrentThreadWindow::open();
    let copy = error.clone();
    drop(error);
    let root = Capture::read_with_buffer(&copy, root);
    let refusal = Capture::read_with_buffer(cause(&root), refusal);
    let ErrorValue::Record(value) = refusal.field("counters") else {
        panic!("work is a native record");
    };
    let work = Capture::read_with_buffer(value, work);
    let ErrorValue::Record(value) = refusal.field("limits") else {
        panic!("limits are a native record");
    };
    let limits = Capture::read_with_buffer(value, limits);
    let ErrorValue::Record(value) = refusal.field("reason") else {
        panic!("reason is a native record");
    };
    let reason = Capture::read_with_buffer(value, reason);
    let measured = window.close();
    assert_eq!(measured.allocations, 0);
    assert_eq!(measured.requested_bytes, 0);
    assert_eq!(root.code, "text-substring");
    assert_eq!(reason.code, "text-substring-posting-operations");
    assert!(matches!(
        work.field("posting_operations"),
        ErrorValue::Unsigned(4)
    ));
    assert!(matches!(
        work.field("candidate_spans"),
        ErrorValue::Unsigned(1)
    ));
    assert!(matches!(
        work.field("verification_bytes"),
        ErrorValue::ByteCount(0)
    ));
    assert!(matches!(
        limits.field("posting_operations"),
        ErrorValue::Unsigned(3)
    ));
    assert!(
        matches!(limits.field("candidate_spans"), ErrorValue::Unsigned(value) if value == u128::from(u64::MAX))
    );
    assert!(
        matches!(limits.field("verification_bytes"), ErrorValue::ByteCount(value) if value == u128::from(u64::MAX))
    );
    assert!(matches!(
        refusal.field("distinct_spans"),
        ErrorValue::Unsigned(2)
    ));
    assert!(matches!(refusal.field("generation"), ErrorValue::Bytes(value) if value == generation));
    assert!(
        copy.source()
            .unwrap()
            .downcast_ref::<purrdf_text::SubstringRefusal>()
            .is_some()
    );
}

#[test]
fn phonetic_original_scalar_and_first_excess_survive_input_disposal() {
    use purrdf_text::phonetic::{MAX_INPUT_SCALARS, canonicalize};

    let input = String::from("中");
    let error = TextError::from(canonicalize(&input).unwrap_err());
    drop(input);
    let root = Vec::with_capacity(1);
    let fields = Vec::with_capacity(1);
    let window = CurrentThreadWindow::open();
    let copy = error.clone();
    drop(error);
    let root = Capture::read_with_buffer(&copy, root);
    let fields = Capture::read_with_buffer(cause(&root), fields);
    let measured = window.close();
    assert_eq!(measured.allocations, 0);
    assert_eq!(measured.requested_bytes, 0);
    assert_eq!(root.code, "text-phonetic");
    assert_eq!(fields.code, "text-phonetic-unsupported-scalar");
    assert!(matches!(
        fields.field("scalar"),
        ErrorValue::Character('中')
    ));
    assert!(
        copy.source()
            .unwrap()
            .downcast_ref::<purrdf_text::phonetic::PhoneticRefusal>()
            .is_some()
    );

    let input = "a".repeat(MAX_INPUT_SCALARS + 1);
    let error = canonicalize(&input).unwrap_err();
    drop(input);
    let fields = Capture::read(&error);
    assert_eq!(fields.code, "text-phonetic-input-too-long");
    assert!(
        matches!(fields.field("limit"), ErrorValue::Unsigned(value) if value == MAX_INPUT_SCALARS as u128)
    );
    assert!(
        matches!(fields.field("observed"), ErrorValue::Unsigned(value) if value == MAX_INPUT_SCALARS as u128 + 1)
    );
    assert!(canonicalize("Smith").is_ok());
}

#[test]
fn html_diagnostics_preserve_encounter_order_and_original_ranges() {
    let analyzer = Analyzer::with_profile(
        AnalyzerProfile::empty_lexicon().with_input_mode(InputMode::HtmlText),
    )
    .unwrap();
    let input = String::from("é&#0; &nope;");
    let error = analyzer.terms(&input).unwrap_err();
    drop(input);
    drop(analyzer);
    let expected_display = error.to_string();
    let TextError::Html(original) = error.clone() else {
        panic!("resident HTML diagnostics retain their owner");
    };
    let buffer = Vec::with_capacity(4);
    let window = CurrentThreadWindow::open();
    let converted: purrdf_sparql_eval::EvalError = error.into();
    let copy = converted.clone();
    drop(converted);
    let purrdf_sparql_eval::EvalError::RetainedNativeFailure(retained) = &copy else {
        panic!("conversion retains the original diagnostic owner");
    };
    assert!(core::ptr::eq(original.cause(), retained.cause()));
    let root = Capture::read_with_buffer(retained.cause(), buffer);
    let measured = window.close();
    assert_eq!(measured.allocations, 0);
    assert_eq!(measured.requested_bytes, 0);
    assert_eq!(root.code, "text-html-diagnostics");
    assert_eq!(
        retained.kind(),
        purrdf_sparql_eval::NativeDiagnosticKind::Data
    );
    assert_eq!(
        copy.to_string(),
        purrdf_sparql_eval::EvalError::data(expected_display).to_string()
    );
    let diagnostics = root.records("diagnostics");
    assert_eq!(diagnostics.len(), 2);
    for (diagnostic, start, end, kind) in [
        (&diagnostics[0], 2, 6, "null-character-reference"),
        (&diagnostics[1], 7, 13, "unknown-named-character-reference"),
    ] {
        assert_eq!(diagnostic.code, "html-character-reference");
        assert!(
            matches!(diagnostic.field("source_start"), ErrorValue::ByteCount(value) if value == start)
        );
        assert!(
            matches!(diagnostic.field("source_end"), ErrorValue::ByteCount(value) if value == end)
        );
        let ErrorValue::Cause(value) = diagnostic.field("kind") else {
            panic!("the HTML kind is the original native cause");
        };
        assert_eq!(Capture::read(value).code, kind);
    }
    assert!(
        retained.source().unwrap().source().is_none(),
        "the diagnostic list owns the records without selecting one as its source"
    );
}

#[test]
fn delta_coverage_keeps_original_term_and_full_width_populations() {
    let predicate = String::from("https://example.org/original/predicate");
    let error = TextDeltaError::CoverageMismatch {
        predicate: Box::new(purrdf_core::TermValue::iri(&predicate)),
        before: u128::MAX,
        added: u128::MAX - 1,
        removed: u128::MAX - 2,
        observed: u128::MAX - 3,
    };
    drop(predicate);
    let buffer = Vec::with_capacity(5);
    let window = CurrentThreadWindow::open();
    let fields = Capture::read_with_buffer(&error, buffer);
    let measured = window.close();
    assert_eq!(measured.allocations, 0);
    assert_eq!(measured.requested_bytes, 0);
    for (name, expected) in [
        ("before", u128::MAX),
        ("added", u128::MAX - 1),
        ("removed", u128::MAX - 2),
        ("observed", u128::MAX - 3),
    ] {
        assert!(matches!(fields.field(name), ErrorValue::Unsigned(value) if value == expected));
    }
    let ErrorValue::Term(value) = fields.field("predicate") else {
        panic!("the predicate remains a borrowed RDF term");
    };
    let predicate = Capture::read(value);
    assert_eq!(predicate.code, "rdf-term-iri");
    assert!(matches!(predicate.field("iri"), ErrorValue::Iri(value)
        if value == "https://example.org/original/predicate"));
    assert!(error.source().is_none());
}

#[test]
fn arithmetic_operands_survive_cloning_and_evaluator_transport_without_inspection_allocations() {
    use purrdf_text::Fixed;
    let error = Fixed::from_raw(i128::MIN)
        .checked_sub(Fixed::from_raw(1))
        .unwrap_err();
    let buffers = [
        Vec::with_capacity(2),
        Vec::with_capacity(2),
        Vec::with_capacity(2),
    ];
    let [root, retained, fields] = buffers;
    let window = CurrentThreadWindow::open();
    let copy = error.clone();
    drop(error);
    let root = Capture::read_with_buffer(&copy, root);
    let retained = Capture::read_with_buffer(cause(&root), retained);
    let native = cause(&retained);
    let fields = Capture::read_with_buffer(native, fields);
    let measured = window.close();
    assert_eq!(measured.allocations, 0);
    assert_eq!(fields.code, "text-fixed-subtract-overflow");
    assert!(matches!(
        fields.field("left_raw"),
        ErrorValue::Signed(i128::MIN)
    ));
    assert!(matches!(fields.field("right_raw"), ErrorValue::Signed(1)));
    let converted: purrdf_sparql_eval::EvalError = copy.clone().into();
    let purrdf_sparql_eval::EvalError::RetainedNativeFailure(retained) = converted else {
        panic!("original arithmetic transport");
    };
    assert!(core::ptr::eq(native, retained.cause()));
}

#[test]
fn prepared_ranking_retains_normalization_inputs_and_score_bound() {
    use purrdf_text::{FieldInput, Fixed, PreparedCorpus, RankingField, RankingProfile};
    let profile = RankingProfile::new(
        vec![RankingField::new("body", Fixed::ONE, Fixed::ONE).unwrap()],
        Vec::new(),
        Some(0),
    )
    .unwrap();
    let corpus = PreparedCorpus::new(&profile, 2, &[u128::from(u64::MAX)]).unwrap();
    let query = corpus.prepare_query(&[("needle", 1)]).unwrap();
    let error = query
        .contribution(
            0,
            &[FieldInput {
                term_frequency: 1,
                length: 1,
            }],
        )
        .unwrap_err();
    let root = Capture::read(&error);
    let retained = Capture::read(cause(&root));
    let fields = Capture::read(cause(&retained));
    assert_eq!(fields.code, "text-fixed-field-normalization");
    assert!(
        matches!(fields.field("b_raw"), ErrorValue::Signed(value) if value == Fixed::ONE.into_raw())
    );
    assert!(matches!(
        fields.field("relative_raw"),
        ErrorValue::Signed(0)
    ));
    assert!(matches!(
        fields.field("normalized_raw"),
        ErrorValue::Signed(0)
    ));
    let error = query
        .score_bound()
        .validate(Fixed::from_raw(-1))
        .unwrap_err();
    let root = Capture::read(&error);
    let retained = Capture::read(cause(&root));
    let fields = Capture::read(cause(&retained));
    assert_eq!(fields.code, "text-fixed-score-outside-bound");
    assert!(matches!(fields.field("score_raw"), ErrorValue::Signed(-1)));
    assert!(
        matches!(fields.field("maximum_raw"), ErrorValue::Signed(value) if value == query.score_bound().maximum().into_raw())
    );
}
