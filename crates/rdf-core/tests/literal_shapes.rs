// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native literal shape admission and compatible implied datatype expansion.

use purrdf_alloc_probe::{CountingAllocator, CurrentThreadWindow};
use purrdf_core::{LiteralComponentError, RdfDatasetBuilder, RdfLiteral, RdfTextDirection};
use purrdf_testkit::error_capture::Capture;

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

#[test]
fn invalid_literal_shapes_cannot_cross_native_validation() {
    for literal in [
        RdfLiteral {
            direction: Some(RdfTextDirection::Ltr),
            ..RdfLiteral::simple("x")
        },
        RdfLiteral::typed("x", RdfLiteral::language_datatype_iri(None)),
        RdfLiteral::typed(
            "x",
            RdfLiteral::language_datatype_iri(Some(RdfTextDirection::Rtl)),
        ),
    ] {
        let mut builder = RdfDatasetBuilder::new();
        builder.intern_literal(literal).unwrap();
        assert_eq!(
            builder
                .freeze()
                .expect_err("invalid shape must not publish")
                .code,
            "rdf-ir-literal-shape"
        );
    }
}

#[test]
fn component_diagnostic_preserves_the_original_account_and_storage_refusal() {
    use purrdf_lex::allocation::{Admission, Memory, StorageError};
    #[derive(Default)]
    struct Account {
        limit: usize,
        live: usize,
        refused: Option<usize>,
    }
    impl Admission for Account {
        fn resize(&mut self, live: usize) -> Result<(), StorageError> {
            if live > self.limit {
                self.refused = Some(live);
                return Err(StorageError::AdmissionFailed);
            }
            self.live = live;
            Ok(())
        }
    }
    let cause = RdfLiteral::validate_components(
        RdfLiteral::language_datatype_iri(None),
        Some("en us"),
        None,
    )
    .expect_err("native grammar refusal");
    let mut account = Account {
        limit: usize::MAX,
        ..Account::default()
    };
    let mut memory = Memory::new(&mut account);
    let diagnostic = cause.into_diagnostic_with_memory(&mut memory).unwrap();
    let retained = diagnostic.retained_bytes().unwrap();
    assert_eq!(memory.admitted_bytes(), retained);
    drop(diagnostic);
    memory.release_bytes(retained).unwrap();
    assert_eq!(account.live, 0);
    for limit in [0, retained - 1] {
        let mut account = Account {
            limit,
            ..Account::default()
        };
        let mut memory = Memory::new(&mut account);
        let window = CurrentThreadWindow::open();
        let failure = cause
            .into_diagnostic_with_memory(&mut memory)
            .expect_err("original account refuses diagnostic storage");
        let measured = window.close();
        assert_eq!(failure, StorageError::AdmissionFailed);
        assert_eq!(memory.admitted_bytes(), 0);
        assert_eq!(measured.retained_bytes, 0);
        assert_eq!(account.live, 0);
        assert!(account.refused.unwrap() > limit);
    }
}

/// A language tag the concrete syntaxes could not write must not cross the
/// intern path either — this is the hole that let `@en us` reach a serializer
/// that emits `@` + the tag verbatim. The refusal carries the grammar's own
/// `langtag-*` code, not a generic shape code, because `purrdf_iri::langtag` is
/// the one owner of the judgement.
#[test]
fn malformed_language_tags_cannot_cross_the_intern_path() {
    for (tag, code) in [
        ("en us", "langtag-terminal-primary-not-alpha"),
        ("1", "langtag-terminal-primary-not-alpha"),
        ("9-9", "langtag-terminal-primary-not-alpha"),
        ("123-456", "langtag-terminal-primary-not-alpha"),
        ("en-", "langtag-subtag-length-zero"),
        ("-", "langtag-subtag-length-zero"),
        ("!!!", "langtag-terminal-primary-not-alpha"),
        ("", "langtag-subtag-length-zero"),
        // The second `LANGTAG` rule: a well-formed primary subtag followed by
        // one that is not alphanumeric. (`en us` above trips the FIRST rule —
        // with no hyphen the whole string is the primary subtag.)
        ("en-u s", "langtag-terminal-subtag-not-alphanum"),
    ] {
        let mut builder = RdfDatasetBuilder::new();
        builder
            .intern_literal(RdfLiteral::language_tagged("x", tag))
            .unwrap();
        let diagnostic = builder
            .freeze()
            .expect_err("a malformed language tag must not publish");
        assert_eq!(diagnostic.code, code, "{tag:?}");
        assert!(!diagnostic.message.is_empty(), "{tag:?} gave no reason");
        let expected = RdfLiteral::validate_components(
            RdfLiteral::language_datatype_iri(None),
            Some(tag),
            None,
        )
        .expect_err("original component refusal");
        let copy = diagnostic.clone();
        assert!(core::ptr::eq(
            std::error::Error::source(&diagnostic).unwrap(),
            std::error::Error::source(&copy).unwrap()
        ));
        drop(diagnostic);
        let original = copy.native_cause().unwrap();
        assert_eq!(
            (original as &dyn std::error::Error).downcast_ref::<LiteralComponentError>(),
            Some(&expected)
        );
        let events = Vec::with_capacity(8);
        let window = CurrentThreadWindow::open();
        let fields = Capture::read_with_buffer(original, events);
        let allocation = window.close();
        assert_eq!(allocation.allocations, 0);
        assert_eq!(fields.child("cause").code, code);
    }
}

/// The over-refusal twin: every tag a codec could legitimately hand the kernel
/// still interns and still freezes, in the case it was authored in.
#[test]
fn well_formed_language_tags_still_cross_the_intern_path() {
    let mut builder = RdfDatasetBuilder::new();
    for tag in [
        "en",
        "en-US",
        "zh-Hans-CN",
        "de-CH-x-phonebk",
        "i-enochian",
        "x-purrdf-afrikaans",
        "x-gmeow-english",
        "en-fr-jura",
        "fr-be-fbcl",
    ] {
        builder
            .intern_literal(RdfLiteral::language_tagged("x", tag))
            .unwrap();
    }
    builder
        .freeze()
        .expect("well-formed language tags must publish");
}

#[test]
fn native_implied_datatype_policy_remains_compatible() {
    let mut builder = RdfDatasetBuilder::new();
    for direction in [
        None,
        Some(RdfTextDirection::Ltr),
        Some(RdfTextDirection::Rtl),
    ] {
        let id = builder
            .intern_literal(RdfLiteral {
                lexical_form: "x".to_owned(),
                datatype: Some("https://example.org/ignored-explicit-type".to_owned()),
                language: Some("EN".to_owned()),
                direction,
            })
            .unwrap();
        let purrdf_core::TermRef::Literal {
            datatype,
            language,
            direction: stored,
            ..
        } = builder.resolve(id)
        else {
            panic!("literal")
        };
        assert_eq!(language, Some("en"));
        assert_eq!(stored, direction);
        assert_eq!(
            builder.resolve(datatype),
            purrdf_core::TermRef::Iri(RdfLiteral::language_datatype_iri(direction))
        );
    }
    builder
        .freeze()
        .expect("implied datatype expansion is valid");
}
