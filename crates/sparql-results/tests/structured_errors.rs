// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original result-reader and writer failures through the portable public API.

use purrdf_alloc_probe::{CountingAllocator, CurrentThreadWindow};
use purrdf_core::{RdfDatasetBuilder, SparqlResult, TermValue};
use purrdf_hash::error::ErrorValue;
use purrdf_sparql_results::{
    Error, MalformedResult, ProvenanceNamespace, ResultProvenance, SparqlResultsFormat, from_json,
    from_xml, serialize, serialize_into, to_xml,
};
use purrdf_testkit::error_capture::{CallerRefusal, Capture};

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

fn one_literal(text: &str) -> SparqlResult {
    SparqlResult::Solutions {
        variables: vec!["x".into()],
        rows: vec![vec![Some(TermValue::simple_literal(text))]],
        aux: RdfDatasetBuilder::new().freeze().unwrap().into(),
    }
}

fn native_syntax_causes_survive_input_disposal_and_clone() {
    let input = String::from("{");
    let error = from_json(input.as_bytes()).unwrap_err();
    drop(input);
    let copy = error.clone();
    drop(error);
    let fields = Capture::read(&copy);
    assert_eq!(fields.code, "sparql-results-json");
    assert!(matches!(
        fields.child("cause").field("offset"),
        ErrorValue::ByteCount(1)
    ));
    assert!(
        std::error::Error::source(&copy)
            .unwrap()
            .downcast_ref::<purrdf_lex::json::Error>()
            .is_some()
    );

    let input = String::from("<sparql>");
    let error = from_xml(input.as_bytes()).unwrap_err();
    drop(input);
    let fields = Capture::read(&error);
    assert_eq!(fields.code, "sparql-results-xml-syntax");
    assert!(matches!(fields.field("line"), ErrorValue::Unsigned(1)));
    assert!(matches!(fields.field("column"), ErrorValue::Unsigned(9)));
    assert!(matches!(
        fields.child("cause").field("offset"),
        ErrorValue::ByteCount(8)
    ));
    assert!(
        std::error::Error::source(&error)
            .unwrap()
            .downcast_ref::<purrdf_lex::xml::XmlError>()
            .is_some()
    );
}

fn semantic_tokens_and_language_tags_keep_original_values() {
    let input = String::from(
        r#"{"head":{"vars":["x"]},"results":{"bindings":[{"x":{"type":"unexpected-λ","value":"original"}}]}}"#,
    );
    let error = from_json(input.as_bytes()).unwrap_err();
    drop(input);
    let reason = Capture::read(&error).child("cause").child("reason");
    assert_eq!(reason.child("kind").code, "results-token-binding-type");
    assert!(matches!(
        reason.field("token"),
        ErrorValue::Text("unexpected-λ")
    ));

    let input = String::from(
        r#"{"head":{"vars":["x"]},"results":{"bindings":[{"x":{"type":"literal","value":"original","xml:lang":"bad_tag"}}]}}"#,
    );
    let error = from_json(input.as_bytes()).unwrap_err();
    drop(input);
    let fields = Capture::read(&error);
    assert_eq!(fields.code, "sparql-results-language-tag");
    assert!(matches!(
        fields.field("language"),
        ErrorValue::Text("bad_tag")
    ));
    let source = std::error::Error::source(&error).unwrap();
    let native = source
        .downcast_ref::<purrdf_iri::langtag::LanguageTagError>()
        .unwrap();
    assert_eq!(fields.child("cause").code, native.diagnostic_code());
}

fn namespace_refusals_keep_spelling_and_original_iri_causes() {
    let input = String::from("urn:bad value λ");
    let error = ProvenanceNamespace::new("p", &input).unwrap_err();
    drop(input);
    let fields = Capture::read(&error).child("cause");
    assert_eq!(fields.code, "results-namespace-iri");
    assert!(matches!(
        fields.field("iri"),
        ErrorValue::Iri("urn:bad value λ")
    ));
    assert!(
        std::error::Error::source(&error)
            .unwrap()
            .source()
            .unwrap()
            .downcast_ref::<purrdf_iri::IriError>()
            .is_some()
    );
    let reserved = ProvenanceNamespace::new("XML", "https://example.org/provenance/").unwrap_err();
    assert!(matches!(
        Capture::read(&reserved).child("cause").field("prefix"),
        ErrorValue::Text("XML")
    ));
    assert!(ProvenanceNamespace::new("p", "https://example.org/provenance/").is_ok());
}

fn egress_refusals_preserve_requested_form_and_full_width_counts() {
    let error = serialize(
        &SparqlResult::Boolean(true),
        SparqlResultsFormat::Csv,
        &ResultProvenance::default(),
        None,
    )
    .unwrap_err();
    let fields = Capture::read(&error);
    assert_eq!(fields.code, "sparql-results-unsupported-form");
    assert!(matches!(fields.field("ask"), ErrorValue::Boolean(true)));
    assert_eq!(fields.child("format").code, "sparql-results-format-csv");
    assert!(
        serialize(
            &SparqlResult::Boolean(true),
            SparqlResultsFormat::Json,
            &ResultProvenance::default(),
            None
        )
        .is_ok()
    );

    let error = Error::MalformedTerm(MalformedResult::RowWidth {
        bindings: usize::MAX,
        variables: 0,
    });
    let fields = Capture::read(&error).child("cause");
    assert!(
        matches!(fields.field("bindings"), ErrorValue::Unsigned(value) if value == usize::MAX as u128)
    );
    assert!(matches!(fields.field("variables"), ErrorValue::Unsigned(0)));
    let mut result = one_literal("original");
    let SparqlResult::Solutions { variables, .. } = &mut result else {
        unreachable!()
    };
    variables[0] = "x\nλ".into();
    let error = serialize(
        &result,
        SparqlResultsFormat::Tsv,
        &ResultProvenance::default(),
        None,
    )
    .unwrap_err();
    assert!(matches!(
        Capture::read(&error).field("variable"),
        ErrorValue::Text("x\nλ")
    ));
}

struct RefusingWriter;

impl std::io::Write for RefusingWriter {
    fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
        Err(purrdf_hash::error::io_error(
            std::io::ErrorKind::BrokenPipe,
            CallerRefusal { limit: u128::MAX },
        ))
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn native_writer_causes_cross_serialization_without_formatting() {
    let error = serialize_into(
        &SparqlResult::Boolean(true),
        SparqlResultsFormat::Json,
        &ResultProvenance::default(),
        None,
        &mut RefusingWriter,
    )
    .unwrap_err();
    let events = Vec::with_capacity(2);
    let window = CurrentThreadWindow::open();
    let fields = Capture::read_with_buffer(&error, events);
    assert_eq!(window.close().allocations, 0);
    assert_eq!(fields.child("kind").code, "drain-broken-pipe");
    let mut fields = fields;
    for _ in 0..8 {
        if fields.code == "caller-refused" {
            break;
        }
        fields = fields.child("cause");
    }
    assert_eq!(fields.code, "caller-refused");
    assert!(matches!(
        fields.field("limit"),
        ErrorValue::Unsigned(u128::MAX)
    ));
    let mut source = std::error::Error::source(&error).unwrap();
    while let Some(next) = source.source() {
        source = next;
    }
    assert_eq!(
        source.downcast_ref::<CallerRefusal>().unwrap().limit,
        u128::MAX
    );
}

fn utf8_and_xml_scalars_keep_exact_native_positions() {
    let input = vec![0xff];
    let error = from_xml(&input).unwrap_err();
    drop(input);
    let fields = Capture::read(&error);
    assert!(matches!(fields.field("length"), ErrorValue::ByteCount(1)));
    assert!(matches!(
        fields.child("cause").field("valid_up_to"),
        ErrorValue::ByteCount(0)
    ));

    let result = one_literal("λ\0");
    let error = to_xml(&result, &ResultProvenance::default(), None).unwrap_err();
    drop(result);
    let fields = Capture::read(&error).child("cause");
    assert!(matches!(
        fields.field("byte_offset"),
        ErrorValue::ByteCount(2)
    ));
    assert!(matches!(
        fields.field("character"),
        ErrorValue::Character('\0')
    ));
}

purrdf_testkit::harness_main!(
    native_syntax_causes_survive_input_disposal_and_clone,
    semantic_tokens_and_language_tags_keep_original_values,
    namespace_refusals_keep_spelling_and_original_iri_causes,
    egress_refusals_preserve_requested_form_and_full_width_counts,
    native_writer_causes_cross_serialization_without_formatting,
    utf8_and_xml_scalars_keep_exact_native_positions,
);
