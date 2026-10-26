// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original ShEx producer fields through the portable public interface.

use purrdf_alloc_probe::{CountingAllocator, CurrentThreadWindow};
use purrdf_hash::error::{ErrorValue, StructuredError};
use purrdf_shex::{check_structure, parse_shape_map, parse_shexc};
use purrdf_testkit::error_capture::Capture;

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

fn actual_shape_map_keeps_original_iri_cause_after_input_disposal() {
    let input = String::from("<alice>@<https://example.org/Shape>");
    let error = parse_shape_map(&input, None).unwrap_err();
    drop(input);
    let copy = error.clone();
    drop(error);
    let fields = Capture::read(&copy);
    assert_eq!(fields.code, "shex-iri");
    assert!(matches!(fields.field("lexical"), ErrorValue::Text("alice")));
    assert_eq!(fields.child("cause").code, "iri-relative-no-base");
    assert!(
        std::error::Error::source(&copy)
            .unwrap()
            .is::<purrdf_iri::IriError>()
    );
}

fn structural_refusals_keep_the_original_unresolved_label() {
    let input = String::from("<https://example.org/Shape> @<https://example.org/missing>");
    let schema = parse_shexc(&input, None).unwrap();
    drop(input);
    let errors = check_structure(&schema).unwrap_err();
    drop(schema);
    assert_eq!(errors.len(), 1);
    let fields = Capture::read(&errors[0]);
    assert_eq!(fields.code, "shex-dangling-shape-reference");
    assert!(matches!(
        fields.field("label"),
        ErrorValue::Text("https://example.org/missing")
    ));
}

fn inspect_without_allocation(error: &dyn StructuredError) -> Capture<'_> {
    let buffer = Vec::with_capacity(24);
    let window = CurrentThreadWindow::open();
    let fields = Capture::read_with_buffer(error, buffer);
    let measured = window.close();
    assert_eq!(measured.allocations, 0, "{measured:?}");
    assert_eq!(measured.retained_bytes, 0, "{measured:?}");
    fields
}

fn lexical_refusals_keep_original_values_offsets_and_presentation() {
    use purrdf_shex::lexer::tokenize;
    // Each input reaches the public scanner. Guarded expected-integer and empty
    // regex branches are deliberately absent: their callers require a digit,
    // and `//` is an annotation token, respectively.
    let cases = [
        (
            "<<",
            "shex-lex-triple-term",
            0,
            "a ShExC schema has no triple-term value: `<<( s p o )>>` / `<< s p o >>` cannot appear in a value set or node constraint",
        ),
        (
            "\0",
            "shex-lex-unexpected-character",
            0,
            "unexpected character '\\0'",
        ),
        (
            "<a b>",
            "shex-lex-forbidden-iri-character",
            2,
            "character ' ' is not allowed in an IRI reference",
        ),
        (
            "<a",
            "shex-lex-unterminated",
            0,
            "unterminated IRI reference",
        ),
        (
            r"<\q>",
            "shex-lex-unicode-escape",
            1,
            "backslash must start a \\u/\\U escape here",
        ),
        (
            r"<\u0z00>",
            "shex-lex-unicode-escape",
            0,
            "bad \\u/\\U escape (expected hex digits)",
        ),
        (
            r"<\uD800>",
            "shex-lex-unicode-escape",
            0,
            "\\u/\\U escape is not a Unicode scalar",
        ),
        (
            "\"a",
            "shex-lex-unterminated",
            0,
            "unterminated string literal",
        ),
        ("\"\\q\"", "shex-lex-bad-escape", 1, "bad string escape \\q"),
        (
            "\"\\q\" <\\uD800>",
            "shex-lex-bad-escape",
            1,
            "bad string escape \\q",
        ),
        ("\"a\\", "shex-lex-bad-escape", 2, "bad string escape \\"),
        (
            "'a\n'",
            "shex-lex-string-newline",
            0,
            "raw newline in short string literal",
        ),
        (
            "_:.",
            "shex-lex-blank-node-label",
            0,
            "malformed blank node label",
        ),
        (
            "{1,?}",
            "shex-lex-malformed-repeat",
            0,
            "malformed repeat range",
        ),
        (
            "{1,2x}",
            "shex-lex-unterminated-repeat",
            0,
            "unterminated repeat range",
        ),
        (
            "{9223372036854775808}",
            "shex-lex-repeat-overflow",
            0,
            "repeat range bound overflows",
        ),
        (
            "{9223372036854775810}",
            "shex-lex-repeat-overflow",
            0,
            "repeat range bound overflows",
        ),
        (
            "%?",
            "shex-lex-expected-action-iri",
            0,
            "expected IRI after '%'",
        ),
        ("%word", "shex-lex-action-name", 0, "expected IRI after '%'"),
        (
            "%<urn:a>?",
            "shex-lex-expected-action-body",
            0,
            "expected '{' or '%' after semantic action name",
        ),
        (
            "%<urn:a>{",
            "shex-lex-unterminated",
            0,
            "unterminated semantic action code",
        ),
        (
            "%<urn:a>{%x%}",
            "shex-lex-unescaped-action-percent",
            9,
            "'%' in semantic action code must be escaped as \\% or close the block",
        ),
        (
            "%<urn:a>{%",
            "shex-lex-unescaped-action-percent",
            9,
            "'%' in semantic action code must be escaped as \\% or close the block",
        ),
        (
            r"%<urn:a>{\q%}",
            "shex-lex-bad-escape",
            9,
            "bad escape in semantic action code",
        ),
        ("/a", "shex-lex-unterminated", 0, "unterminated regex"),
        ("/a\n/", "shex-lex-regex-newline", 0, "raw newline in regex"),
        (r"/a\q/", "shex-lex-bad-escape", 2, "bad regex escape \\q"),
        ("/a\\", "shex-lex-bad-escape", 2, "bad regex escape \\"),
    ];
    for (input, code, offset, message) in cases {
        let owned = input.to_owned();
        let original = tokenize(&owned).unwrap_err();
        drop(owned);
        let error = original.clone();
        drop(original);
        assert_eq!(error.byte_offset(), Some(offset), "{input:?}");
        let expected = format!("ShExC lex error at byte {offset}: {message}");
        assert_eq!(error.to_string(), expected, "{input:?}");
        assert_eq!(format!("{error:?}"), expected, "{input:?}");
        let fields = inspect_without_allocation(&error);
        assert_eq!(fields.code, "shex-lex");
        assert!(
            matches!(fields.field("at"), ErrorValue::Unsigned(value) if value == offset as u128)
        );
        let ErrorValue::Cause(reason) = fields.field("reason") else {
            panic!("a native lexical reason must be a borrowed cause");
        };
        let reason = inspect_without_allocation(reason);
        assert_eq!(reason.code, code, "{input:?}");
        if matches!(
            code,
            "shex-lex-unterminated" | "shex-lex-bad-escape" | "shex-lex-unicode-escape"
        ) {
            let expected_context = match input.chars().next().unwrap() {
                '<' => "shex-lex-context-iri",
                '\'' | '"' => "shex-lex-context-string",
                '%' => "shex-lex-context-code",
                '/' => "shex-lex-context-regex",
                _ => panic!("fixture lexical context"),
            };
            assert_eq!(reason.child("context").code, expected_context);
        }
        match code {
            "shex-lex-unexpected-character" => assert!(matches!(
                reason.field("scalar"),
                ErrorValue::Character('\0')
            )),
            "shex-lex-forbidden-iri-character" => {
                assert!(matches!(reason.field("scalar"), ErrorValue::Character(' ')));
            }
            "shex-lex-string-newline" | "shex-lex-regex-newline" => assert!(matches!(
                reason.field("scalar"),
                ErrorValue::Character('\n')
            )),
            "shex-lex-blank-node-label" => assert!(matches!(
                reason.field("following"),
                ErrorValue::Character('.')
            )),
            "shex-lex-malformed-repeat"
            | "shex-lex-expected-action-iri"
            | "shex-lex-expected-action-body" => assert!(matches!(
                reason.field("following"),
                ErrorValue::Character('?')
            )),
            "shex-lex-unterminated-repeat" => assert!(matches!(
                reason.field("following"),
                ErrorValue::Character('x')
            )),
            "shex-lex-action-name" => {
                assert!(matches!(reason.field("word"), ErrorValue::Text("word")));
            }
            "shex-lex-repeat-overflow" => {
                let (operation, left, right) = if input == "{9223372036854775808}" {
                    ("shex-repeat-add", 9_223_372_036_854_775_800, 8)
                } else {
                    ("shex-repeat-multiply", 922_337_203_685_477_581, 10)
                };
                assert_eq!(reason.child("operation").code, operation);
                assert!(matches!(reason.field("left"), ErrorValue::Signed(value) if value == left));
                assert!(
                    matches!(reason.field("right"), ErrorValue::Signed(value) if value == right)
                );
            }
            "shex-lex-bad-escape" | "shex-lex-unescaped-action-percent" => {
                let following = if input.ends_with('\\') || input.ends_with('%') {
                    None
                } else if code == "shex-lex-unescaped-action-percent" {
                    Some('x')
                } else {
                    Some('q')
                };
                assert!(match (reason.field("following"), following) {
                    (ErrorValue::Absent, None) => true,
                    (ErrorValue::Character(actual), Some(expected)) => actual == expected,
                    _ => false,
                });
            }
            _ => {}
        }
        // Both schema entrypoints tokenize through the same first-fault path.
        assert_eq!(parse_shexc(input, None).unwrap_err(), error, "{input:?}");
        assert_eq!(
            purrdf_shex::ExactSchema::parse_shexc(input, None).unwrap_err(),
            error,
            "{input:?}"
        );
    }
    for valid in [
        r"<\u0041>",
        r#""\n\u0041""#,
        "_:a",
        "{1}",
        "{1,2}",
        "{9223372036854775807}",
        "%<urn:a>%",
        r"%<urn:a>{\%\\\u0041%}",
        r"/a\/b/",
    ] {
        assert!(tokenize(valid).is_ok(), "{valid:?}");
    }
    // This token is the valid neighbour of an unterminated slash construct,
    // rather than a fabricated way to reach the defensive empty-regex branch.
    assert!(matches!(
        tokenize("//").unwrap()[0].token,
        purrdf_shex::lexer::Token::AnnotMarker
    ));
}

fn lexical_native_causes_survive_source_disposal_and_imports() {
    use purrdf_shex::lexer::tokenize;
    for (input, context, cause) in [
        (r"<\q>", "shex-lex-context-iri", "uchar-not-an-escape"),
        (r"<\u00zz>", "shex-lex-context-iri", "uchar-bad-hex"),
        (r"<\uD800>", "shex-lex-context-iri", "uchar-not-a-scalar"),
        (r#""\u00zz""#, "shex-lex-context-string", "uchar-bad-hex"),
        (
            r"%<urn:a>{\u00zz%}",
            "shex-lex-context-code",
            "uchar-bad-hex",
        ),
        (r"/\u00zz/", "shex-lex-context-regex", "uchar-bad-hex"),
    ] {
        let owned = input.to_owned();
        let error = tokenize(&owned).unwrap_err();
        drop(owned);
        let copy = error.clone();
        drop(error);
        let lexical = inspect_without_allocation(&copy).child("reason");
        assert_eq!(lexical.child("context").code, context);
        let ErrorValue::Cause(native) = lexical.field("cause") else {
            panic!("native cause");
        };
        assert_eq!(inspect_without_allocation(native).code, cause);
        assert!(
            std::error::Error::source(&copy)
                .unwrap()
                .source()
                .unwrap()
                .is::<purrdf_iri::terminals::UcharError>()
        );
    }
    let input = String::from("@cantbethislong");
    let original = tokenize(&input).unwrap_err();
    drop(input);
    let error = original.clone();
    drop(original);
    assert_eq!(parse_shexc("@cantbethislong", None).unwrap_err(), error);
    assert_eq!(
        purrdf_shex::ExactSchema::parse_shexc("@cantbethislong", None).unwrap_err(),
        error
    );
    let fields = inspect_without_allocation(&error);
    let ErrorValue::Cause(reason) = fields.field("reason") else {
        panic!("native language reason");
    };
    let lexical = inspect_without_allocation(reason);
    assert_eq!(lexical.code, "shex-lex-language-tag");
    assert!(matches!(
        lexical.field("language"),
        ErrorValue::Text("cantbethislong")
    ));
    let ErrorValue::Cause(native) = lexical.field("cause") else {
        panic!("native language cause");
    };
    assert_eq!(
        inspect_without_allocation(native).code,
        "langtag-subtag-length-over-eight"
    );
    assert!(
        std::error::Error::source(&error)
            .unwrap()
            .source()
            .unwrap()
            .is::<purrdf_iri::langtag::LanguageTagError>()
    );
    assert_eq!(
        error.to_string(),
        "ShExC lex error at byte 0: malformed language tag @cantbethislong: language-tag subtag longer than 8 characters [langtag-subtag-length-over-eight]"
    );
    for valid in ["@abcdefgh", "@x-cantbethislong", "@en-US"] {
        assert!(tokenize(valid).is_ok(), "{valid:?}");
    }
    let root = parse_shexc("IMPORT <https://example.org/import>", None).unwrap();
    let imported = purrdf_shex::resolve_imports(root, &|_| Err(error.clone())).unwrap_err();
    let fields = Capture::read(&imported);
    assert_eq!(fields.code, "shex-import");
    assert_eq!(
        fields.child("cause").child("reason").child("cause").code,
        "langtag-subtag-length-over-eight"
    );
    let healthy = parse_shexc("IMPORT <https://example.org/import>", None).unwrap();
    assert!(
        purrdf_shex::resolve_imports(healthy, &|_| parse_shexc(
            "<https://example.org/S> IRI",
            None
        ))
        .is_ok()
    );
}

fn shexj_ingress_retains_original_json_causes_and_precedence() {
    use purrdf_lex::json::{self, Limits};
    use purrdf_shex::{ExactSchema, JsonMemberReason, parse_shexj};

    let limits = Limits {
        unique_members: true,
        ..Limits::with_depth(128)
    };
    // The array would also be an invalid ShExJ root, but JSON depth refusal
    // precedes schema interpretation. The existing 128-container ShExJ control
    // supplies its valid boundary neighbour without another schema generator.
    let deep = format!("{}null{}", "[".repeat(129), "]".repeat(129));
    let cases = [
        ("{", "json-expected", None),
        ("\"raw\ncontrol\"", "json-raw-control", None),
        ("\"\\", "json-invalid-escape", Some("json-escape-truncated")),
        (
            r#""\q""#,
            "json-invalid-escape",
            Some("json-escape-invalid"),
        ),
        (
            r#""\u0z00""#,
            "json-invalid-escape",
            Some("json-escape-bad-hex"),
        ),
        (
            r#""\uD800""#,
            "json-invalid-escape",
            Some("json-escape-unpaired-high"),
        ),
        (
            r#""\uDC00""#,
            "json-invalid-escape",
            Some("json-escape-unpaired-low"),
        ),
        ("null true", "json-trailing-data", None),
        (
            r#"{"type":"Schema","type":"Schema"}"#,
            "json-duplicate-member",
            None,
        ),
        (deep.as_str(), "json-depth-limit", None),
    ];
    for (input, kind_code, escape_code) in cases {
        let original = json::read_with(input, limits).unwrap_err();
        let owned = input.to_owned();
        let first = parse_shexj(&owned, None).unwrap_err();
        drop(owned);
        let error = first.clone();
        drop(first);
        assert_eq!(ExactSchema::parse_shexj(input, None).unwrap_err(), error);
        assert_eq!(
            parse_shexj(input, Some("not an absolute base")).unwrap_err(),
            error,
            "the original JSON fault precedes base resolution"
        );
        let expected = format!("ShExJ error: invalid JSON: {original}");
        assert_eq!(error.to_string(), expected);
        assert_eq!(format!("{error:?}"), expected);
        assert_eq!(error.byte_offset(), None);
        assert_eq!(error.locate(input), None);
        let source = std::error::Error::source(&error).unwrap();
        assert_eq!(source.downcast_ref::<json::Error>().unwrap(), &original);
        assert!(
            source.source().is_none(),
            "native JSON source semantics are unchanged"
        );
        let fields = inspect_without_allocation(&error);
        assert_eq!(fields.code, "shex-json-read");
        let ErrorValue::Cause(cause) = fields.field("cause") else {
            panic!("original JSON cause");
        };
        let json_fields = inspect_without_allocation(cause);
        assert_eq!(json_fields.code, "json-document");
        assert!(
            matches!(json_fields.field("offset"), ErrorValue::ByteCount(offset) if offset == original.offset() as u128)
        );
        let ErrorValue::Cause(kind) = json_fields.field("kind") else {
            panic!("original JSON kind");
        };
        let kind_fields = inspect_without_allocation(kind);
        assert_eq!(kind_fields.code, kind_code);
        if kind_code == "json-expected" {
            assert!(matches!(
                kind_fields.field("expected"),
                ErrorValue::Text("a `\"`-quoted member name")
            ));
        }
        if kind_code == "json-depth-limit" {
            assert!(matches!(
                kind_fields.field("limit"),
                ErrorValue::Unsigned(128)
            ));
        }
        if let Some(code) = escape_code {
            let ErrorValue::Cause(escape) = kind_fields.field("cause") else {
                panic!("original escape kind");
            };
            assert_eq!(inspect_without_allocation(escape).code, code);
        }
    }
    for valid in [
        r#"{"type":"Schema"}"#,
        r#"{"type":"Schema","shapes":[{"type":"NodeConstraint","id":"https://example.org/S","pattern":"a\nb"}]}"#,
        r#"{"type":"Schema","shapes":[{"type":"NodeConstraint","id":"https://example.org/S","pattern":"\uD834\uDD1E"}]}"#,
    ] {
        assert!(parse_shexj(valid, None).is_ok(), "{valid}");
        assert!(ExactSchema::parse_shexj(valid, None).is_ok(), "{valid}");
    }
    let valid_json = r#"{"type":"Schema"}"#;
    let base_fault = parse_shexj(valid_json, Some("not an absolute base")).unwrap_err();
    assert_eq!(Capture::read(&base_fault).code, "shex-iri");
    let schema_fault = parse_shexj("[]", None).unwrap_err();
    assert_eq!(Capture::read(&schema_fault).code, "shex-json");
    let schema_cause = std::error::Error::source(&schema_fault).unwrap();
    assert!(schema_cause.is::<JsonMemberReason>());
    assert!(
        schema_cause
            .source()
            .unwrap()
            .is::<json::record::DecodeError>()
    );
    let root = parse_shexj(
        r#"{"type":"Schema","imports":["https://example.org/import"]}"#,
        None,
    )
    .unwrap();
    let malformed = String::from("{");
    let first = purrdf_shex::resolve_imports(root, &|_| parse_shexj(&malformed, None)).unwrap_err();
    drop(malformed);
    let imported = first.clone();
    drop(first);
    let fields = Capture::read(&imported);
    assert_eq!(fields.code, "shex-import");
    assert!(matches!(
        fields.field("iri"),
        ErrorValue::Iri("https://example.org/import")
    ));
    assert_eq!(fields.child("cause").code, "shex-json-read");
    assert_eq!(
        fields.child("cause").child("cause").child("kind").code,
        "json-expected"
    );
    let root = parse_shexj(
        r#"{"type":"Schema","imports":["https://example.org/import"]}"#,
        None,
    )
    .unwrap();
    assert!(purrdf_shex::resolve_imports(root, &|_| parse_shexj(valid_json, None)).is_ok());
}

fn shape_map_positions_are_bytes_after_unicode() {
    let source = r#""é"!"#;
    let error = parse_shape_map(source, None).unwrap_err();
    assert_eq!(error.byte_offset(), Some(4));
    let position = error.locate(source).unwrap();
    assert_eq!((position.line, position.column), (1, 4));
    assert!(parse_shape_map(r#""é" @START"#, None).is_ok());
    let ascii = parse_shape_map(r#""x"!"#, None).unwrap_err();
    assert_eq!(ascii.byte_offset(), Some(3));
    let fields = inspect_without_allocation(&error);
    assert!(matches!(
        fields.field("scalar_offset"),
        ErrorValue::Unsigned(3)
    ));
    let multiline = "\n\"é\"!";
    let line_error = parse_shape_map(multiline, None).unwrap_err();
    assert_eq!(line_error.byte_offset(), Some(5));
    let position = line_error.locate(multiline).unwrap();
    assert_eq!((position.line, position.column), (2, 4));
    let schema = parse_shexc("<urn:S> .", None).unwrap();
    let exact = purrdf_shex::ExactSchema::parse_shexc("<urn:S> .", None).unwrap();
    let data = purrdf_core::RdfDatasetBuilder::new().freeze().unwrap();
    let options = purrdf_shex::ValidationOptions::default();
    assert_eq!(
        purrdf_shex::validate_shape_map(&schema, &data, source, None, &options).unwrap_err(),
        error
    );
    assert_eq!(
        purrdf_shex::validate_shape_map_exact(&exact, &data, source, None, &options).unwrap_err(),
        error
    );
}

fn shape_map_refusals_keep_native_grammar_and_scalars() {
    let cases = [
        (
            r#""é"!"#,
            4,
            "shape-map-expected",
            Some("shape-map-expect-association"),
            "expected '@' between node selector and shape",
        ),
        (
            "{<urn:s> <urn:p> <urn:o>}@START",
            17,
            "shape-map-expected",
            Some("shape-map-expect-focus"),
            "expected FOCUS in triple pattern",
        ),
        (
            "!",
            0,
            "shape-map-expected",
            Some("shape-map-expect-term"),
            "expected a term (<iri>, _:blank, \"literal\" or <<( s p o )>>)",
        ),
        (
            "<<( <urn:s> <urn:p> <urn:o>)> @START",
            27,
            "shape-map-expected",
            Some("shape-map-expect-triple-close"),
            "expected ')>>' to close a triple term opened with '<<('",
        ),
        (
            "{FOCUS ! _}@START",
            7,
            "shape-map-expected",
            Some("shape-map-expect-predicate"),
            "expected a predicate (<iri> or 'a')",
        ),
        (
            r#""x" @!"#,
            5,
            "shape-map-expected",
            Some("shape-map-expect-shape"),
            "expected a shape label (START or <iri>)",
        ),
        (
            r#""x"^^! @START"#,
            5,
            "shape-map-expected",
            Some("shape-map-expect-iri"),
            "expected an IRI in angle brackets",
        ),
        (
            "_",
            1,
            "shape-map-expected",
            Some("shape-map-expect-blank-colon"),
            "expected ':' after '_' in blank node",
        ),
        (
            "_:.",
            2,
            "shape-map-expected",
            Some("shape-map-expect-blank-start"),
            "a blank-node label must begin with PN_CHARS_U or a digit after `_:`",
        ),
        (
            "{FOCUS <urn:p> _ @START",
            17,
            "shape-map-expected",
            Some("shape-map-expect-character"),
            "expected '}'",
        ),
        (
            r#""x" @START !"#,
            11,
            "shape-map-trailing",
            None,
            "trailing input after shape map",
        ),
        (
            "<< <urn:s> <urn:p> <urn:o> >> @START",
            0,
            "shape-map-reifier",
            None,
            "`<< s p o >>` is RDF 1.2 reifier syntax, not a triple term; write a triple term as `<<( s p o )>>`",
        ),
        (
            "<urn:x",
            6,
            "shape-map-unterminated",
            None,
            "unterminated IRI",
        ),
        (
            "<urn:x y>",
            6,
            "shape-map-unterminated",
            None,
            "unterminated IRI",
        ),
        (
            "\"é\n",
            3,
            "shape-map-unterminated",
            None,
            "unterminated string literal",
        ),
        (
            r"<\q>",
            1,
            "shape-map-unicode-escape",
            None,
            "a backslash in an IRI must open a \\u/\\U escape",
        ),
        (
            r"<\u00zz>",
            1,
            "shape-map-unicode-escape",
            None,
            "bad \\u/\\U escape in IRI (expected hex digits)",
        ),
        (
            r"<\uD800>",
            1,
            "shape-map-unicode-escape",
            None,
            "\\u/\\U escape in IRI is not a Unicode scalar value",
        ),
        (
            r#""\u00zz" @START"#,
            1,
            "shape-map-unicode-escape",
            None,
            "bad \\u/\\U escape in string (expected hex digits)",
        ),
        (
            "\"é\\",
            4,
            "shape-map-dangling-escape",
            None,
            "dangling escape",
        ),
        (
            r#""x\q" @START"#,
            3,
            "shape-map-invalid-escape",
            None,
            "invalid string escape",
        ),
    ];
    for (input, at, code, expectation, message) in cases {
        let owned = input.to_owned();
        let original = parse_shape_map(&owned, None).unwrap_err();
        drop(owned);
        let error = original.clone();
        drop(original);
        assert_eq!(error.byte_offset(), Some(at), "{input:?}");
        let expected = format!("ShExC syntax error at byte {at}: {message}");
        assert_eq!(error.to_string(), expected, "{input:?}");
        assert_eq!(format!("{error:?}"), expected, "{input:?}");
        let fields = inspect_without_allocation(&error);
        assert_eq!(fields.code, "shex-shape-map-syntax");
        assert!(
            matches!(fields.field("at"), ErrorValue::ByteCount(actual) if actual == at as u128)
        );
        assert!(
            matches!(fields.field("scalar_offset"), ErrorValue::Unsigned(actual) if actual == input[..at].chars().count() as u128)
        );
        let ErrorValue::Cause(reason) = fields.field("reason") else {
            panic!("native map reason");
        };
        let reason = inspect_without_allocation(reason);
        assert_eq!(reason.code, code, "{input:?}");
        if let Some(expected) = expectation {
            let observed = input[at..].chars().next();
            assert_eq!(reason.child("expected").code, expected);
            assert!(match (reason.field("found"), observed) {
                (ErrorValue::Character(actual), Some(wanted)) => actual == wanted,
                (ErrorValue::Absent, None) => true,
                _ => false,
            });
            if expected == "shape-map-expect-triple-close" {
                assert!(matches!(reason.field("found"), ErrorValue::Character(')')));
                assert!(matches!(reason.field("next"), ErrorValue::Character('>')));
                assert!(matches!(
                    reason.field("after_next"),
                    ErrorValue::Character(' ')
                ));
            }
            if expected == "shape-map-expect-character" {
                assert!(matches!(
                    reason.child("expected").field("scalar"),
                    ErrorValue::Character('}')
                ));
            }
        }
        if code == "shape-map-unicode-escape" {
            let ErrorValue::Cause(cause) = reason.field("cause") else {
                panic!("native UCHAR cause");
            };
            let native = inspect_without_allocation(cause);
            let expected = if input.contains("D800") {
                "uchar-not-a-scalar"
            } else if input.contains("00zz") {
                "uchar-bad-hex"
            } else {
                "uchar-not-an-escape"
            };
            assert_eq!(native.code, expected);
            assert_eq!(
                reason.child("context").code,
                if input.starts_with('"') {
                    "shape-map-context-string"
                } else {
                    "shape-map-context-iri"
                }
            );
            assert!(
                std::error::Error::source(&error)
                    .unwrap()
                    .source()
                    .unwrap()
                    .is::<purrdf_iri::terminals::UcharError>()
            );
        } else {
            assert!(
                std::error::Error::source(&error)
                    .unwrap()
                    .source()
                    .is_none()
            );
        }
        if code == "shape-map-invalid-escape" {
            assert!(matches!(reason.field("found"), ErrorValue::Character('q')));
        }
        if code == "shape-map-trailing" {
            assert!(matches!(reason.field("found"), ErrorValue::Character('!')));
        }
        if code == "shape-map-reifier" {
            assert!(matches!(reason.field("found"), ErrorValue::Character(' ')));
        }
    }
    for input in [
        r#""é" @START"#,
        "{FOCUS <urn:p> _}@START",
        "{<urn:s> <urn:p> FOCUS}@START",
        "_:a@START",
        r#""x"^^<urn:t> @START"#,
        "<<( <urn:s> <urn:p> <urn:o>)>> @START",
        r"<urn:\u0041>@START",
        r#""\n\u0041" @START"#,
    ] {
        assert!(parse_shape_map(input, None).is_ok(), "{input}");
    }
}

fn shape_map_owned_names_and_language_causes_survive_input_disposal() {
    for (input, name, role) in [
        ("ex:n @START", "ex:n", "shape-map-position-node"),
        (
            "{FOCUS ex:p _}@START",
            "ex:p",
            "shape-map-position-predicate",
        ),
        ("<urn:s>@ex:S", "ex:S", "shape-map-position-shape"),
        (r#""x"^^ex:t @START"#, "ex:t", "shape-map-position-iri"),
    ] {
        let input = input.to_owned();
        let original = parse_shape_map(&input, None).unwrap_err();
        drop(input);
        let error = original.clone();
        drop(original);
        let fields = inspect_without_allocation(&error);
        let ErrorValue::Cause(reason) = fields.field("reason") else {
            panic!("native reason");
        };
        let reason = inspect_without_allocation(reason);
        assert_eq!(reason.code, "shape-map-prefixed-name");
        assert!(matches!(reason.field("name"), ErrorValue::Text(actual) if actual == name));
        assert_eq!(reason.child("position").code, role);
        assert!(
            error
                .to_string()
                .contains("the ShapeMap grammar admits none")
        );
    }
    // Only classifications reachable through this ASCII-alnum/hyphen scanner.
    // String UCHAR NotAnEscape and RFC-only language failures are not fabricated.
    for (input, raw, tag, direction, cause) in [
        (
            r#""x"@en---rtl @START"#,
            "en---rtl",
            "en-",
            Some("rtl"),
            "langtag-subtag-length-zero",
        ),
        (
            r#""x"@en--bad @START"#,
            "en--bad",
            "en--bad",
            None,
            "langtag-subtag-length-zero",
        ),
        (
            r#""x"@cantbethislong @START"#,
            "cantbethislong",
            "cantbethislong",
            None,
            "langtag-subtag-length-over-eight",
        ),
        (
            r#""x"@9 @START"#,
            "9",
            "9",
            None,
            "langtag-terminal-primary-not-alpha",
        ),
    ] {
        let input = input.to_owned();
        let original = parse_shape_map(&input, None).unwrap_err();
        drop(input);
        let error = original.clone();
        drop(original);
        let fields = inspect_without_allocation(&error);
        let ErrorValue::Cause(reason) = fields.field("reason") else {
            panic!("native map reason");
        };
        let reason = inspect_without_allocation(reason);
        let ErrorValue::Cause(language) = reason.field("cause") else {
            panic!("native language payload");
        };
        let language = inspect_without_allocation(language);
        assert_eq!(language.code, "shape-map-language");
        assert!(matches!(language.field("raw"), ErrorValue::Text(actual) if actual == raw));
        assert!(matches!(language.field("tag"), ErrorValue::Text(actual) if actual == tag));
        assert!(
            matches!(language.field("tag_bytes"), ErrorValue::ByteCount(actual) if actual == tag.len() as u128)
        );
        assert!(matches!(
            language.field("profile"),
            ErrorValue::Code("concrete-syntax-langtag-bounded")
        ));
        assert!(match (language.field("direction"), direction) {
            (ErrorValue::Absent, None) => true,
            (ErrorValue::Code(actual), Some(wanted)) => actual == wanted,
            _ => false,
        });
        let ErrorValue::Cause(native) = language.field("cause") else {
            panic!("native language cause");
        };
        assert_eq!(inspect_without_allocation(native).code, cause);
        let payload = std::error::Error::source(&error)
            .unwrap()
            .source()
            .unwrap()
            .downcast_ref::<purrdf_shex::ShapeMapLanguageRefusal>()
            .unwrap();
        assert_eq!(payload.raw(), raw);
        assert_eq!(payload.tag(), tag);
        assert_eq!(
            payload
                .direction()
                .map(purrdf_core::RdfTextDirection::as_str),
            direction
        );
        assert_eq!(payload.cause().diagnostic_code(), cause);
        assert!(
            std::error::Error::source(payload)
                .unwrap()
                .is::<purrdf_iri::langtag::LanguageTagError>()
        );
    }
    for input in [
        r#""x"@en--rtl @START"#,
        r#""x"@abcdefgh @START"#,
        r#""x"@x-cantbethislong @START"#,
    ] {
        assert!(parse_shape_map(input, None).is_ok(), "{input}");
    }
}

fn schema_grammar_refusals_keep_original_tokens_and_positions() {
    use purrdf_shex::{ExactSchema, GrammarExpectation as E, GrammarReason, ShexError};
    // The keyword fallback and guarded expect calls are defensive paths, not
    // fabricated scanner inputs. These cases reach the public production seam.
    for (input, expected, token, reason) in [
        (
            "START .",
            E::StartEquals,
            Some("."),
            "expected '=' after start, found Some(Dot)",
        ),
        (
            "<urn:S> { true . }",
            E::Predicate,
            Some("true"),
            "expected predicate, found Some(Word(\"true\"))",
        ),
        (
            "PREFIX true",
            E::PrefixName,
            Some("true"),
            "expected prefix name after PREFIX",
        ),
        (
            "BASE true",
            E::BaseIri,
            Some("true"),
            "expected IRI after BASE",
        ),
        ("IMPORT", E::ImportIri, None, "expected IRI after IMPORT"),
        (
            "<urn:S> +",
            E::ShapeExpression,
            Some("+"),
            "expected shape expression, found Some(Plus)",
        ),
        (
            "<urn:S> LENGTH .",
            E::StringFacet,
            Some("."),
            "expected non-negative INTEGER as string facet value",
        ),
        (
            "<urn:S> TOTALDIGITS .",
            E::DigitFacet,
            Some("."),
            "expected non-negative INTEGER as digit-count facet value",
        ),
        (
            "<urn:S> MININCLUSIVE .",
            E::NumericFacet,
            Some("."),
            "expected numeric literal as facet value, found Some(Dot)",
        ),
        (
            "<urn:S> [ . ]",
            E::WildcardExclusion,
            Some("]"),
            "expected '-' exclusion after '.' in a value set",
        ),
        (
            "<urn:S> [ . - true ]",
            E::Exclusion,
            Some("true"),
            "expected IRI, literal or language tag exclusion, found Some(Word(\"true\"))",
        ),
        (
            "<urn:S> [ @en~ - 1 ]",
            E::LanguageExclusion,
            Some("1"),
            "expected language tag exclusion",
        ),
        (
            "<urn:S> [ \"x\"~ - @en ]",
            E::Literal,
            Some("@en"),
            "expected literal, found Some(LangTag(\"en\"))",
        ),
        (
            "<urn:é> ( .",
            E::RightParen,
            None,
            "expected ')', found None",
        ),
    ] {
        let owned = input.to_owned();
        let error = parse_shexc(&owned, None).unwrap_err();
        let exact = ExactSchema::parse_shexc(&owned, None).unwrap_err();
        assert_eq!(error, exact, "{input}");
        let ShexError::Grammar { reason: native, at } = &error else {
            panic!("{input}: {error}");
        };
        let (GrammarReason::Expected {
            expected: actual,
            encountered,
        }
        | GrammarReason::FacetValue {
            expected: actual,
            encountered,
            ..
        }) = native.as_ref()
        else {
            panic!("{input}: {error}");
        };
        assert_eq!(*actual, expected, "{input}");
        if let GrammarReason::FacetValue { facet, .. } = native.as_ref() {
            let keyword = owned.split_whitespace().nth(1).unwrap();
            assert_eq!(&owned[facet.start..facet.end], keyword);
            assert_eq!(
                facet.token,
                purrdf_shex::lexer::Token::Word(keyword.to_owned())
            );
            let _ = inspect_without_allocation(facet);
            let fields = inspect_without_allocation(native.as_ref());
            let facet_fields = fields.child("facet");
            assert!(
                matches!(facet_fields.child("token").field("value"), ErrorValue::Text(value) if value == keyword)
            );
        }

        if let Some(text) = token {
            let operand = encountered.as_ref().unwrap();
            assert_eq!(&owned[operand.start..operand.end], text, "{input}");
            let fields = inspect_without_allocation(operand);
            assert!(
                matches!(fields.field("start"), ErrorValue::ByteCount(value) if value == operand.start as u128)
            );
            assert!(
                matches!(fields.field("end"), ErrorValue::ByteCount(value) if value == operand.end as u128)
            );
            let _ = inspect_without_allocation(&operand.token);
            // Wildcard dispatch probes beyond '-' while historical at stays '-'.
            if expected == E::Exclusion {
                assert_eq!(&owned[*at..=*at], "-");
            }
        } else {
            assert!(encountered.is_none());
            assert_eq!(*at, owned.len());
        }
        let at = *at;
        assert_eq!(
            error.to_string(),
            format!("ShExC syntax error at byte {at}: {reason}")
        );
        assert_eq!(format!("{error:?}"), error.to_string());
        assert_eq!(error.byte_offset(), Some(at));
        assert_eq!(
            error.locate(&owned).unwrap(),
            purrdf_iri::LineIndex::new(&owned).locate(&owned, at)
        );
        drop(owned);
        let cloned = error.clone();
        drop(error);
        let fields = inspect_without_allocation(&cloned);
        assert_eq!(fields.code, "shex-syntax");
        assert!(matches!(fields.field("at"), ErrorValue::Unsigned(value) if value == at as u128));
        let ErrorValue::Cause(native) = fields.field("reason") else {
            panic!("native grammar cause");
        };
        let _ = inspect_without_allocation(native);
        assert!(
            std::error::Error::source(&cloned)
                .unwrap()
                .is::<GrammarReason>()
        );
    }
    for (input, keyword) in [
        ("<urn:S> mAxLeNgTh .", "mAxLeNgTh"),
        ("<urn:S> FRACTIONDIGITS .", "FRACTIONDIGITS"),
        ("<urn:S> MAXEXCLUSIVE .", "MAXEXCLUSIVE"),
    ] {
        let error = parse_shexc(input, None).unwrap_err();
        let ShexError::Grammar { reason, .. } = &error else {
            panic!("{error}");
        };
        let GrammarReason::FacetValue { facet, .. } = reason.as_ref() else {
            panic!("{error}");
        };
        assert_eq!(&input[facet.start..facet.end], keyword);
        assert_eq!(
            facet.token,
            purrdf_shex::lexer::Token::Word(keyword.to_owned())
        );
    }
    // Successful neighbors use the same parsing paths, including exact facets.
    for input in [
        "PREFIX ex: <urn:> BASE <urn:base> IMPORT <urn:other> START = @ex:S ex:S {}",
        "<urn:S> { <urn:p> . }",
        "<urn:S> LENGTH 1",
        "<urn:S> TOTALDIGITS 1",
        "<urn:S> MININCLUSIVE 1",
        "<urn:S> [ . - <urn:x> ]",
        "<urn:S> [ @en~ - @fr ]",
        "<urn:S> [ \"x\"~ - \"y\" ]",
        "<urn:é> ( . )",
    ] {
        assert!(parse_shexc(input, None).is_ok(), "{input}");
        assert!(ExactSchema::parse_shexc(input, None).is_ok(), "{input}");
    }
    // The full source is tokenized before grammar admission, preserving priority.
    assert!(matches!(
        parse_shexc("START . <\\q>", None),
        Err(ShexError::Lex { .. })
    ));
}

fn schema_grammar_placement_and_rich_operands_survive_input_disposal() {
    use purrdf_shex::lexer::{CodeName, Token};
    use purrdf_shex::{GrammarReason, ShexError};
    for (source, expected_text) in [
        ("PREFIX ex:local <urn:>", "PREFIX name must end with ':'"),
        ("PREFIX ex: true", "expected IRI after PREFIX name"),
        (
            "<urn:S> [ \"é\"@EN^^<urn:t> ]",
            "literal cannot carry both language tag and datatype",
        ),
        (
            "<urn:S> { $<urn:label> (&<urn:reference>) }",
            "'$' label cannot name an inclusion",
        ),
        (
            "<urn:S> { $_:label (&_:reference) }",
            "'$' label cannot name an inclusion",
        ),
    ] {
        let input = source.to_owned();
        let original = parse_shexc(&input, None).unwrap_err();
        let error = original.clone();
        drop(original);
        let ShexError::Grammar { reason, at } = &error else {
            panic!("{source}: {error}");
        };
        assert_eq!(reason.to_string(), expected_text);
        match reason.as_ref() {
            GrammarReason::PrefixLocal { encountered } => {
                assert_eq!(encountered.token, Token::PName("ex".into(), "local".into()));
            }
            GrammarReason::PrefixIri {
                prefix,
                encountered,
            } => {
                assert_eq!(&input[prefix.start..prefix.end], "ex:");
                assert_eq!(prefix.token, Token::PName("ex".into(), String::new()));
                assert_eq!(
                    encountered.as_ref().unwrap().token,
                    Token::Word("true".into())
                );
            }
            GrammarReason::LiteralSuffix {
                literal,
                language,
                marker,
            } => {
                assert_eq!(&input[literal.start..literal.end], "\"é\"");
                assert_eq!(language.token, Token::LangTag("EN".into()));
                assert_eq!(&input[marker.start..marker.end], "^^");
                assert_eq!(*at, marker.start);
            }
            GrammarReason::LabelledInclusion {
                label,
                reference,
                label_tokens,
                reference_tokens,
            } => {
                let (expected_label, expected_reference) = if source.contains("_:") {
                    ("_:label", "_:reference")
                } else {
                    ("urn:label", "urn:reference")
                };
                assert_eq!(label, expected_label);
                assert_eq!(reference, expected_reference);
                assert_eq!(label_tokens[0].token, Token::Dollar);
                assert_eq!(reference_tokens[0].token, Token::LParen);
                assert_eq!(reference_tokens.last().unwrap().token, Token::RParen);
                for operand in label_tokens.iter().chain(reference_tokens) {
                    assert_ne!(&input[operand.start..operand.end], "");
                    let _ = inspect_without_allocation(operand);
                }
                assert_eq!(&input[*at..=*at], "}");
            }
            _ => panic!("placement refusal: {error}"),
        }
        drop(input);
        let fields = inspect_without_allocation(&error);
        let ErrorValue::Cause(native) = fields.field("reason") else {
            panic!("native reason");
        };
        let fields = inspect_without_allocation(native);
        if matches!(reason.as_ref(), GrammarReason::LabelledInclusion { .. }) {
            assert!(matches!(fields.field("label"), ErrorValue::Text(_)));
            assert!(matches!(fields.field("reference"), ErrorValue::Text(_)));
        }
    }
    for input in [
        "PREFIX ex: <urn:> ex:S {}",
        "<urn:S> [ \"é\"@EN ]",
        "<urn:S> { $<urn:label> (<urn:p> .) }",
        "<urn:S> { $_:label (<urn:p> .) }",
    ] {
        assert!(parse_shexc(input, None).is_ok(), "{input}");
    }
    // Real unexpected tokens retain every payload, not a diagnostic rendering.
    for input in ["+", "/α/i", "{2,4}", "<urn:S> [ ? ]"] {
        let owned = input.to_owned();
        let original = parse_shexc(&owned, None).unwrap_err();
        drop(owned);
        let error = original.clone();
        drop(original);
        let ShexError::Grammar { reason, .. } = &error else {
            panic!("{input}: {error}");
        };
        let token = match reason.as_ref() {
            GrammarReason::Declaration { encountered } => &encountered.token,
            GrammarReason::Expected {
                encountered: Some(operand),
                ..
            }
            | GrammarReason::ValueSet {
                encountered: Some(operand),
            } => &operand.token,
            _ => panic!("unexpected refusal: {error}"),
        };
        let fields = inspect_without_allocation(token);
        match token {
            Token::Regex { pattern, flags } => {
                assert_eq!(pattern, "α");
                assert_eq!(flags, "i");
                assert!(matches!(fields.field("flags"), ErrorValue::Text("i")));
            }
            Token::Repeat { min, max } => {
                assert_eq!((*min, *max), (2, 4));
                assert!(matches!(fields.field("max"), ErrorValue::Signed(4)));
            }
            Token::Plus | Token::Question => {}
            _ => panic!("{input}: {token:?}"),
        }
    }
    // Code is itself accepted at document scope, so expose it at a shape cursor.
    for input in ["<urn:S> %ex:action{ body %}", "<urn:S> %<urn:action>%"] {
        let owned = input.to_owned();
        let original = parse_shexc(&owned, None).unwrap_err();
        drop(owned);
        let error = original.clone();
        drop(original);
        let ShexError::Grammar { reason, .. } = &error else {
            panic!("{input}: {error}");
        };
        let GrammarReason::Expected {
            encountered: Some(operand),
            ..
        } = reason.as_ref()
        else {
            panic!("{input}: {error}");
        };
        let Token::Code { name, code } = &operand.token else {
            panic!("code token");
        };
        let fields = inspect_without_allocation(&operand.token);
        let _ = inspect_without_allocation(name);
        assert_eq!(
            fields.child("name").code,
            match name {
                CodeName::PName(_, _) => "shexc-code-pname",
                CodeName::Iri(_) => "shexc-code-iri",
            }
        );
        assert_eq!(code.as_deref(), input.contains("body").then_some(" body "));
    }
    let root = parse_shexc("IMPORT <urn:import>", None).unwrap();
    let imported =
        purrdf_shex::resolve_imports(root, &|_| parse_shexc("<urn:S> [ ? ]", None)).unwrap_err();
    let fields = inspect_without_allocation(&imported);
    assert_eq!(fields.child("cause").code, "shex-syntax");
    assert!(
        std::error::Error::source(&imported)
            .unwrap()
            .source()
            .unwrap()
            .is::<GrammarReason>()
    );
}

fn facet_conflicts_keep_both_original_occurrences_and_exact_values() {
    use purrdf_shex::{ExactSchema, FacetKind as K, FacetReason, NumericLiteral, ShexError};
    for (input, message, kind, old, new) in [
        (
            "<urn:S> /old/i /new/m",
            "duplicate pattern facet",
            K::Pattern,
            0,
            0,
        ),
        (
            "<urn:S> LENGTH 1 length 2",
            "duplicate length facet",
            K::Length,
            1,
            2,
        ),
        (
            "<urn:S> mInLeNgTh 3 MINLENGTH 4",
            "duplicate minlength facet",
            K::MinLength,
            3,
            4,
        ),
        (
            "<urn:S> MAXLENGTH 5 MAXLENGTH 6",
            "duplicate maxlength facet",
            K::MaxLength,
            5,
            6,
        ),
        (
            "<urn:S> TOTALDIGITS 7 TOTALDIGITS 8",
            "duplicate totaldigits facet",
            K::TotalDigits,
            7,
            8,
        ),
        (
            "<urn:S> FRACTIONDIGITS 9 FRACTIONDIGITS 10",
            "duplicate fractiondigits facet",
            K::FractionDigits,
            9,
            10,
        ),
    ] {
        let owned = input.to_owned();
        let original = parse_shexc(&owned, None).unwrap_err();
        assert_eq!(
            ExactSchema::parse_shexc(&owned, None).unwrap_err(),
            original
        );
        let ShexError::Facet { reason, at } = &original else {
            panic!("{original}");
        };
        assert_eq!(
            original.to_string(),
            format!("ShExC syntax error at byte {at}: {message}")
        );
        assert_eq!(original.byte_offset(), Some(*at));
        assert_eq!(format!("{original:?}"), original.to_string());
        match reason.as_ref() {
            FacetReason::Pattern { previous, incoming } => {
                assert_eq!(&owned[previous.start..previous.end], "/old/i");
                assert_eq!(&owned[incoming.start..incoming.end], "/new/m");
                assert_eq!(*at, incoming.start);
                let fields = inspect_without_allocation(reason.as_ref());
                assert!(matches!(
                    fields.child("previous").child("token").field("flags"),
                    ErrorValue::Text("i")
                ));
                assert!(matches!(
                    fields.child("incoming").child("token").field("pattern"),
                    ErrorValue::Text("new")
                ));
            }
            FacetReason::UnsignedDuplicate {
                kind: actual,
                previous,
                incoming,
            } => {
                assert_eq!(*actual, kind);
                assert_eq!((previous.value(), incoming.value()), (old, new));
                assert_eq!(
                    &owned[previous.token().start..previous.token().end],
                    old.to_string()
                );
                assert_eq!(
                    &owned[incoming.token().start..incoming.token().end],
                    new.to_string()
                );
                assert!(previous.keyword().start < incoming.keyword().start);
                assert_eq!(*at, owned.len());
                let fields = inspect_without_allocation(previous);
                assert!(
                    matches!(fields.field("value"), ErrorValue::Unsigned(value) if value == u128::from(old))
                );
            }
            _ => panic!("{original}"),
        }
        drop(owned);
        let error = original.clone();
        drop(original);
        let fields = inspect_without_allocation(&error);
        assert_eq!(fields.code, "shex-syntax");
        let ErrorValue::Cause(reason) = fields.field("reason") else {
            panic!("native cause");
        };
        let _ = inspect_without_allocation(reason);
        assert!(
            std::error::Error::source(&error)
                .unwrap()
                .is::<FacetReason>()
        );
    }
    for (keyword, kind, first, second, datatype) in [
        (
            "mInInClUsIvE",
            K::MinInclusive,
            "5",
            "5.0000000000000000001",
            purrdf_xsd::XsdDatatype::Decimal,
        ),
        (
            "MINEXCLUSIVE",
            K::MinExclusive,
            "1E999",
            "2E999",
            purrdf_xsd::XsdDatatype::Double,
        ),
        (
            "MAXINCLUSIVE",
            K::MaxInclusive,
            "1.5",
            "1.50",
            purrdf_xsd::XsdDatatype::Decimal,
        ),
        (
            "MAXEXCLUSIVE",
            K::MaxExclusive,
            "-0.0",
            "0E0",
            purrdf_xsd::XsdDatatype::Double,
        ),
    ] {
        let input = format!("<urn:S> {keyword} {first} {keyword} {second}");
        let original = parse_shexc(&input, None).unwrap_err();
        assert_eq!(
            ExactSchema::parse_shexc(&input, None).unwrap_err(),
            original
        );
        let ShexError::Facet { reason, at } = &original else {
            panic!("{original}");
        };
        let FacetReason::NumericDuplicate {
            kind: actual,
            previous,
            incoming,
        } = reason.as_ref()
        else {
            panic!("{original}");
        };
        assert_eq!(*actual, kind);
        assert_eq!(previous.exact_lexical(), first);
        assert_eq!(incoming.exact_lexical(), second);
        assert_eq!(incoming.exact_datatype(), datatype);
        assert_eq!(
            &input[previous.keyword().start..previous.keyword().end],
            keyword
        );
        assert_eq!(&input[incoming.token().start..incoming.token().end], second);
        assert_eq!(*at, input.len());
        let fields = inspect_without_allocation(previous);
        assert!(
            matches!(fields.child("exact").field("lexical"), ErrorValue::Text(value) if value == first)
        );
        let _ = inspect_without_allocation(previous.value());
        let _ = inspect_without_allocation(incoming);
        if kind == K::MinInclusive {
            assert_eq!(*previous.value(), NumericLiteral::Integer(5));
            assert_eq!(*incoming.value(), NumericLiteral::Integer(5));
            assert_ne!(previous.exact_lexical(), incoming.exact_lexical());
        }
        if kind == K::MinExclusive {
            assert!(
                matches!(previous.value(), NumericLiteral::Fractional(value) if value.is_infinite())
            );
        }
        drop(input);
        let error = original.clone();
        drop(original);
        let fields = inspect_without_allocation(&error);
        let ErrorValue::Cause(reason) = fields.field("reason") else {
            panic!("native numeric evidence");
        };
        let _ = inspect_without_allocation(reason);
    }
    // Healthy neighbors admit distinct facets and preserve ordinary/exact paths.
    for input in [
        "<urn:S> /old/i",
        "<urn:S> LENGTH 1 MINLENGTH 0 MAXLENGTH 2",
        "<urn:S> TOTALDIGITS 3 FRACTIONDIGITS 2",
        "<urn:S> MININCLUSIVE 5 MAXINCLUSIVE 5.0000000000000000001",
    ] {
        assert!(parse_shexc(input, None).is_ok(), "{input}");
        assert!(ExactSchema::parse_shexc(input, None).is_ok(), "{input}");
    }
}

fn facet_admission_keeps_native_causes_datatypes_and_first_fault() {
    use purrdf_shex::{AdmittedNumericFacet, ExactSchema, FacetReason, NumericLiteral, ShexError};
    use purrdf_xsd::XsdDatatype;
    for (keyword, lexical, cause_name, message) in [
        (
            "mInLeNgTh",
            "-1",
            "invalid-digit",
            "string facet value must be a non-negative integer",
        ),
        (
            "FRACTIONDIGITS",
            "18446744073709551616",
            "positive-overflow",
            "digit-count facet value must be a non-negative integer",
        ),
    ] {
        let input = format!("<urn:S> {keyword} {lexical}");
        let original = parse_shexc(&input, None).unwrap_err();
        assert_eq!(
            ExactSchema::parse_shexc(&input, None).unwrap_err(),
            original
        );
        let ShexError::Facet { reason, at } = &original else {
            panic!("{original}");
        };
        let FacetReason::Unsigned {
            keyword: word,
            token,
            cause,
            ..
        } = reason.as_ref()
        else {
            panic!("{original}");
        };
        assert_eq!(&input[word.start..word.end], keyword);
        assert_eq!(&input[token.start..token.end], lexical);
        assert_eq!(*at, token.start);
        assert_eq!(
            original.to_string(),
            format!("ShExC syntax error at byte {at}: {message}")
        );
        assert_eq!(&lexical.parse::<u64>().unwrap_err(), cause);
        let fields = inspect_without_allocation(reason.as_ref());
        assert!(matches!(fields.field("minimum"), ErrorValue::Unsigned(0)));
        assert!(
            matches!(fields.field("maximum"), ErrorValue::Unsigned(value) if value == u128::from(u64::MAX))
        );
        assert!(
            matches!(fields.child("cause").child("kind").field("name"), ErrorValue::Code(value) if value == cause_name)
        );
        drop(input);
        let error = original.clone();
        drop(original);
        assert!(
            std::error::Error::source(&error)
                .unwrap()
                .source()
                .unwrap()
                .is::<core::num::ParseIntError>()
        );
        let _ = inspect_without_allocation(&error);
    }
    for (datatype, recognized) in [
        (
            "http://www.w3.org/2001/XMLSchema#string",
            Some(XsdDatatype::String),
        ),
        ("urn:unknown", None),
    ] {
        let input = format!(
            "PREFIX dt: <{datatype}> <urn:é> dt: FRACTIONDIGITS 2 MAXEXCLUSIVE 5.0000000000000000001 MININCLUSIVE 0 TOTALDIGITS 7 MINEXCLUSIVE -1 MAXINCLUSIVE 9"
        );
        let original = parse_shexc(&input, None).unwrap_err();
        assert_eq!(
            ExactSchema::parse_shexc(&input, None).unwrap_err(),
            original
        );
        let ShexError::Facet { reason, at } = &original else {
            panic!("{original}");
        };
        let FacetReason::Datatype {
            token,
            resolved,
            recognized: actual,
            facets,
        } = reason.as_ref()
        else {
            panic!("{original}");
        };
        assert_eq!(&input[token.start..token.end], "dt:");
        assert_eq!(resolved, datatype);
        assert_eq!(*actual, recognized);
        assert_eq!(facets.len(), 6);
        assert_eq!(*at, input.len());
        let mut previous_end = token.end;
        for (index, facet) in facets.iter().enumerate() {
            let fields = inspect_without_allocation(facet);
            assert!(fields.code.starts_with("shex-facet-admitted-"));
            match facet {
                AdmittedNumericFacet::Range { operand, .. } => {
                    assert!(operand.keyword().start > previous_end);
                    previous_end = operand.token().end;
                    if index == 1 {
                        assert_eq!(operand.exact_lexical(), "5.0000000000000000001");
                    }
                    let _ = inspect_without_allocation(operand);
                }
                AdmittedNumericFacet::Digits { operand, .. } => {
                    assert!(operand.keyword().start > previous_end);
                    previous_end = operand.token().end;
                    assert_eq!(operand.value(), if index == 0 { 2 } else { 7 });
                }
            }
        }
        assert_eq!(
            original.to_string(),
            format!(
                "ShExC syntax error at byte {at}: numeric facet on non-numeric datatype <{datatype}>"
            )
        );
        assert_eq!(
            original.locate(&input).unwrap(),
            purrdf_iri::LineIndex::new(&input).locate(&input, *at)
        );
        drop(input);
        let error = original.clone();
        drop(original);
        let fields = inspect_without_allocation(&error);
        let ErrorValue::Cause(reason) = fields.field("reason") else {
            panic!("native datatype evidence");
        };
        let _ = inspect_without_allocation(reason);
    }
    // A digit-only refusal must not capture the previous constraint's exact
    // range record: no exact bound belongs to this datatype occurrence.
    let input = "<urn:Earlier> MININCLUSIVE 4 <urn:S> <urn:unknown> TOTALDIGITS 2";
    let error = ExactSchema::parse_shexc(input, None).unwrap_err();
    let ShexError::Facet { reason, .. } = &error else {
        panic!("{error}");
    };
    let FacetReason::Datatype { facets, .. } = reason.as_ref() else {
        panic!("{error}");
    };
    assert_eq!(facets.len(), 1);
    assert!(
        matches!(&facets[0], AdmittedNumericFacet::Digits { operand, .. } if operand.value() == 2)
    );
    // Incoming conversion precedes duplicate detection; suitability comes last.
    for input in [
        "<urn:S> LENGTH 1 LENGTH -1",
        "<urn:S> <urn:unknown> TOTALDIGITS 1 TOTALDIGITS -1",
    ] {
        let error = parse_shexc(input, None).unwrap_err();
        assert!(
            matches!(error, ShexError::Facet { reason, .. } if matches!(reason.as_ref(), FacetReason::Unsigned { .. }))
        );
    }
    assert!(matches!(
        parse_shexc("<urn:S> LENGTH 1 LENGTH .", None),
        Err(ShexError::Grammar { .. })
    ));
    // Ordinary range refusal stays separate; exact parsing still saturates its
    // AST stand-in and retains the original huge bound before a duplicate.
    let huge = "9".repeat(400);
    let input = format!("<urn:S> MININCLUSIVE {huge} MININCLUSIVE 1");
    assert!(matches!(
        parse_shexc(&input, None),
        Err(ShexError::Syntax { .. })
    ));
    let error = ExactSchema::parse_shexc(&input, None).unwrap_err();
    let ShexError::Facet { reason, .. } = &error else {
        panic!("{error}");
    };
    let FacetReason::NumericDuplicate { previous, .. } = reason.as_ref() else {
        panic!("{error}");
    };
    assert_eq!(previous.exact_lexical(), huge);
    assert!(matches!(previous.value(), NumericLiteral::Fractional(value) if *value == f64::MAX));
    for input in [
        "<urn:S> MINLENGTH 0 MAXLENGTH 18446744073709551615",
        "<urn:S> <http://www.w3.org/2001/XMLSchema#decimal> MININCLUSIVE 0 TOTALDIGITS 2",
        "<urn:S> <urn:unknown> LENGTH 1",
        "<urn:S> <http://www.w3.org/2001/XMLSchema#string> /x/",
    ] {
        assert!(parse_shexc(input, None).is_ok(), "{input}");
        assert!(ExactSchema::parse_shexc(input, None).is_ok(), "{input}");
    }
    let root = parse_shexc("IMPORT <urn:import>", None).unwrap();
    let imported = purrdf_shex::resolve_imports(root, &|_| parse_shexc("<urn:S> LENGTH -1", None))
        .unwrap_err();
    let fields = inspect_without_allocation(&imported);
    assert_eq!(fields.child("cause").code, "shex-syntax");
    assert!(
        std::error::Error::source(&imported)
            .unwrap()
            .source()
            .unwrap()
            .source()
            .unwrap()
            .is::<core::num::ParseIntError>()
    );
}

fn shexj_materialization_retains_native_members_and_grammar() {
    use purrdf_shex::{ExactSchema, JsonMemberReason, ShexError};
    // Materialization admission and grammar use the public JSON ingress,
    // without cursor or malformed native Number fabrication.
    for (input, code, message) in [
        ("[]", "shexj-object", "Schema: expected a JSON object"),
        (
            r#"{"type":"Other"}"#,
            "shexj-type",
            "Schema: expected type \"Schema\", found \"Other\"",
        ),
        (
            r#"{"type":"Schema","startActs":[{"type":"SemAct"}]}"#,
            "shexj-missing",
            "SemAct: missing \"name\"",
        ),
        (
            r#"{"type":"Schema","startActs":[{"type":"SemAct","name":"urn:a","code":7}]}"#,
            "shexj-member",
            "SemAct: \"code\" must be a string",
        ),
        (
            r#"{"type":"Schema","start":{"type":"NodeConstraint","length":-0}}"#,
            "shexj-member",
            "NodeConstraint: \"length\" must be a non-negative integer",
        ),
        (
            r#"{"type":"Schema","start":{"type":"NodeConstraint","minlength":false}}"#,
            "shexj-member",
            "NodeConstraint: \"minlength\" must be a number",
        ),
        (
            r#"{"type":"Schema","start":{"type":"Shape","expression":{"type":"EachOf","max":1.0,"expressions":[]}}}"#,
            "shexj-member",
            "EachOf/OneOf: \"max\" must be an integer",
        ),
        (
            r#"{"type":"Schema","start":{"type":"Shape","expression":{"type":"TripleConstraint","predicate":"urn:p","min":null}}}"#,
            "shexj-member",
            "TripleConstraint: \"min\" must be a number",
        ),
        (
            r#"{"type":"Schema","start":{"type":"Shape","closed":null}}"#,
            "shexj-member",
            "Shape: \"closed\" must be a boolean",
        ),
        (
            r#"{"type":"Schema","start":{"type":"NodeConstraint","mininclusive":"NaN"}}"#,
            "shexj-member",
            "NodeConstraint: \"mininclusive\" must be a number",
        ),
        (
            r#"{"type":"Schema","shapes":null}"#,
            "shexj-member",
            "Schema: \"shapes\" must be an array",
        ),
        (
            r#"{"type":"Schema","z":{"b":[1.50,true],"a":null},"a":0}"#,
            "shexj-unknown-member",
            "Schema: unknown key \"z\"",
        ),
        (
            r#"{"type":"Schema","imports":["urn:i",{"x":true}]}"#,
            "shexj-entry",
            "Schema: imports entries must be strings",
        ),
        (
            r#"{"type":"Schema","shapes":[false]}"#,
            "shexj-object",
            "shapes entry: expected a JSON object",
        ),
        (
            r#"{"type":"Schema","shapes":[{"id":7,"type":"Shape"}]}"#,
            "shexj-identifier",
            "shapes entry: missing string \"id\"",
        ),
        (
            r#"{"type":"Schema","start":null}"#,
            "shexj-discriminator",
            "shapeExpr: missing \"type\" discriminator",
        ),
        (
            r#"{"type":"Schema","start":{"type":"ShapeAnd"}}"#,
            "shexj-missing",
            "ShapeAnd: missing \"shapeExprs\"",
        ),
        (
            r#"{"type":"Schema","start":{"type":"ShapeNot"}}"#,
            "shexj-missing",
            "ShapeNot: missing \"shapeExpr\"",
        ),
        (
            r#"{"type":"Schema","start":{"type":"Other"}}"#,
            "shexj-unknown-type",
            "shapeExpr: unknown type \"Other\"",
        ),
        (
            r#"{"type":"Schema","start":{"type":"NodeConstraint","values":[{"type":"IriStemRange","stem":3}]}}"#,
            "shexj-stem",
            "IriStemRange: stem must be a string or Wildcard",
        ),
        (
            r#"{"type":"Schema","start":{"type":"NodeConstraint","values":[{"type":"IriStemRange"}]}}"#,
            "shexj-missing",
            "IriStemRange: missing \"stem\"",
        ),
        (
            r#"{"type":"Schema","start":{"type":"NodeConstraint","values":[{"type":"LiteralStemRange"}]}}"#,
            "shexj-missing",
            "LiteralStemRange: missing \"stem\"",
        ),
        (
            r#"{"type":"Schema","start":{"type":"NodeConstraint","values":[{"type":"LanguageStemRange"}]}}"#,
            "shexj-missing",
            "LanguageStemRange: missing \"stem\"",
        ),
        (
            r#"{"type":"Schema","start":{"type":"Shape","extra":["urn:p",false]}}"#,
            "shexj-entry",
            "Shape: extra entries must be strings",
        ),
        (
            r#"{"type":"Schema","start":{"type":"Shape","expression":false}}"#,
            "shexj-discriminator",
            "tripleExpr: missing \"type\" discriminator",
        ),
        (
            r#"{"type":"Schema","start":{"type":"Shape","expression":{"type":"OneOf"}}}"#,
            "shexj-missing",
            "OneOf: missing \"expressions\"",
        ),
        (
            r#"{"type":"Schema","start":{"type":"Shape","expression":{"type":"Other"}}}"#,
            "shexj-unknown-type",
            "tripleExpr: unknown type \"Other\"",
        ),
        (
            r#"{"type":"Schema","start":{"type":"Shape","annotations":[{"type":"Annotation","predicate":"urn:p"}]}}"#,
            "shexj-missing",
            "Annotation: missing \"object\"",
        ),
        (
            r#"{"type":"Schema","start":{"type":"NodeConstraint","nodeKind":"other"}}"#,
            "shexj-node-kind",
            "NodeConstraint: unknown nodeKind \"other\"",
        ),
        (
            r#"{"type":"Schema","start":{"type":"NodeConstraint","values":[{"type":"LiteralStemRange","stem":false}]}}"#,
            "shexj-stem",
            "LiteralStemRange: stem must be a string or Wildcard",
        ),
        (
            r#"{"type":"Schema","start":{"type":"NodeConstraint","values":[{"type":"LanguageStemRange","stem":{"type":"Other"}}]}}"#,
            "shexj-type",
            "LanguageStemRange: expected type \"Wildcard\", found \"Other\"",
        ),
    ] {
        let owned = input.to_owned();
        let original = purrdf_shex::parse_shexj(&owned, None).unwrap_err();
        assert_eq!(
            ExactSchema::parse_shexj(&owned, None).unwrap_err(),
            original
        );
        assert_eq!(original.to_string(), format!("ShExJ error: {message}"));
        assert_eq!(format!("{original:?}"), original.to_string());
        assert_eq!(original.byte_offset(), None);
        assert_eq!(original.locate(&owned), None);
        drop(owned);
        let error = original.clone();
        drop(original);
        let fields = inspect_without_allocation(&error);
        assert_eq!(fields.code, "shex-json");
        let ErrorValue::Cause(native) = fields.field("reason") else {
            panic!("native materialization reason");
        };
        let fields = inspect_without_allocation(native);
        assert_eq!(fields.code, code, "{input}");
        assert!(
            std::error::Error::source(&error)
                .unwrap()
                .is::<JsonMemberReason>()
        );
        let ShexError::JsonMember(reason) = &error else {
            panic!("native member variant");
        };
        match reason.as_ref() {
            JsonMemberReason::Member {
                member,
                expected,
                offered,
                ..
            } => {
                let _ = inspect_without_allocation(member);
                let _ = inspect_without_allocation(expected);
                let offered_fields = inspect_without_allocation(offered);
                assert_eq!(
                    offered_fields.child("value").code,
                    match offered.value() {
                        purrdf_lex::json::Value::Null => "json-null",
                        purrdf_lex::json::Value::Bool(_) => "json-boolean",
                        purrdf_lex::json::Value::Number(_) => "json-number",
                        _ => "json-string",
                    }
                );
            }
            JsonMemberReason::Entry { index, .. } => assert_eq!(*index, 1),
            JsonMemberReason::Type { expected, .. } => {
                let _ = inspect_without_allocation(expected);
            }
            JsonMemberReason::Discriminator { offered, .. } => {
                let _ = inspect_without_allocation(offered.value());
            }
            _ => {}
        }
    }
    // Healthy counterparts span the same object/member/type and value paths.
    for input in [
        r#"{"type":"Schema","@context":null}"#,
        r#"{"type":"Schema","imports":["urn:i"],"startActs":[{"type":"SemAct","name":"urn:a","code":"7"}]}"#,
        r#"{"type":"Schema","shapes":[{"id":"urn:S","type":"Shape","closed":false,"extra":["urn:p"]}]}"#,
        r#"{"type":"Schema","start":{"type":"ShapeAnd","shapeExprs":[{"type":"ShapeOr","shapeExprs":[{"type":"ShapeExternal"}]}]}}"#,
        r#"{"type":"Schema","start":{"type":"ShapeNot","shapeExpr":{"type":"ShapeExternal"}}}"#,
        r#"{"type":"Schema","start":{"type":"NodeConstraint","nodeKind":"literal","length":0,"minlength":0,"maxlength":18446744073709551615,"mininclusive":1e999,"maxinclusive":1.5,"values":[{"type":"IriStemRange","stem":{"type":"Wildcard"},"exclusions":["urn:x"]},{"type":"LiteralStemRange","stem":"x"},{"type":"LanguageStemRange","stem":""},{"value":"v","type":"urn:custom-datatype"}]}}"#,
        r#"{"type":"Schema","start":{"type":"Shape","expression":{"type":"EachOf","min":-0,"max":-1,"expressions":[{"type":"OneOf","expressions":[{"type":"TripleConstraint","predicate":"urn:p","min":-2,"max":9223372036854775807}]}]},"annotations":[{"type":"Annotation","predicate":"urn:a","object":"urn:b"}]}}"#,
    ] {
        assert!(purrdf_shex::parse_shexj(input, None).is_ok(), "{input}");
        assert!(ExactSchema::parse_shexj(input, None).is_ok(), "{input}");
    }
}

fn shexj_rejected_values_keep_depth_spelling_priority_and_numeric_profiles() {
    use purrdf_lex::json::{self, Value};
    use purrdf_shex::{
        ExactSchema, JsonContext, JsonMember, JsonMemberReason, JsonType, NumericLiteral, ShexError,
    };
    let input = format!(
        r#"{{"type":"Schema","start":{{"type":"NodeConstraint","mininclusive":{}}}}}"#,
        "9".repeat(400)
    );
    let error = purrdf_shex::parse_shexj(&input, None).unwrap_err();
    let ShexError::JsonMember(reason) = &error else {
        panic!("{error}");
    };
    let JsonMemberReason::NumericRange {
        context,
        member,
        offered,
        converted,
    } = reason.as_ref()
    else {
        panic!("numeric range");
    };
    assert_eq!(*context, JsonContext::Type(JsonType::NodeConstraint));
    assert_eq!(*member, JsonMember::Mininclusive);
    assert_eq!(*converted, f64::INFINITY.to_bits());
    assert_eq!(
        offered.value().as_number().unwrap().lexeme(),
        "9".repeat(400)
    );
    assert_eq!(
        error.to_string(),
        "ShExJ error: NodeConstraint: \"mininclusive\" out of range"
    );
    let _ = inspect_without_allocation(reason.as_ref());
    let exact = ExactSchema::parse_shexj(&input, None).unwrap();
    let purrdf_shex::ShapeExpr::Node(node) = exact.schema().start.as_deref().unwrap() else {
        panic!("exact node");
    };
    assert!(
        matches!(node.mininclusive, Some(NumericLiteral::Fractional(value)) if value == f64::MAX)
    );
    drop(exact);
    drop(input);
    let cloned = error.clone();
    drop(error);
    let fields = inspect_without_allocation(&cloned);
    let ErrorValue::Cause(reason) = fields.field("reason") else {
        panic!("owned numeric cause");
    };
    assert_eq!(
        inspect_without_allocation(reason).code,
        "shexj-numeric-range"
    );

    // The unknown member's whole native payload survives source and DOM drop.
    // Its original member order and number spelling remain observable.
    let value = format!(
        r#"{{"b":1.50,"a":{}true{}}}"#,
        "[".repeat(60),
        "]".repeat(60)
    );
    let input = format!(r#"{{"type":"Schema","first":{value},"second":0}}"#);
    let original = purrdf_shex::parse_shexj(&input, None).unwrap_err();
    let expected = json::read(&value).unwrap();
    drop(input);
    drop(value);
    let error = original.clone();
    drop(original);
    let ShexError::JsonMember(reason) = &error else {
        panic!("owned member");
    };
    let JsonMemberReason::Unknown { name, offered, .. } = reason.as_ref() else {
        panic!("unknown member");
    };
    assert_eq!(name, "first");
    assert!(offered.value().same_text(&expected));
    let Value::Object(object) = offered.value() else {
        panic!("original object");
    };
    let fields = inspect_without_allocation(offered.value());
    assert_eq!(fields.code, "json-object");
    assert_eq!(
        object.keys().map(String::as_str).collect::<Vec<_>>(),
        ["b", "a"]
    );
    assert_eq!(
        object.get("b").unwrap().as_number().unwrap().lexeme(),
        "1.50"
    );
    let mut nested = object.get("a").unwrap();
    for _ in 0..60 {
        let _ = inspect_without_allocation(nested);
        nested = &nested.as_array().unwrap()[0];
    }
    assert_eq!(nested, &Value::Bool(true));
    drop(expected);
    drop(error);

    // Read-before-base-before-schema priority and the existing typed families.
    assert!(matches!(
        purrdf_shex::parse_shexj("{", Some("bad base")),
        Err(ShexError::JsonRead(_))
    ));
    assert!(matches!(
        purrdf_shex::parse_shexj("[]", Some("bad base")),
        Err(ShexError::Iri { .. })
    ));
    let id_first = r#"{"type":"Schema","shapes":[{"id":"relative","type":"Other"}]}"#;
    assert!(matches!(
        purrdf_shex::parse_shexj(id_first, None),
        Err(ShexError::Iri { .. })
    ));
    let language_first = r#"{"type":"Schema","start":{"type":"NodeConstraint","values":[{"value":"v","language":"en-","unknown":true}]}}"#;
    assert!(matches!(
        purrdf_shex::parse_shexj(language_first, None),
        Err(ShexError::LanguageTag { .. })
    ));
    let child_first =
        r#"{"type":"Schema","start":{"type":"ShapeNot","shapeExpr":null,"unknown":true}}"#;
    let error = purrdf_shex::parse_shexj(child_first, None).unwrap_err();
    assert!(
        matches!(error, ShexError::JsonMember(reason) if matches!(reason.as_ref(), JsonMemberReason::Discriminator { .. }))
    );
    // Unknown value-set types are literal datatype IRIs, not unknown tags.
    assert!(purrdf_shex::parse_shexj(r#"{"type":"Schema","start":{"type":"NodeConstraint","values":[{"value":"v","type":"urn:datatype"}]}}"#, None).is_ok());
    let root = parse_shexc("IMPORT <urn:import>", None).unwrap();
    let imported = purrdf_shex::resolve_imports(root, &|_| {
        purrdf_shex::parse_shexj(r#"{"type":"Schema","shapes":null}"#, None)
    })
    .unwrap_err();
    assert_eq!(
        inspect_without_allocation(&imported).child("cause").code,
        "shex-json"
    );
    assert!(
        std::error::Error::source(&imported)
            .unwrap()
            .source()
            .unwrap()
            .is::<JsonMemberReason>()
    );
}

purrdf_testkit::harness_main!(
    shexj_materialization_retains_native_members_and_grammar,
    shexj_rejected_values_keep_depth_spelling_priority_and_numeric_profiles,
    facet_conflicts_keep_both_original_occurrences_and_exact_values,
    facet_admission_keeps_native_causes_datatypes_and_first_fault,
    schema_grammar_refusals_keep_original_tokens_and_positions,
    schema_grammar_placement_and_rich_operands_survive_input_disposal,
    actual_shape_map_keeps_original_iri_cause_after_input_disposal,
    structural_refusals_keep_the_original_unresolved_label,
    lexical_refusals_keep_original_values_offsets_and_presentation,
    lexical_native_causes_survive_source_disposal_and_imports,
    shexj_ingress_retains_original_json_causes_and_precedence,
    shape_map_positions_are_bytes_after_unicode,
    shape_map_refusals_keep_native_grammar_and_scalars,
    shape_map_owned_names_and_language_causes_survive_input_disposal,
);
