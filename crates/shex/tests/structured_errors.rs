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
    use purrdf_shex::{ExactSchema, parse_shexj};

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
    assert!(std::error::Error::source(&schema_fault).is_none());
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

purrdf_testkit::harness_main!(
    actual_shape_map_keeps_original_iri_cause_after_input_disposal,
    structural_refusals_keep_the_original_unresolved_label,
    lexical_refusals_keep_original_values_offsets_and_presentation,
    lexical_native_causes_survive_source_disposal_and_imports,
    shexj_ingress_retains_original_json_causes_and_precedence,
    shape_map_positions_are_bytes_after_unicode,
    shape_map_refusals_keep_native_grammar_and_scalars,
    shape_map_owned_names_and_language_causes_survive_input_disposal,
);
