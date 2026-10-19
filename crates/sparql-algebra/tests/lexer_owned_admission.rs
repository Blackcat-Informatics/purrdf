// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original scanner/parser bytes, before-growth refusal and retained typed failures.

use purrdf_alloc_probe::{CountingAllocator, CurrentThreadWindow};
use purrdf_hash::error::ErrorValue;
use purrdf_lex::allocation::{Admission, Memory, StorageError};
use purrdf_sparql_algebra::SparqlParser;
use purrdf_sparql_algebra::algebra::{
    AggregateExpression, AggregateFunction, Expression, OrderExpression,
};
use purrdf_sparql_algebra::ast::{Literal, NamedNode};
use purrdf_sparql_algebra::error::ParseError;
use purrdf_sparql_algebra::lexer::{LexerOptions, tokenize, tokenize_with_memory};
use purrdf_testkit::error_capture::Capture;

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

struct Account {
    window: CurrentThreadWindow,
    live: usize,
    peak: usize,
    ceiling: usize,
}

impl Admission for Account {
    fn resize(&mut self, bytes: usize) -> Result<(), StorageError> {
        let measured = self.window.sample();
        assert!(measured.retained_bytes <= i64::try_from(self.live).unwrap());
        if bytes > self.ceiling {
            return Err(StorageError::AdmissionFailed);
        }
        self.live = bytes;
        self.peak = self.peak.max(bytes);
        Ok(())
    }
}

fn escaped_tokens_keep_exact_original_storage_and_scanner_values() {
    let input = format!(
        "SELECT ?x {{ {} }}",
        "<http://example.org/a\\u0041> p:a\\~b \"c\\nlong\" . ".repeat(1024)
    );
    let expected = tokenize(&input).unwrap();
    let mut account = Account {
        window: CurrentThreadWindow::open(),
        live: 0,
        peak: 0,
        ceiling: usize::MAX,
    };
    let mut memory = Memory::new(&mut account);
    let tokens = tokenize_with_memory(&input, LexerOptions::default(), &mut memory).unwrap();
    assert_eq!(tokens, expected);
    let live = memory.admitted_bytes();
    drop(tokens);
    memory.release_bytes(live).unwrap();
    let measured = account.window.close();
    assert!(measured.peak_working_bytes <= i64::try_from(account.peak).unwrap());
    assert_eq!(measured.retained_bytes, 0);
    assert_eq!(account.live, 0);
}

fn first_token_capacity_refuses_before_allocating() {
    let mut account = Account {
        window: CurrentThreadWindow::open(),
        live: 0,
        peak: 0,
        ceiling: 0,
    };
    let mut memory = Memory::new(&mut account);
    let error = tokenize_with_memory("SELECT", LexerOptions::default(), &mut memory).unwrap_err();
    assert_eq!(error, ParseError::Storage(StorageError::AdmissionFailed));
    drop(error);
    let measured = account.window.close();
    assert_eq!(measured.allocations, 0);
    assert_eq!(measured.retained_bytes, 0);
}

fn lexical_failure_releases_token_and_decoding_scratch_before_publication() {
    let input = format!("{} \"bad\\z\"", "?x ".repeat(1024));
    let expected = tokenize(&input).unwrap_err();
    let mut account = Account {
        window: CurrentThreadWindow::open(),
        live: 0,
        peak: 0,
        ceiling: usize::MAX,
    };
    let mut memory = Memory::new(&mut account);
    let error = tokenize_with_memory(&input, LexerOptions::default(), &mut memory).unwrap_err();
    assert_eq!(error, expected);
    let ParseError::Lex { .. } = &error else {
        panic!("lexical error")
    };
    assert_eq!(
        memory.admitted_bytes(),
        0,
        "the native reason needs no text buffer"
    );
    let live = memory.admitted_bytes();
    drop(error);
    memory.release_bytes(live).unwrap();
    let measured = account.window.close();
    assert!(measured.peak_working_bytes <= i64::try_from(account.peak).unwrap());
    assert_eq!(measured.retained_bytes, 0);
    assert_eq!(account.live, 0);
}

fn original_iri_cause_is_borrowed_without_rendering_or_reparsing() {
    use std::error::Error;

    let mut error = SparqlParser::new()
        .parse_query("SELECT * WHERE { <http://example.org/%zz> ?p ?o }")
        .unwrap_err();
    let mut buffer = Vec::with_capacity(8);
    let window = CurrentThreadWindow::open();
    let parent = Capture::read_with_buffer(&error, buffer);
    assert_eq!(parent.code, "sparql-parse-iri");
    let ErrorValue::Cause(reason) = parent.field("reason") else {
        panic!("typed IRI reason")
    };
    buffer = parent.events;
    let reason = Capture::read_with_buffer(reason, buffer);
    assert_eq!(reason.code, "sparql-iri-validation");
    assert!(matches!(
        reason.field("include_code"),
        ErrorValue::Boolean(true)
    ));
    let ErrorValue::Cause(cause) = reason.field("cause") else {
        panic!("original native cause")
    };
    buffer = reason.events;
    let cause = Capture::read_with_buffer(cause, buffer);
    assert_eq!(cause.code, "iri-bad-percent-encoding");
    assert!(matches!(cause.field("offset"), ErrorValue::ByteCount(19)));
    let measured = window.close();
    assert_eq!(measured.allocations, 0);
    drop(cause);

    // Changing presentation context must not replace or guess a native cause.
    let ParseError::Iri { lexical, reason } = &mut error else {
        unreachable!()
    };
    *lexical = "http://example.org/valid".into();
    let original = reason.validation_error().unwrap();
    assert!(matches!(
        original,
        purrdf_iri::IriError::BadPercentEncoding(19)
    ));
    let detail = error.presentation();
    assert_eq!(
        detail.detail().unwrap().message_id(),
        "iri-bad-percent-encoding"
    );
    assert!(
        error
            .source()
            .unwrap()
            .source()
            .unwrap()
            .is::<purrdf_iri::IriError>()
    );
    let clone = error.clone();
    drop(error);
    assert_eq!(
        Capture::read(&clone).child("reason").child("cause").code,
        "iri-bad-percent-encoding"
    );
}

fn native_lexical_reasons_keep_original_fields_and_messages() {
    let cases = [
        (
            "%",
            "sparql-lex-unexpected-character",
            "unexpected character '%'",
        ),
        ("&x", "sparql-lex-expected-and", "expected '&&'"),
        (
            "\"unterminated",
            "sparql-lex-unterminated-string",
            "unterminated string literal",
        ),
        (
            "'''unterminated",
            "sparql-lex-unterminated-string",
            "unterminated string literal",
        ),
        (
            "\"tail\\",
            "sparql-lex-unterminated-escape",
            "unterminated escape",
        ),
        (
            "\"\\uD800\"",
            "sparql-lex-unicode-escape",
            "bad unicode escape",
        ),
        (
            "\"\\u12zz\"",
            "sparql-lex-unicode-escape",
            "bad unicode escape",
        ),
        ("\"\\z\"", "sparql-lex-bad-escape", "bad escape \\z"),
        (
            "\"line\nbreak\"",
            "sparql-lex-raw-newline",
            "raw newline in short string literal",
        ),
        (
            "$",
            "sparql-lex-empty-variable",
            "empty variable name after sigil",
        ),
        (
            "_:-",
            "sparql-lex-blank-label-start",
            "a blank node label must begin with PN_CHARS_U or [0-9]: BLANK_NODE_LABEL ::= '_:' ( PN_CHARS_U | [0-9] ) ((PN_CHARS | '.')* PN_CHARS)?",
        ),
        ("@", "sparql-lex-empty-language-tag", "empty language tag"),
    ];
    for (source, code, english) in cases {
        let error = tokenize(source).unwrap_err();
        let mut buffer = Vec::with_capacity(8);
        assert_eq!(
            error.to_string(),
            format!("SPARQL lex error at byte 0: {english}")
        );
        assert_eq!(error.owned_text_bytes(), Some(0));
        let window = CurrentThreadWindow::open();
        let parent = Capture::read_with_buffer(&error, buffer);
        assert!(matches!(parent.field("at"), ErrorValue::ByteCount(0)));
        let ErrorValue::Cause(reason) = parent.field("reason") else {
            panic!("typed native reason")
        };
        buffer = parent.events;
        let reason = Capture::read_with_buffer(reason, buffer);
        assert_eq!(reason.code, code);
        assert_eq!(window.close().allocations, 0);
        match code {
            "sparql-lex-unexpected-character" => assert!(matches!(
                reason.field("character"),
                ErrorValue::Character('%')
            )),
            "sparql-lex-expected-and" => {
                assert!(matches!(reason.field("next"), ErrorValue::Character('x')));
            }
            "sparql-lex-unterminated-string" => {
                assert!(
                    matches!(reason.field("long"), ErrorValue::Boolean(long) if long == source.starts_with("'''"))
                );
                assert!(
                    matches!(reason.field("quote"), ErrorValue::Character(quote) if quote == source.chars().next().unwrap())
                );
            }
            "sparql-lex-unterminated-escape" => assert!(matches!(
                reason.field("escape_at"),
                ErrorValue::ByteCount(5)
            )),
            "sparql-lex-unicode-escape" => {
                assert!(matches!(
                    reason.field("escape_at"),
                    ErrorValue::ByteCount(1)
                ));
                assert_eq!(
                    reason.child("cause").code,
                    if source.contains("D800") {
                        "uchar-not-a-scalar"
                    } else {
                        "uchar-bad-hex"
                    }
                );
            }
            "sparql-lex-bad-escape" => {
                assert!(matches!(
                    reason.field("character"),
                    ErrorValue::Character('z')
                ));
                assert!(matches!(
                    reason.field("escape_at"),
                    ErrorValue::ByteCount(1)
                ));
            }
            "sparql-lex-raw-newline" => assert!(matches!(
                reason.field("character"),
                ErrorValue::Character('\n')
            )),
            "sparql-lex-empty-variable" => {
                assert!(matches!(reason.field("sigil"), ErrorValue::Character('$')));
                assert!(matches!(reason.field("next"), ErrorValue::Absent));
            }
            "sparql-lex-blank-label-start" => {
                assert!(matches!(reason.field("next"), ErrorValue::Character('-')));
            }
            "sparql-lex-empty-language-tag" => {
                assert!(matches!(reason.field("next"), ErrorValue::Absent));
            }
            _ => unreachable!("the cases name the native lexical conditions"),
        }
    }
}

fn lexical_failure_preserves_the_callers_original_owner() {
    let mut account = Account {
        window: CurrentThreadWindow::open(),
        live: 0,
        peak: 0,
        ceiling: usize::MAX,
    };
    let mut memory = Memory::new(&mut account);
    let original = memory.string("caller-owned buffer").unwrap();
    let live = memory.admitted_bytes();
    let error =
        tokenize_with_memory("?x \"bad\\z\"", LexerOptions::default(), &mut memory).unwrap_err();
    assert!(matches!(error, ParseError::Lex { .. }));
    assert_eq!(memory.admitted_bytes(), live);
    assert_eq!(original, "caller-owned buffer");
    drop(error);
    memory.release_string(original).unwrap();
    assert_eq!(memory.admitted_bytes(), 0);
    let measured = account.window.close();
    assert_eq!(measured.retained_bytes, 0);
    assert!(measured.peak_working_bytes <= i64::try_from(account.peak).unwrap());
}

fn native_refusals_need_no_message_allocation_and_caller_text_stays_message_data() {
    let mut account = Account {
        window: CurrentThreadWindow::open(),
        live: 0,
        peak: 0,
        ceiling: 0,
    };
    let mut memory = Memory::new(&mut account);
    let error = tokenize_with_memory("%", LexerOptions::default(), &mut memory).unwrap_err();
    assert!(matches!(error, ParseError::Lex { .. }));
    assert_eq!(account.window.close().allocations, 0);
    let caller = ParseError::lex("opaque caller message", usize::MAX);
    let capture = Capture::read(&caller);
    assert!(matches!(capture.field("at"), ErrorValue::ByteCount(at) if at == usize::MAX as u128));
    assert!(matches!(
        capture.child("reason").field("message"),
        ErrorValue::Message("opaque caller message")
    ));
}

fn cdt_refusals_retain_the_native_signature_and_full_width_counts() {
    use purrdf_cdt::{CdtArity, CdtFn};
    for (function, arguments, code) in [
        (CdtFn::Head, "1, 2", "cdt-arity-fixed"),
        (CdtFn::Subseq, "1", "cdt-arity-range"),
        (CdtFn::Merge, "1", "cdt-arity-at-least"),
        (CdtFn::MapConstructor, "1", "cdt-arity-pairs"),
    ] {
        let query = format!(
            "SELECT (<{}>({arguments}) AS ?value) WHERE {{}}",
            function.iri()
        );
        let error = SparqlParser::new().parse_query(&query).unwrap_err();
        let capture = Capture::read(&error);
        assert_eq!(capture.code, "sparql-parse-cdt-arity");
        let signature = capture.child("expected");
        assert_eq!(signature.code, code);
        match function {
            CdtFn::Head => assert!(matches!(signature.field("count"), ErrorValue::Unsigned(1))),
            CdtFn::Subseq => {
                assert!(matches!(signature.field("min"), ErrorValue::Unsigned(2)));
                assert!(matches!(signature.field("max"), ErrorValue::Unsigned(3)));
            }
            CdtFn::Merge => assert!(matches!(signature.field("min"), ErrorValue::Unsigned(2))),
            CdtFn::MapConstructor => assert!(signature.events.is_empty()),
            _ => unreachable!("the four native signature shapes above"),
        }
    }
    for (arity, field) in [
        (CdtArity::Fixed(usize::MAX), "count"),
        (
            CdtArity::Range {
                min: 0,
                max: usize::MAX,
            },
            "max",
        ),
        (CdtArity::AtLeast(usize::MAX), "min"),
    ] {
        assert!(
            matches!(Capture::read(&arity).field(field), ErrorValue::Unsigned(value) if value == usize::MAX as u128)
        );
    }
}

fn aggregate_refusals_move_the_original_key_without_allocating() {
    let mut key = String::with_capacity(4096);
    key.push_str("first rejected key");
    let original = key.as_ptr();
    let capacity = key.capacity();
    let args = vec![Expression::Literal(Literal::new_simple("value"))];
    let scalarvals = vec![
        (key, Literal::new_simple("first")),
        (
            String::from("second rejected key"),
            Literal::new_simple("second"),
        ),
    ];
    let events = Vec::with_capacity(4);
    let window = CurrentThreadWindow::open();
    let cause =
        AggregateExpression::new(AggregateFunction::Sum, args, scalarvals, Vec::new(), false)
            .unwrap_err();
    let capture = Capture::read_with_buffer(&cause, events);
    let inspected = window.close();
    assert_eq!(inspected.allocations, 0);
    let fields = capture.child("cause");
    assert!(matches!(fields.field("key"), ErrorValue::Text(text)
        if text == "first rejected key" && text.as_ptr() == original));
    assert_eq!(cause.owned_text_bytes(), capacity);
    assert_eq!(fields.child("function").code, "sparql-aggregate-sum");

    let error = ParseError::Aggregate {
        cause,
        at: usize::MAX,
    };
    assert_eq!(error.owned_text_bytes(), Some(capacity));
    assert_eq!(error.byte_offset(), Some(usize::MAX));
    assert!(
        std::error::Error::source(&error)
            .unwrap()
            .source()
            .is_some()
    );
    assert_eq!(error.presentation().message_id(), "sparql-parse-syntax");
    assert_eq!(error.presentation().english(), error.to_string());
    let parsed = Capture::read(&error);
    assert!(matches!(parsed.field("at"), ErrorValue::ByteCount(at) if at == usize::MAX as u128));
    assert_eq!(parsed.child("cause").code, "sparql-aggregate-scalarval");
}

fn aggregate_errors_expose_all_functions_and_private_fields() {
    for (function, code) in [
        (AggregateFunction::Count, "sparql-aggregate-count"),
        (AggregateFunction::Sum, "sparql-aggregate-sum"),
        (AggregateFunction::Avg, "sparql-aggregate-avg"),
        (AggregateFunction::Min, "sparql-aggregate-min"),
        (AggregateFunction::Max, "sparql-aggregate-max"),
        (AggregateFunction::Sample, "sparql-aggregate-sample"),
        (
            AggregateFunction::GroupConcat,
            "sparql-aggregate-group-concat",
        ),
        (AggregateFunction::Fold, "sparql-aggregate-fold"),
        (
            AggregateFunction::Custom(NamedNode::new_unchecked("https://example.org/aggregate")),
            "sparql-aggregate-custom",
        ),
    ] {
        let args = if function == AggregateFunction::Count {
            vec![Expression::Literal(Literal::new_simple("a")); 2]
        } else {
            Vec::new()
        };
        let supplied = args.len();
        let error =
            AggregateExpression::new(function, args, Vec::new(), Vec::new(), false).unwrap_err();
        let fields = Capture::read(&error).child("cause");
        assert!(
            matches!(fields.field("supplied"), ErrorValue::Unsigned(n) if n == supplied as u128)
        );
        let function = fields.child("function");
        assert_eq!(function.code, code);
        if code == "sparql-aggregate-custom" {
            assert!(matches!(function.field("iri"), ErrorValue::Iri(iri)
                if iri == "https://example.org/aggregate"));
        }
        assert_eq!(error.owned_text_bytes(), 0);
    }
    let expression = Expression::Literal(Literal::new_simple("value"));
    let error = AggregateExpression::new(
        AggregateFunction::Sum,
        vec![expression.clone()],
        Vec::new(),
        vec![OrderExpression::Asc(expression)],
        false,
    )
    .unwrap_err();
    let capture = Capture::read(&error);
    assert_eq!(capture.code, "sparql-aggregate-order-by");
    assert_eq!(
        capture.child("cause").child("function").code,
        "sparql-aggregate-sum"
    );
}

purrdf_testkit::harness_main!(
    escaped_tokens_keep_exact_original_storage_and_scanner_values,
    first_token_capacity_refuses_before_allocating,
    lexical_failure_releases_token_and_decoding_scratch_before_publication,
    original_iri_cause_is_borrowed_without_rendering_or_reparsing,
    native_lexical_reasons_keep_original_fields_and_messages,
    lexical_failure_preserves_the_callers_original_owner,
    native_refusals_need_no_message_allocation_and_caller_text_stays_message_data,
    cdt_refusals_retain_the_native_signature_and_full_width_counts,
    aggregate_refusals_move_the_original_key_without_allocating,
    aggregate_errors_expose_all_functions_and_private_fields,
);
