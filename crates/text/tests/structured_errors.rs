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

fn delta_failures_keep_original_identities_counts_and_nested_causes() {
    use purrdf_text::TextDeltaError;
    let expected = [0x17; 32];
    let actual = [0xe3; 32];
    for error in [
        TextDeltaError::MissingAddition {
            ordinal: usize::MAX,
        },
        TextDeltaError::SourceMismatch { expected, actual },
        TextDeltaError::AnalyzerMismatch { expected, actual },
        TextDeltaError::SourceOverlap {
            document: u32::MAX,
            identity: actual,
        },
        TextDeltaError::TermCapacity {
            observed: u128::MAX,
            limit: u32::MAX,
            cause: u32::try_from(u128::MAX).unwrap_err(),
        },
        TextDeltaError::Indexing(TextError::data("original caller data")),
    ] {
        let retained = error.clone();
        drop(error);
        let buffer = Vec::with_capacity(32);
        let window = CurrentThreadWindow::open();
        let capture = Capture::read_with_buffer(&retained, buffer);
        assert_eq!(window.close().allocations, 0);
        match capture.code {
            "text-delta-missing-addition" => assert!(matches!(
                capture.field("ordinal"), ErrorValue::Unsigned(value) if value == usize::MAX as u128
            )),
            "text-delta-source-mismatch" | "text-delta-analyzer-mismatch" => {
                assert!(
                    matches!(capture.field("expected"), ErrorValue::Bytes(value) if value == expected)
                );
                assert!(
                    matches!(capture.field("actual"), ErrorValue::Bytes(value) if value == actual)
                );
            }
            "text-delta-source-overlap" => {
                assert!(
                    matches!(capture.field("document"), ErrorValue::Unsigned(value) if value == u128::from(u32::MAX))
                );
                assert!(
                    matches!(capture.field("identity"), ErrorValue::Bytes(value) if value == actual)
                );
            }
            "text-delta-term-capacity" => {
                assert!(matches!(
                    capture.field("observed"),
                    ErrorValue::Unsigned(u128::MAX)
                ));
                assert!(
                    matches!(capture.field("limit"), ErrorValue::Unsigned(value) if value == u128::from(u32::MAX))
                );
                assert_eq!(capture.child("cause").code, "integer-conversion-range");
                assert!(
                    std::error::Error::source(&retained)
                        .unwrap()
                        .is::<std::num::TryFromIntError>()
                );
            }
            "text-delta-indexing" => assert_eq!(capture.child("cause").code, "text-data"),
            unexpected => panic!("unexpected delta record: {unexpected}"),
        }
    }
}

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

fn data_corpus_refusal_preserves_full_width_totals_and_population() {
    use purrdf_text::{PreparedCorpus, RankingProfile};
    let profile = RankingProfile::single_field();
    let error: EvalError = PreparedCorpus::new(&profile, u64::MAX, &[u128::MAX])
        .unwrap_err()
        .into();
    let cause = Capture::read(&error).child("cause").child("cause");
    assert_eq!(cause.code, "text-data-corpus-bound");
    assert!(matches!(cause.field("field"), ErrorValue::Unsigned(0)));
    assert!(matches!(
        cause.field("total"),
        ErrorValue::Unsigned(u128::MAX)
    ));
    assert!(
        matches!(cause.field("population"), ErrorValue::Unsigned(value) if value == u128::from(u64::MAX))
    );
    assert!(
        matches!(cause.field("documents"), ErrorValue::Unsigned(value) if value == u128::from(u64::MAX))
    );
}

fn data_query_refusals_keep_original_order_and_exact_frequency_totals() {
    use purrdf_text::{PreparedCorpus, RankingProfile};
    let profile = RankingProfile::single_field();
    let corpus = PreparedCorpus::new(&profile, 1, &[1]).unwrap();
    let error: EvalError = corpus
        .prepare_query(&[("b", 0), ("a", 0)])
        .unwrap_err()
        .into();
    let cause = Capture::read(&error).child("cause").child("cause");
    assert_eq!(cause.code, "text-data-query-order");
    assert!(matches!(cause.field("term"), ErrorValue::Text("a")));
    assert!(matches!(cause.field("previous"), ErrorValue::Text("b")));
    let error: EvalError = corpus.prepare_query(&[("a", 2)]).unwrap_err().into();
    let cause = Capture::read(&error).child("cause").child("cause");
    assert_eq!(cause.code, "text-data-query-frequency-sum");
    assert!(matches!(
        cause.child("sum").child("raw").field("value"),
        ErrorValue::Signed(2)
    ));
    assert!(matches!(
        cause.child("total").child("raw").field("value"),
        ErrorValue::Signed(1)
    ));
    assert!(matches!(cause.field("unit"), ErrorValue::Code("tokens")));
}

fn data_scoring_refusals_keep_original_ordinal_and_field_facts() {
    use purrdf_text::{FieldInput, PreparedCorpus, RankingProfile};
    let profile = RankingProfile::single_field();
    let corpus = PreparedCorpus::new(&profile, 1, &[2]).unwrap();
    let query = corpus.prepare_query(&[("a", 1)]).unwrap();
    let error: EvalError = query.contribution(usize::MAX, &[]).unwrap_err().into();
    let cause = Capture::read(&error).child("cause").child("cause");
    assert_eq!(cause.code, "text-data-term-ordinal");
    assert!(
        matches!(cause.field("ordinal"), ErrorValue::Unsigned(value) if value == usize::MAX as u128)
    );
    assert!(matches!(cause.field("terms"), ErrorValue::Unsigned(1)));
    let error = query
        .contribution(
            0,
            &[FieldInput {
                term_frequency: 3,
                length: 2,
            }],
        )
        .unwrap_err();
    assert_eq!(
        error.to_string(),
        "text-index input error: field frequency, length, and corpus total are inconsistent"
    );
    let error: EvalError = error.into();
    let cause = Capture::read(&error).child("cause").child("cause");
    assert_eq!(cause.code, "text-data-field-facts");
    assert!(matches!(cause.field("frequency"), ErrorValue::Unsigned(3)));
    assert!(matches!(cause.field("length"), ErrorValue::Unsigned(2)));
    assert!(matches!(cause.field("total"), ErrorValue::Unsigned(2)));
}

fn source_read_failure_retains_the_actual_backend_error_through_conversion_and_clone() {
    use purrdf_core::{TermValue, TripleDatasetBuilder, TripleReadError};
    use purrdf_text::{GraphSelector, TextIndex, TextIndexConfig};
    let dataset = TripleDatasetBuilder::default().freeze().unwrap();
    let read = dataset.read().unwrap();
    let config = TextIndexConfig::new(
        vec![TermValue::iri("https://example.org/p")],
        GraphSelector::Named(TermValue::iri("https://example.org/g")),
        Analyzer::empty_lexicon(),
    )
    .unwrap();
    let error = TextIndex::from_dataset(&read, &config).unwrap_err();
    assert!(matches!(&error, TextError::SourceRead(_)));
    assert_eq!(
        error.to_string(),
        "text-index source read error: the triple-only carrier does not support named graphs"
    );
    let error: EvalError = error.into();
    assert!(error.is_native_storage_failure());
    assert_eq!(error.code(), Some("native-sparql-source-read"));
    let clone = error.clone();
    let source = std::error::Error::source(&error).unwrap().source().unwrap();
    let cloned_source = std::error::Error::source(&clone).unwrap().source().unwrap();
    assert!(matches!(
        source.downcast_ref::<TripleReadError>(),
        Some(TripleReadError::NamedGraphsUnsupported)
    ));
    assert!(core::ptr::eq(source, cloned_source));
    drop(error);
    let cause = Capture::read(&clone).child("cause").child("cause");
    assert_eq!(cause.code, "triple-read-named-graphs-unsupported");

    let empty = purrdf_core::RdfDatasetBuilder::new().freeze().unwrap();
    let index = TextIndex::from_dataset(&*empty, &config).unwrap();
    let delta = index.with_delta(&read, &*empty, &*empty).unwrap_err();
    assert_eq!(
        delta.to_string(),
        "text-index source read error: the triple-only carrier does not support named graphs"
    );
    let retained = delta.clone();
    drop(delta);
    let capture = Capture::read(&retained);
    assert_eq!(capture.code, "text-delta-indexing");
    assert_eq!(capture.child("cause").code, "text-source-read");
    assert_eq!(
        capture.child("cause").child("cause").child("cause").code,
        "triple-read-named-graphs-unsupported"
    );
}

fn ledger_source_errors_preserve_nested_node_keys_and_native_reason_refusals() {
    use purrdf_core::TermValue;
    use purrdf_core::evidence::{
        Information, LedgerError, LedgerNodeKey, LedgerReadError, ReasonDescription, StandpointKey,
    };
    use purrdf_hash::hex::Digest32;
    use purrdf_lex::allocation::{Memory, Resident};
    // A constructed missing-node control exercises the complete retained key.
    let error = LedgerReadError::Declaration(LedgerError::MissingNode(LedgerNodeKey::Information(
        StandpointKey::Declared(Digest32::new([7; 32])),
        Information::Both,
    )));
    let key = Capture::read(&error).child("cause").child("key");
    assert_eq!(key.code, "ledger-node-information");
    assert!(
        matches!(key.child("standpoint").field("identity"), ErrorValue::Bytes(bytes) if bytes == [7; 32])
    );
    assert_eq!(key.child("information").code, "evidence-information-both");
    // The reason refusal itself is produced by native identity admission.
    let iri = TermValue::iri("https://example.org/cause");
    let description = ReasonDescription {
        cause: &iri,
        concern: &iri,
        constraint: None,
        vault: None,
        watermark: None,
    };
    let mut resident = Resident;
    let refusal = description
        .identity(&mut Memory::new(&mut resident))
        .unwrap_err();
    let error = LedgerError::Reason(refusal);
    let cause = Capture::read(&error).child("cause");
    assert_eq!(cause.code, "evidence-reason-concern-must-be-triple");
}

fn baseline_configuration_keeps_artifact_counts_hashes_and_missing_flags() {
    let error: EvalError = Analyzer::resolve(AnalyzerProfile::empty_lexicon(), &[b"unwanted"])
        .unwrap_err()
        .into();
    let cause = Capture::read(&error).child("cause").child("cause");
    assert_eq!(cause.code, "text-config-empty-lexicon-artifacts");
    assert!(matches!(cause.field("count"), ErrorValue::Unsigned(1)));
    let bytes = b"unknown artifact";
    let identity = *purrdf_hash::blake3::hash(bytes).as_bytes();
    let error: EvalError = Analyzer::resolve(AnalyzerProfile::standard(), &[bytes])
        .unwrap_err()
        .into();
    let cause = Capture::read(&error).child("cause").child("cause");
    assert_eq!(cause.code, "text-config-unexpected-baseline");
    assert!(matches!(cause.field("identity"), ErrorValue::Bytes(value) if value == identity));
    let error: EvalError = Analyzer::resolve(AnalyzerProfile::standard(), &[])
        .unwrap_err()
        .into();
    let cause = Capture::read(&error).child("cause").child("cause");
    assert_eq!(cause.code, "text-config-missing-baselines");
    assert_eq!(
        cause
            .events
            .iter()
            .filter(|event| matches!(event, Event::Value(None, ErrorValue::Boolean(false))))
            .count(),
        5
    );
}

fn dictionary_configuration_keeps_conflicting_costs_and_emoji_byte_range() {
    use purrdf_text::segment::Dictionary;
    let error: EvalError =
        Dictionary::with_costs([("river".to_owned(), 17), ("river".to_owned(), 3)])
            .unwrap_err()
            .into();
    let cause = Capture::read(&error).child("cause").child("cause");
    assert_eq!(cause.code, "text-dictionary-conflicting-costs");
    assert!(matches!(cause.field("word"), ErrorValue::Text("river")));
    assert!(matches!(cause.field("first"), ErrorValue::Unsigned(3)));
    assert!(matches!(cause.field("second"), ErrorValue::Unsigned(17)));
    let error: EvalError = Dictionary::canonical_key("a😀b").unwrap_err().into();
    let cause = Capture::read(&error).child("cause").child("cause");
    assert_eq!(cause.code, "text-dictionary-protected-emoji");
    assert!(matches!(cause.field("start"), ErrorValue::Unsigned(1)));
    assert!(matches!(cause.field("end"), ErrorValue::Unsigned(5)));
}

fn dictionary_artifact_refusals_keep_original_identity_header_and_utf8_values() {
    use purrdf_text::segment::Dictionary;
    for (bytes, code, field, value) in [
        (vec![0x85], "text-dictionary-artifact-arity", "items", 5),
        (
            vec![0x98, 6],
            "text-dictionary-artifact-head-canonical",
            "bytes",
            2,
        ),
        (
            vec![0x86, 0x62, b'x'],
            "text-dictionary-artifact-text-truncated",
            "requested",
            2,
        ),
        (
            vec![0x86, 0x61, 0xff],
            "text-dictionary-artifact-utf8",
            "valid_up_to",
            0,
        ),
    ] {
        let identity = *purrdf_hash::blake3::hash(&bytes).as_bytes();
        let error: EvalError = Dictionary::from_artifact(identity, &bytes)
            .unwrap_err()
            .into();
        let cause = Capture::read(&error).child("cause").child("cause");
        assert_eq!(cause.code, code);
        assert!(matches!(cause.field(field), ErrorValue::Unsigned(actual) if actual == value));
    }
    let bytes = b"artifact";
    let actual = *purrdf_hash::blake3::hash(bytes).as_bytes();
    let error: EvalError = Dictionary::from_artifact([0; 32], bytes)
        .unwrap_err()
        .into();
    let cause = Capture::read(&error).child("cause").child("cause");
    assert_eq!(cause.code, "text-dictionary-artifact-identity");
    assert!(matches!(cause.field("expected"), ErrorValue::Bytes(value) if value == [0; 32]));
    assert!(matches!(cause.field("actual"), ErrorValue::Bytes(value) if value == actual));
}

fn binding_configuration_failure_retains_both_semantic_configurations() {
    use purrdf_core::{RdfDatasetBuilder, TermValue};
    use purrdf_text::{GraphSelector, TextIndex, TextIndexConfig, verify_binding};
    let dataset = RdfDatasetBuilder::new().freeze().unwrap();
    let config = TextIndexConfig::new(
        vec![TermValue::iri("https://example.org/p")],
        GraphSelector::Default,
        Analyzer::empty_lexicon(),
    )
    .unwrap();
    let index = TextIndex::from_dataset(&dataset, &config).unwrap();
    let supplied = config.clone().with_surface_index(false);
    let error: EvalError = verify_binding(&index, &dataset, &supplied)
        .unwrap_err()
        .into();
    let cause = Capture::read(&error).child("cause").child("cause");
    assert_eq!(cause.code, "text-config-binding-mismatch");
    let expected = cause.child("expected");
    let supplied = cause.child("supplied");
    assert!(matches!(
        expected.field("surface"),
        ErrorValue::Boolean(true)
    ));
    assert!(matches!(
        supplied.field("surface"),
        ErrorValue::Boolean(false)
    ));
    assert!(matches!(
        expected.field("graph_scope"),
        ErrorValue::Code("default")
    ));
    assert_eq!(expected.child("projection").code, "text-projection-lexical");
    let identity = config.analyzer().fingerprint();
    assert!(
        matches!(expected.field("analyzer_identity"), ErrorValue::Bytes(value) if value == identity)
    );
    assert!(
        matches!(supplied.field("analyzer_identity"), ErrorValue::Bytes(value) if value == identity)
    );
    assert!(
        expected
            .events
            .iter()
            .any(|event| matches!(event, Event::BeginList(Some("predicates"), 1)))
    );
}

fn search_input_refusals_keep_actual_cardinality_and_observed_scalar_limit() {
    use purrdf_text::phonetic::{MAX_INPUT_SCALARS, bounded_edit_distance};
    let input = "東".repeat(MAX_INPUT_SCALARS + 20);
    let error: EvalError = bounded_edit_distance(&input, "valid", 1)
        .unwrap_err()
        .into();
    let cause = Capture::read(&error).child("cause").child("cause");
    assert_eq!(cause.code, "text-data-edit-input");
    assert!(
        matches!(cause.field("observed"), ErrorValue::Unsigned(value) if value == (MAX_INPUT_SCALARS + 1) as u128)
    );
    assert!(matches!(
        cause.field("complete_count"),
        ErrorValue::Boolean(false)
    ));
    assert!(
        matches!(cause.field("limit"), ErrorValue::Unsigned(value) if value == MAX_INPUT_SCALARS as u128)
    );
    let index = SurfaceIndex::from_texts(Analyzer::empty_lexicon(), [(0, "healthy")]).unwrap();
    for (input, count) in [("", 0), ("two words", 2)] {
        let error: EvalError = index.substring(input).unwrap_err().into();
        let cause = Capture::read(&error).child("cause").child("cause");
        assert_eq!(cause.code, "text-data-substring-terms");
        assert!(matches!(cause.field("count"), ErrorValue::Unsigned(value) if value == count));
    }
    assert!(index.substring_exhaustive("healthy").is_ok());
}

fn relation_failures_retain_native_terms_rank_flags_arities_and_query_owners() {
    use purrdf_core::{RdfDatasetBuilder, TermValue, datatype::XSD_INTEGER};
    use purrdf_sparql_eval::{PfArgs, PropertyFunction};
    use purrdf_text::{
        GraphSelector, TermOccurrenceRelation, TextIndex, TextIndexConfig, TextSearchRelation,
    };
    let dataset = RdfDatasetBuilder::new().freeze().unwrap();
    let config = TextIndexConfig::new(
        vec![TermValue::iri("https://example.org/p")],
        GraphSelector::Default,
        Analyzer::empty_lexicon(),
    )
    .unwrap();
    let index = std::sync::Arc::new(TextIndex::from_dataset(&dataset, &config).unwrap());
    let search = TextSearchRelation::new(index.clone());
    let occurrence = TermOccurrenceRelation::new(index);
    let fail = |relation: &dyn PropertyFunction, args: &[Option<&TermValue>]| {
        let (subject, object) = args.split_at(1);
        match relation.open(&PfArgs::new(subject, object), None) {
            Err(error) => error,
            Ok(_) => panic!("the invalid invocation must refuse"),
        }
    };
    let integer = TermValue::typed_literal("3", XSD_INTEGER);
    let iri = TermValue::iri("https://example.org/input");
    let text = TermValue::simple_literal("alpha");
    for (value, code) in [
        (&integer, "text-call-needle-datatype"),
        (&iri, "text-call-needle-kind"),
    ] {
        let error = fail(&search, &[None, Some(value), None, None, None, None]);
        let cause = Capture::read(&error).child("cause").child("cause");
        assert_eq!(cause.code, code);
        assert!(matches!(cause.field("position"), ErrorValue::Unsigned(1)));
        if code == "text-call-needle-datatype" {
            assert!(
                matches!(cause.field("datatype"), ErrorValue::Iri(value) if value == XSD_INTEGER)
            );
        } else {
            assert!(matches!(
                cause.child("value").field("iri"),
                ErrorValue::Iri("https://example.org/input")
            ));
        }
    }
    let rank = TermValue::typed_literal("-00042", XSD_INTEGER);
    let error = fail(&search, &[None, Some(&text), None, Some(&rank), None, None]);
    let cause = Capture::read(&error).child("cause").child("cause");
    assert_eq!(cause.code, "text-call-rank-nonpositive");
    assert!(matches!(cause.field("lexical"), ErrorValue::Text("-00042")));
    assert!(matches!(cause.field("negative"), ErrorValue::Boolean(true)));
    assert!(matches!(cause.field("zero"), ErrorValue::Boolean(false)));
    for (rank, code) in [
        (iri.clone(), "text-call-rank-kind"),
        (text.clone(), "text-call-rank-datatype"),
        (
            TermValue::typed_literal("1e3", XSD_INTEGER),
            "text-call-rank-lexical",
        ),
        (
            TermValue::typed_literal("-0", XSD_INTEGER),
            "text-call-rank-nonpositive",
        ),
    ] {
        let error = fail(&search, &[None, Some(&text), None, Some(&rank), None, None]);
        let cause = Capture::read(&error).child("cause").child("cause");
        assert_eq!(cause.code, code);
        assert!(matches!(cause.field("position"), ErrorValue::Unsigned(3)));
        if code == "text-call-rank-nonpositive" {
            assert!(matches!(cause.field("negative"), ErrorValue::Boolean(true)));
            assert!(matches!(cause.field("zero"), ErrorValue::Boolean(true)));
        }
    }
    let error = fail(&search, &[None, Some(&text), None, None, Some(&iri), None]);
    let cause = Capture::read(&error).child("cause").child("cause");
    assert_eq!(cause.code, "text-call-language-kind");
    assert!(matches!(cause.field("position"), ErrorValue::Unsigned(4)));
    assert_eq!(cause.child("value").code, "rdf-term-iri");
    for (relation, args, code) in [
        (
            &search as &dyn PropertyFunction,
            vec![None; 6],
            "text-call-search-unbound",
        ),
        (
            &occurrence as &dyn PropertyFunction,
            vec![None; 4],
            "text-call-occurrence-unbound",
        ),
    ] {
        let error = fail(relation, &args);
        assert_eq!(
            Capture::read(&error).child("cause").child("cause").code,
            code
        );
    }
    let error = fail(&search, &[None, Some(&text)]);
    let cause = Capture::read(&error).child("cause").child("cause");
    assert_eq!(cause.code, "text-call-arity");
    assert!(matches!(
        cause.child("declared").field("object"),
        ErrorValue::Unsigned(5)
    ));
    assert!(matches!(
        cause.child("supplied").field("object"),
        ErrorValue::Unsigned(1)
    ));
    let words = TermValue::simple_literal("ALPHA beta");
    let error = fail(&occurrence, &[None, Some(&words), None, None]);
    let clone = error.clone();
    drop(error);
    let cause = Capture::read(&clone).child("cause").child("cause");
    assert_eq!(cause.code, "text-call-occurrence-terms");
    assert!(matches!(
        cause.field("text"),
        ErrorValue::Text("ALPHA beta")
    ));
    assert!(matches!(cause.field("count"), ErrorValue::Unsigned(2)));
    let terms: Vec<_> = cause
        .events
        .iter()
        .filter_map(|event| match event {
            Event::Value(None, ErrorValue::Text(term)) => Some(*term),
            _ => None,
        })
        .collect();
    assert_eq!(terms, ["alpha", "beta"]);
    let scoped =
        purrdf_core::query_scope::LexicalQuery::encode(text.clone(), Some(iri.clone())).unwrap();
    let error = fail(&search, &[None, Some(&scoped), None, None, None, None]);
    let cause = Capture::read(&error).child("cause").child("cause");
    assert_eq!(cause.code, "text-call-field-scope");
    assert!(matches!(
        cause.child("predicate").field("iri"),
        ErrorValue::Iri("https://example.org/input")
    ));
    let malformed = TermValue::Triple {
        s: purrdf_core::TermBox::new(iri.clone()),
        p: purrdf_core::TermBox::new(iri.clone()),
        o: purrdf_core::TermBox::new(text),
    };
    let error = fail(&search, &[None, Some(&malformed), None, None, None, None]);
    let cause = Capture::read(&error).child("cause").child("cause");
    assert_eq!(cause.code, "text-call-lexical-scope");
    assert_eq!(cause.child("cause").code, "lexical-scope-invalid-encoding");
    let subject = [None];
    let object = [Some(&iri), None, None, None, None];
    let Err(error) = purrdf_sparql_eval::property_fn::open_contained(
        &search,
        "https://example.org/search",
        &PfArgs::new(&subject, &object),
        None,
    ) else {
        panic!("a nonliteral needle must refuse at the public contained boundary");
    };
    assert_eq!(error.code(), Some(EvalError::FUNCTION_OPERATIONAL_CODE));
    let cause = Capture::read(&error).child("cause").child("cause");
    assert_eq!(cause.code, "text-call-needle-kind");
    assert!(matches!(
        cause.child("value").field("iri"),
        ErrorValue::Iri("https://example.org/input")
    ));
}

fn phrase_and_dictionary_refusals_keep_registration_input_and_analyzer_identities() {
    use purrdf_core::{RdfDatasetBuilder, TermValue};
    use purrdf_sparql_eval::{PfArgs, PropertyFunction};
    use purrdf_text::{
        AccentFold, GraphSelector, PhraseOccurrenceRelation, TextIndex, TextIndexConfig,
    };
    let dataset = RdfDatasetBuilder::new().freeze().unwrap();
    let analyzer = Analyzer::empty_lexicon();
    let config = TextIndexConfig::new(
        vec![TermValue::iri("https://example.org/p")],
        GraphSelector::Default,
        analyzer,
    )
    .unwrap();
    let index = std::sync::Arc::new(TextIndex::from_dataset(&dataset, &config).unwrap());
    let phrase = PhraseOccurrenceRelation::new(index.clone());
    let iri = TermValue::iri("https://example.org/invalid");
    let subject = [None];
    for value in [None, Some(&iri)] {
        let object = [value, None, None, None];
        let error = phrase
            .open(&PfArgs::new(&subject, &object), None)
            .err()
            .expect("a phrase requires a plain bound string");
        let cause = Capture::read(&error).child("cause").child("cause");
        assert_eq!(cause.code, "text-call-phrase-binding");
        if value.is_some() {
            assert_eq!(cause.child("value").code, "rdf-term-iri");
        } else {
            assert!(matches!(cause.field("value"), ErrorValue::Absent));
        }
    }
    let error = phrase
        .open(&PfArgs::new(&subject, &[]), None)
        .err()
        .expect("the phrase arity is explicit");
    let cause = Capture::read(&error).child("cause").child("cause");
    assert_eq!(cause.code, "text-call-phrase-arity");
    assert!(matches!(
        cause.child("supplied").field("object"),
        ErrorValue::Unsigned(0)
    ));
    let error = phrase.on_predicate(&iri).unwrap_err();
    let cause = Capture::read(&error).child("cause").child("cause");
    assert_eq!(cause.code, "text-call-phrase-predicate");
    assert!(matches!(
        cause.child("predicate").field("iri"),
        ErrorValue::Iri("https://example.org/invalid")
    ));
    let workspace = WorkspaceCapability::resident();
    let dictionary = purrdf_text::dictionary::Dictionary::compile(&index, &[], &workspace).unwrap();
    let other_config = TextIndexConfig::new(
        config.predicates().to_vec(),
        GraphSelector::Default,
        Analyzer::with_profile(
            AnalyzerProfile::empty_lexicon().with_accent_fold(AccentFold::Preserve),
        )
        .unwrap(),
    )
    .unwrap();
    let other = std::sync::Arc::new(TextIndex::from_dataset(&dataset, &other_config).unwrap());
    let expected = config.analyzer().fingerprint();
    let actual = other_config.analyzer().fingerprint();
    assert_ne!(expected, actual);
    let error = dictionary
        .scan(other, None, &workspace)
        .expect_err("even an empty dictionary retains its analyzer law");
    let cause = Capture::read(&error).child("cause").child("cause");
    assert_eq!(cause.code, "text-call-dictionary-analyzer");
    assert!(matches!(cause.field("expected"), ErrorValue::Bytes(value) if value == expected));
    assert!(matches!(cause.field("actual"), ErrorValue::Bytes(value) if value == actual));
}

fn native_query_diagnostics_keep_the_original_relation_cause_on_both_public_routes() {
    use purrdf_core::{
        RdfDatasetBuilder, RdfDiagnostic, SparqlRequest, TermValue, datatype::XSD_INTEGER,
    };
    use purrdf_sparql_eval::{
        ExtensionEnv, NativeSparqlEngine, PropertyFunctionRegistry, QueryOptions,
    };
    use purrdf_text::{GraphSelector, TextIndex, TextIndexConfig, TextSearchRelation};
    let dataset = RdfDatasetBuilder::new().freeze().unwrap();
    let config = TextIndexConfig::new(
        vec![TermValue::iri("https://example.org/p")],
        GraphSelector::Default,
        Analyzer::empty_lexicon(),
    )
    .unwrap();
    let index = std::sync::Arc::new(TextIndex::from_dataset(&dataset, &config).unwrap());
    let mut relations = PropertyFunctionRegistry::new();
    relations.register(
        "https://example.org/search".to_owned(),
        std::sync::Arc::new(TextSearchRelation::new(index)),
    );
    let env = ExtensionEnv::over_relations(relations).unwrap();
    let options = QueryOptions::new().with_env(&env);
    let query =
        "SELECT ?doc WHERE { ?doc <https://example.org/search> (42 ?score ?rank ?lang ?matched) }";
    let request = || SparqlRequest {
        query,
        base_iri: None,
        substitutions: &[],
    };
    let engine = NativeSparqlEngine::new();
    let inspect = |diagnostic: &RdfDiagnostic| {
        assert!(diagnostic.native_cause().is_some(), "{diagnostic}");
        let cause = Capture::read(diagnostic)
            .child("attachments")
            .child("native_cause")
            .child("cause")
            .child("cause");
        assert_eq!(cause.code, "text-call-needle-datatype");
        assert!(matches!(cause.field("position"), ErrorValue::Unsigned(1)));
        assert!(matches!(cause.field("datatype"), ErrorValue::Iri(value) if value == XSD_INTEGER));
        assert!(
            std::error::Error::source(diagnostic)
                .unwrap()
                .downcast_ref::<EvalError>()
                .is_some()
        );
    };
    let diagnostic = engine
        .query_with_options_view(&dataset, request(), options)
        .unwrap_err();
    inspect(&diagnostic);
    let copy = diagnostic.clone();
    assert_eq!(copy, diagnostic);
    assert!(core::ptr::eq(
        diagnostic.native_cause().unwrap(),
        copy.native_cause().unwrap()
    ));
    drop(diagnostic);
    inspect(&copy);
    let failure = engine
        .query_fallible_view(&dataset, request(), options)
        .unwrap_err();
    inspect(
        failure
            .diagnostic()
            .expect("the source remains operational"),
    );
}

purrdf_testkit::harness_main!(
    delta_failures_keep_original_identities_counts_and_nested_causes,
    phonetic_native_scalar_survives_evaluator_conversion,
    substring_native_refusal_preserves_counters_limits_and_generation,
    html_native_diagnostic_list_preserves_standard_codes_and_original_ranges,
    wide_substring_fields_are_not_narrowed_by_native_cause_transport,
    fixed_arithmetic_failures_keep_original_raw_operands_and_domain_values,
    exact_integer_records_preserve_all_binary_limbs_without_rendering,
    configuration_bounds_keep_the_requested_value_and_native_limit,
    configuration_ranking_keeps_signed_values_and_field_indices,
    configuration_predicates_keep_terms_and_original_iri_causes,
    data_corpus_refusal_preserves_full_width_totals_and_population,
    data_query_refusals_keep_original_order_and_exact_frequency_totals,
    data_scoring_refusals_keep_original_ordinal_and_field_facts,
    source_read_failure_retains_the_actual_backend_error_through_conversion_and_clone,
    ledger_source_errors_preserve_nested_node_keys_and_native_reason_refusals,
    baseline_configuration_keeps_artifact_counts_hashes_and_missing_flags,
    dictionary_configuration_keeps_conflicting_costs_and_emoji_byte_range,
    dictionary_artifact_refusals_keep_original_identity_header_and_utf8_values,
    binding_configuration_failure_retains_both_semantic_configurations,
    search_input_refusals_keep_actual_cardinality_and_observed_scalar_limit,
    relation_failures_retain_native_terms_rank_flags_arities_and_query_owners,
    phrase_and_dictionary_refusals_keep_registration_input_and_analyzer_identities,
    native_query_diagnostics_keep_the_original_relation_cause_on_both_public_routes,
);
