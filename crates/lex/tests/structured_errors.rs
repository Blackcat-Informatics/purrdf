// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Inspect real logical refusals and parser failures without matching their enums.

use purrdf_hash::error::{ErrorValue, ErrorVisitor, StructuredError};
use purrdf_lex::allocation::{CapacityError, CapacitySpace, StorageError};

#[derive(Default)]
struct Fields<'error> {
    entries: Vec<(&'static str, ErrorValue<'error>)>,
    lists: Vec<(Option<&'static str>, usize)>,
    list_ends: usize,
}

impl<'error> ErrorVisitor<'error> for Fields<'error> {
    fn variant(&mut self, code: &'static str) {
        self.entries.push(("variant", ErrorValue::Code(code)));
    }

    fn value(&mut self, name: Option<&'static str>, value: ErrorValue<'error>) {
        self.entries.push((name.unwrap_or("entry"), value));
    }

    fn begin_list(&mut self, name: Option<&'static str>, length: usize) {
        self.lists.push((name, length));
    }

    fn end_list(&mut self) {
        self.list_ends += 1;
    }
}

#[test]
fn original_capacity_counts_survive_storage_and_json_transport() {
    let widest = CapacityError::check(CapacitySpace::Quads, u128::MAX, u128::MAX - 1)
        .expect_err("the original producer retains all 128 bits");
    let mut wide_fields = Fields::default();
    widest.visit(&mut wide_fields);
    assert!(matches!(
        wide_fields.entries[2],
        ("attempted", ErrorValue::Unsigned(u128::MAX))
    ));
    assert!(
        matches!(wide_fields.entries[3], ("limit", ErrorValue::Unsigned(n)) if n == u128::MAX - 1)
    );
    let capacity = CapacityError::slot(CapacitySpace::GlobalTerms, u64::MAX, u64::MAX)
        .expect_err("the exclusive count exceeds the full-width address space");
    let storage = StorageError::from(capacity);
    let json = purrdf_lex::json::Error::new(purrdf_lex::json::ErrorKind::Storage(storage), 17);
    let mut pending: Vec<&dyn StructuredError> = vec![&json];
    let mut observed = Vec::new();
    while let Some(error) = pending.pop() {
        let mut fields = Fields::default();
        error.visit(&mut fields);
        for (name, value) in fields.entries {
            if let ErrorValue::Cause(cause) = value {
                pending.push(cause);
            } else {
                observed.push((name, value));
            }
        }
    }
    assert_eq!(observed.len(), 8);
    assert!(matches!(
        observed[0],
        ("variant", ErrorValue::Code("json-document"))
    ));
    assert!(matches!(observed[1], ("offset", ErrorValue::ByteCount(17))));
    assert!(matches!(
        observed[2],
        ("variant", ErrorValue::Code("json-storage"))
    ));
    assert!(matches!(
        observed[3],
        ("variant", ErrorValue::Code("storage-capacity"))
    ));
    assert!(matches!(
        observed[4],
        ("variant", ErrorValue::Code("capacity-exceeded"))
    ));
    assert!(matches!(
        observed[5],
        ("space", ErrorValue::Code("rdf-global-terms"))
    ));
    assert!(
        matches!(observed[6], ("attempted", ErrorValue::Unsigned(n)) if n == u128::from(u64::MAX) + 1)
    );
    assert!(matches!(observed[7], ("limit", ErrorValue::Unsigned(n)) if n == u128::from(u64::MAX)));
}

#[test]
fn parser_reports_original_escape_reason_and_byte_offset() {
    let error = purrdf_lex::json::read(r#""\uDC00""#).expect_err("a lone low surrogate is invalid");
    let mut fields = Fields::default();
    error.visit(&mut fields);
    assert_eq!(fields.entries.len(), 3);
    assert!(
        matches!(fields.entries[2], ("offset", ErrorValue::ByteCount(n)) if n == error.offset() as u128)
    );
    let ErrorValue::Cause(kind) = fields.entries[1].1 else {
        panic!("JSON must preserve its typed cause");
    };
    let mut reason = Fields::default();
    kind.visit(&mut reason);
    assert!(matches!(
        reason.entries[0],
        ("variant", ErrorValue::Code("json-invalid-escape"))
    ));
    let ErrorValue::Cause(escape) = reason.entries[1].1 else {
        panic!("the escape reason must remain typed");
    };
    let mut detail = Fields::default();
    escape.visit(&mut detail);
    assert_eq!(detail.entries.len(), 1);
    assert!(matches!(
        detail.entries[0],
        ("variant", ErrorValue::Code("json-escape-unpaired-low"))
    ));
}

#[test]
fn yaml_and_scalar_decoders_publish_native_positions_and_limits() {
    let yaml = purrdf_lex::yaml::read_with(
        "[[0]]",
        purrdf_lex::yaml::Limits {
            max_depth: 1,
            ..purrdf_lex::yaml::Limits::DEFAULT
        },
    )
    .expect_err("the nested sequence exceeds the original depth limit");
    let mut location = Fields::default();
    yaml.visit(&mut location);
    assert_eq!(location.entries.len(), 5);
    assert!(
        matches!(location.entries[2], ("offset", ErrorValue::ByteCount(n)) if n == yaml.offset() as u128)
    );
    assert!(
        matches!(location.entries[3], ("line", ErrorValue::Unsigned(n)) if n == yaml.line() as u128)
    );
    assert!(
        matches!(location.entries[4], ("column", ErrorValue::Unsigned(n)) if n == yaml.column() as u128)
    );
    let ErrorValue::Cause(kind) = location.entries[1].1 else {
        panic!("YAML must retain its original limit refusal");
    };
    let mut limit = Fields::default();
    kind.visit(&mut limit);
    assert!(matches!(
        limit.entries[0],
        ("variant", ErrorValue::Code("yaml-depth-limit"))
    ));
    assert!(matches!(
        limit.entries[1],
        ("limit", ErrorValue::Unsigned(1))
    ));

    let percent = purrdf_lex::percent::decode("%G0").expect_err("not a hexadecimal pair");
    let crockford = purrdf_lex::crockford::parse_u128("x").expect_err("not 26 digits");
    let uchar = purrdf_lex::terminals::decode_uchar(br"\uD800")
        .expect_err("a surrogate is not a Unicode scalar");
    let cases: [(&dyn StructuredError, &str); 3] = [
        (&percent, "percent-malformed"),
        (&crockford, "crockford-length"),
        (&uchar, "uchar-not-a-scalar"),
    ];
    for (error, expected) in cases {
        let mut fields = Fields::default();
        error.visit(&mut fields);
        assert!(
            matches!(fields.entries[0], ("variant", ErrorValue::Code(code)) if code == expected)
        );
        match expected {
            "percent-malformed" => assert!(matches!(
                fields.entries[1],
                ("offset", ErrorValue::ByteCount(0))
            )),
            "crockford-length" => assert!(matches!(
                fields.entries[1],
                ("found", ErrorValue::ByteCount(1))
            )),
            _ => assert_eq!(fields.entries.len(), 1),
        }
    }
}

#[test]
fn diagnostic_inspection_retains_typed_arguments_and_optional_detail() {
    use purrdf_lex::diagnostic::{DiagnosticParameter, DiagnosticPresentation, DiagnosticValue};

    let detail = DiagnosticPresentation::new("example-detail", "detail", vec![])
        .expect("a fixed template without arguments");
    let presentation = DiagnosticPresentation::new(
        "example-count",
        "count {count}",
        vec![DiagnosticParameter::new(
            "count",
            DiagnosticValue::Unsigned(u64::MAX),
        )],
    )
    .expect("the named counter exists")
    .with_detail(detail)
    .expect("one detail level is allowed");
    let mut fields = Fields::default();
    presentation.visit(&mut fields);
    assert_eq!(fields.lists, [(Some("parameters"), 1)]);
    assert_eq!(fields.list_ends, 1);
    assert!(matches!(
        fields.entries[1],
        ("message_id", ErrorValue::Code("example-count"))
    ));
    let ErrorValue::Record(parameter) = fields.entries[2].1 else {
        panic!("the list must retain structured parameters");
    };
    let mut argument = Fields::default();
    parameter.visit(&mut argument);
    assert!(matches!(
        argument.entries[1],
        ("name", ErrorValue::Text("count"))
    ));
    let ErrorValue::Record(value) = argument.entries[2].1 else {
        panic!("the counter must not be replaced with rendered English");
    };
    let mut counter = Fields::default();
    value.visit(&mut counter);
    assert!(
        matches!(counter.entries[1], ("value", ErrorValue::Unsigned(n)) if n == u128::from(u64::MAX))
    );
    let ErrorValue::Record(detail) = fields.entries[4].1 else {
        panic!("the original secondary detail must remain structured");
    };
    let mut secondary = Fields::default();
    detail.visit(&mut secondary);
    assert_eq!(secondary.lists, [(Some("parameters"), 0)]);
    assert!(matches!(
        secondary.entries[3],
        ("detail", ErrorValue::Absent)
    ));
}

#[test]
fn xml_namespace_producers_retain_typed_reasons_and_existing_messages() {
    use purrdf_lex::xml::{Document, XML_NAMESPACE, XMLNS_NAMESPACE};

    let cases = [
        (
            "<a xmlns:xmlns='https://example.org/'/>".to_owned(),
            "xml-xmlns-prefix-declaration",
            "the `xmlns` prefix cannot be declared",
        ),
        (
            "<a xmlns:xml='https://example.org/'/>".to_owned(),
            "xml-prefix-namespace-mismatch",
            "the `xml` prefix is bound to the XML namespace",
        ),
        (
            format!("<a xmlns:x='{XML_NAMESPACE}'/>"),
            "xml-namespace-prefix-mismatch",
            "only `xml` may be bound to the XML namespace",
        ),
        (
            format!("<a xmlns='{XML_NAMESPACE}'/>"),
            "xml-namespace-prefix-mismatch",
            "only `xml` may be bound to the XML namespace",
        ),
        (
            format!("<a xmlns:x='{XMLNS_NAMESPACE}'/>"),
            "xml-xmlns-namespace-declaration",
            "the xmlns namespace cannot be declared",
        ),
        (
            "<xmlns:a/>".to_owned(),
            "xml-xmlns-element-prefix",
            "an element cannot have the `xmlns` prefix",
        ),
    ];
    for (text, expected, message) in cases {
        let error = Document::parse(&text).expect_err("a reserved namespace is refused");
        assert_eq!(
            error.kind().to_string(),
            format!("reserved namespace misuse: {message}")
        );
        let mut document = Fields::default();
        error.visit(&mut document);
        assert!(
            matches!(document.entries[2], ("offset", ErrorValue::ByteCount(n)) if n == error.offset() as u128)
        );
        let ErrorValue::Cause(kind) = document.entries[1].1 else {
            panic!("XML must retain its typed kind");
        };
        let mut refusal = Fields::default();
        kind.visit(&mut refusal);
        assert!(matches!(
            refusal.entries[0],
            ("variant", ErrorValue::Code("xml-reserved-namespace"))
        ));
        let ErrorValue::Cause(reason) = refusal.entries[1].1 else {
            panic!("namespace refusals must retain the producer's reason");
        };
        let mut reason_fields = Fields::default();
        reason.visit(&mut reason_fields);
        assert_eq!(reason_fields.entries.len(), 1);
        assert!(
            matches!(reason_fields.entries[0], ("variant", ErrorValue::Code(code)) if code == expected)
        );
    }
}

#[test]
fn html_inspection_preserves_reference_ranges_and_destination_refusal() {
    use purrdf_lex::html::{Mode, resolution_layout, resolve_preallocated, resolve_strict};

    let diagnostics =
        resolve_strict("a&#0;b", Mode::Text).expect_err("a null reference is diagnosed");
    assert_eq!(diagnostics.len(), 1);
    let mut diagnostic = Fields::default();
    diagnostics[0].visit(&mut diagnostic);
    assert!(matches!(
        diagnostic.entries[2],
        ("source_start", ErrorValue::ByteCount(1))
    ));
    assert!(matches!(
        diagnostic.entries[3],
        ("source_end", ErrorValue::ByteCount(5))
    ));
    let ErrorValue::Cause(kind) = diagnostic.entries[1].1 else {
        panic!("HTML must retain the standard reference classification");
    };
    let mut classification = Fields::default();
    kind.visit(&mut classification);
    assert!(matches!(
        classification.entries[0],
        ("variant", ErrorValue::Code("null-character-reference"))
    ));

    let layout = resolution_layout("&#65;", Mode::Text).expect("a finite reference");
    let refusal = resolve_preallocated("&#65;", Mode::Text, layout, String::new(), vec![], vec![])
        .expect_err("the original caller destination has no capacity");
    let mut destination = Fields::default();
    refusal.visit(&mut destination);
    assert_eq!(destination.entries.len(), 1);
    assert!(matches!(
        destination.entries[0],
        ("variant", ErrorValue::Code("html-destination-capacity"))
    ));
}
