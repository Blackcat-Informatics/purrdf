// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Composite parser and comparison failures through the generic typed consumer.

use core::error::Error as _;
use purrdf_hash::error::ErrorValue;
use purrdf_testkit::error_capture::Capture;

#[global_allocator]
static GLOBAL: purrdf_alloc_probe::CountingAllocator = purrdf_alloc_probe::CountingAllocator;

#[test]
fn parser_refusals_keep_native_causes_after_lexical_input_destruction() {
    use purrdf_cdt::{CdtDatatype, CdtError, memory::ReadError};
    use purrdf_iri::langtag::LanguageTagError;
    use purrdf_lex::terminals::UcharError;

    for (input, expected) in [
        (r#"["\uD800"]"#, UcharError::NotAScalar),
        (r#"["\uZZZZ"]"#, UcharError::BadHex),
    ] {
        let error = {
            let lexical = input.to_owned();
            purrdf_cdt::parse_list(&lexical).unwrap_err()
        };
        assert!(matches!(
            &error,
            CdtError::BadEscape { offset: 2, cause: Some(cause), .. } if *cause == expected
        ));
        let record = Capture::read(&error);
        assert_eq!(record.code, "cdt-bad-escape");
        assert_eq!(record.events.len(), 3);
        assert!(matches!(record.field("offset"), ErrorValue::ByteCount(2)));
        assert!(matches!(record.field("reason"), ErrorValue::Text(reason) if !reason.is_empty()));
        assert_eq!(record.child("cause").code, Capture::read(&expected).code);
        assert_eq!(
            error.source().unwrap().downcast_ref::<UcharError>(),
            Some(&expected)
        );
    }
    let error = {
        let lexical = r#"["x"@cantbethislong]"#.to_owned();
        purrdf_cdt::parse_list(&lexical).unwrap_err()
    };
    assert!(matches!(
        &error,
        CdtError::BadLanguageTag {
            offset: 4,
            cause: Some(LanguageTagError::SubtagLengthOverEight),
            ..
        }
    ));
    assert_eq!(
        Capture::read(&error).child("cause").code,
        "langtag-subtag-length-over-eight"
    );
    assert_eq!(
        error.to_string(),
        "at byte 4: bad language tag: language-tag subtag longer than 8 characters"
    );
    assert_eq!(
        error.source().unwrap().downcast_ref::<LanguageTagError>(),
        Some(&LanguageTagError::SubtagLengthOverEight)
    );
    for input in [r#"["\q"]"#, r#"["x"@en--up]"#] {
        let error = purrdf_cdt::parse_list(input).unwrap_err();
        assert!(matches!(
            Capture::read(&error).field("cause"),
            ErrorValue::Absent
        ));
        assert!(error.source().is_none());
    }
    let wrapped = purrdf_cdt::parse::try_parse_cdt(
        r#"["\uD800"]"#,
        CdtDatatype::List,
        &mut purrdf_lex::allocation::Resident,
    )
    .unwrap_err();
    let record = Capture::read(&wrapped);
    assert_eq!(record.code, "cdt-read-lexical");
    assert_eq!(record.child("cause").code, "cdt-bad-escape");
    assert_eq!(
        record.child("cause").child("cause").code,
        "uchar-not-a-scalar"
    );
    let lexical_source = wrapped
        .source()
        .unwrap()
        .downcast_ref::<CdtError>()
        .unwrap();
    assert_eq!(
        lexical_source
            .source()
            .unwrap()
            .downcast_ref::<UcharError>(),
        Some(&UcharError::NotAScalar)
    );

    struct RefuseMemory;
    impl purrdf_cdt::memory::Admission for RefuseMemory {
        fn resize(&mut self, live_bytes: usize) -> Result<(), purrdf_cdt::memory::StorageError> {
            if live_bytes == 0 {
                Ok(())
            } else {
                Err(purrdf_cdt::memory::StorageError::AdmissionFailed)
            }
        }
    }
    impl purrdf_cdt::memory::Storage for RefuseMemory {}
    let storage =
        purrdf_cdt::parse::try_parse_cdt("[1]", CdtDatatype::List, &mut RefuseMemory).unwrap_err();
    assert!(matches!(
        storage,
        ReadError::Storage(purrdf_cdt::memory::StorageError::AdmissionFailed)
    ));
    let record = Capture::read(&storage);
    assert_eq!(record.code, "cdt-read-storage");
    assert_eq!(record.child("cause").code, "storage-admission-failed");
    assert_eq!(
        storage
            .source()
            .unwrap()
            .downcast_ref::<purrdf_cdt::memory::StorageError>(),
        Some(&purrdf_cdt::memory::StorageError::AdmissionFailed)
    );

    for input in [r#"["\u0041"]"#, r#"["\U0001F431"]"#, r#"["x"@en--ltr]"#] {
        let value = purrdf_cdt::parse_list(input).expect("accepted neighbouring terminal");
        assert_eq!(
            purrdf_cdt::parse_list(&value.canonical_lexical()).unwrap(),
            value
        );
    }
}

#[test]
fn duplicate_key_retains_original_offset_and_canonical_lexical_key() {
    let error = purrdf_cdt::parse_map("{1: true, 1: false}")
        .expect_err("repeated identical map keys must be refused");
    let record = Capture::read(&error);
    assert_eq!(record.code, "cdt-duplicate-map-key");
    assert_eq!(record.events.len(), 2);
    assert!(matches!(record.field("offset"), ErrorValue::ByteCount(10)));
    let expected = format!("\"1\"^^<{}>", purrdf_xsd::datatype::XSD_INTEGER);
    assert!(matches!(record.field("key"), ErrorValue::Text(key) if key == expected));
}

#[test]
fn comparison_refusal_retains_classification_and_original_reason() {
    let left = purrdf_cdt::parse_list("[\"x\"^^<http://example.org/unmodelled>]")
        .expect("a well-formed list");
    let right = purrdf_cdt::parse_list("[\"y\"^^<http://example.org/unmodelled>]")
        .expect("a well-formed list");
    let error = purrdf_cdt::value_equal(&left, &right)
        .expect_err("the custom value space has no equality implementation");
    let record = Capture::read(&error);
    assert_eq!(record.code, "cdt-type");
    assert_eq!(record.events.len(), 2);
    assert_eq!(record.child("kind").code, "cdt-type-unmodelled");
    assert!(matches!(record.field("reason"), ErrorValue::Text(reason) if reason == error.reason));
}

#[test]
fn public_comparisons_preserve_ill_typed_and_undefined_classifications() {
    let ill =
        purrdf_cdt::parse_list(r#"["bad"^^<http://www.w3.org/2001/XMLSchema#integer>]"#).unwrap();
    let good = purrdf_cdt::parse_list("[1]").unwrap();
    let map = purrdf_cdt::parse_map("{}").unwrap();
    for (error, code) in [
        (
            purrdf_cdt::value_equal(&ill, &good).unwrap_err(),
            "cdt-type-ill-typed",
        ),
        (
            purrdf_cdt::value_less_than(&good, &map).unwrap_err(),
            "cdt-type-undefined",
        ),
    ] {
        let record = Capture::read(&error);
        assert_eq!(record.events.len(), 2);
        assert_eq!(record.child("kind").code, code);
        assert!(
            matches!(record.field("reason"), ErrorValue::Text(reason) if reason == error.reason)
        );
        assert!(error.source().is_none());
    }
    assert_eq!(purrdf_cdt::value_equal(&good, &good), Ok(true));
}

#[test]
fn inspection_borrows_retained_diagnostic_payload_without_allocating() {
    for input in ["{1: true, 1: false}", "[<relative>]"] {
        let error = {
            let lexical = input.to_owned();
            if lexical.starts_with('{') {
                purrdf_cdt::parse_map(&lexical)
            } else {
                purrdf_cdt::parse_list(&lexical)
            }
            .unwrap_err()
        };
        let (name, payload) = match &error {
            purrdf_cdt::CdtError::DuplicateMapKey { key, .. } => ("key", key),
            purrdf_cdt::CdtError::NotAbsoluteIri { iri, .. } => ("iri", iri),
            _ => panic!("expected original owned diagnostic"),
        };
        assert_eq!(error.owned_bytes(), payload.capacity());
        let buffer = Vec::with_capacity(8);
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        let record = Capture::read_with_buffer(std::hint::black_box(&error), buffer);
        let measured = window.close();
        assert_eq!(measured, purrdf_alloc_probe::Measurement::default());
        let (ErrorValue::Text(borrowed) | ErrorValue::Iri(borrowed)) = record.field(name) else {
            panic!("diagnostic text must remain typed and borrowed");
        };
        assert_eq!(borrowed.as_ptr(), payload.as_ptr());
        assert_eq!(borrowed, payload);
    }
}

#[test]
fn public_grammar_refusals_report_original_positions_and_reasons() {
    for (input, code, offset, field) in [
        ("[1 2]", "cdt-unexpected", 3, "expected"),
        ("[", "cdt-unexpected-end", 1, "expected"),
        ("[1e]", "cdt-bad-numeric-literal", 1, "reason"),
        ("[_:]", "cdt-bad-blank-node-label", 1, "reason"),
        ("[] x", "cdt-trailing-text", 3, ""),
    ] {
        let error = {
            let lexical = input.to_owned();
            purrdf_cdt::parse_list(&lexical).unwrap_err()
        };
        let record = Capture::read(&error);
        assert_eq!(record.code, code, "{input}");
        assert!(matches!(record.field("offset"), ErrorValue::ByteCount(value) if value == offset));
        if !field.is_empty() {
            assert_eq!(record.events.len(), 2);
            assert!(matches!(record.field(field), ErrorValue::Text(reason) if !reason.is_empty()));
        } else {
            assert_eq!(record.events.len(), 1);
        }
        assert!(error.source().is_none());
    }
}

#[test]
fn original_nested_causes_are_inspected_without_allocation() {
    let error = purrdf_cdt::parse::try_parse_cdt(
        r#"["\uD800"]"#,
        purrdf_cdt::CdtDatatype::List,
        &mut purrdf_lex::allocation::Resident,
    )
    .unwrap_err();
    let buffers = [
        Vec::with_capacity(8),
        Vec::with_capacity(8),
        Vec::with_capacity(8),
    ];
    let window = purrdf_alloc_probe::CurrentThreadWindow::open();
    let [outer_buffer, lexical_buffer, terminal_buffer] = buffers;
    let outer = Capture::read_with_buffer(std::hint::black_box(&error), outer_buffer);
    let ErrorValue::Cause(lexical) = outer.field("cause") else {
        panic!("original lexical cause");
    };
    let lexical = Capture::read_with_buffer(lexical, lexical_buffer);
    let ErrorValue::Cause(terminal) = lexical.field("cause") else {
        panic!("original terminal cause");
    };
    let terminal = Capture::read_with_buffer(terminal, terminal_buffer);
    let source = error.source().unwrap().source().unwrap();
    let measured = window.close();
    assert_eq!(measured, purrdf_alloc_probe::Measurement::default());
    assert_eq!(terminal.code, "uchar-not-a-scalar");
    assert_eq!(
        source.downcast_ref::<purrdf_lex::terminals::UcharError>(),
        Some(&purrdf_lex::terminals::UcharError::NotAScalar)
    );
}
