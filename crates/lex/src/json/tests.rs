// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use super::{
    Error, ErrorKind, Event, Format, Kind, Limits, Number, Object, Reader, Value, occurrences,
    read, read_slice, read_with, write, write_compact, write_pretty,
};
use crate::json_escape::{JsonEscapeErrorKind, JsonEscapes};

fn kind(text: &str) -> ErrorKind {
    read(text).expect_err(text).kind()
}

fn number(lexeme: &str) -> Value {
    Value::Number(Number::from_lexeme(lexeme).expect("a JSON number"))
}

// ---- values and whitespace -------------------------------------------------

#[test]
fn each_kind_reads_to_its_own_variant() {
    assert_eq!(read("null").unwrap(), Value::Null);
    assert_eq!(read("true").unwrap(), Value::Bool(true));
    assert_eq!(read("false").unwrap(), Value::Bool(false));
    assert_eq!(read("1").unwrap(), number("1"));
    assert_eq!(read("\"a\"").unwrap(), Value::String("a".to_owned()));
    assert_eq!(read("[]").unwrap(), Value::Array(Vec::new()));
    assert_eq!(read("{}").unwrap(), Value::Object(Object::new()));
    assert_eq!(read("[true]").unwrap().kind(), Kind::Array);
}

#[test]
fn only_the_four_rfc_8259_whitespace_bytes_separate_tokens() {
    assert_eq!(
        read(" \t\r\n { \"a\" : [ 1 , 2 ] } \n").unwrap(),
        read("{\"a\":[1,2]}").unwrap()
    );
    // FORM FEED, VERTICAL TAB and NO-BREAK SPACE are not JSON whitespace.
    for bad in ["\u{c}1", "1\u{b}", "\u{a0}1"] {
        assert!(read(bad).is_err(), "{bad:?}");
    }
}

#[test]
fn an_empty_document_is_refused_and_a_bare_scalar_is_a_document() {
    assert_eq!(
        read("").unwrap_err(),
        Error::new(ErrorKind::Expected("a JSON value"), 0)
    );
    assert_eq!(kind("   "), ErrorKind::Expected("a JSON value"));
    assert_eq!(read(" 7 ").unwrap(), number("7"));
}

// ---- trailing data (refusal and its neighbour) -----------------------------

#[test]
fn trailing_garbage_is_refused_and_trailing_whitespace_is_accepted() {
    for (bad, at) in [
        ("{} x", 3),
        ("1 2", 2),
        ("[1]]", 3),
        ("nullnull", 4),
        ("\"a\"\"b\"", 3),
    ] {
        assert_eq!(
            read(bad).unwrap_err(),
            Error::new(ErrorKind::Trailing, at),
            "{bad:?}"
        );
    }
    for good in ["{} ", "1\n", "[1] \t\r\n", "null "] {
        assert!(read(good).is_ok(), "{good:?}");
    }
}

// ---- numbers keep their text ----------------------------------------------

#[test]
fn a_number_is_kept_as_its_source_lexeme_verbatim() {
    for lexeme in [
        "0",
        "-0",
        "1.5",
        "1.50",
        "15e-1",
        "1E+2",
        "-83.42391749999999999999999999999999999999",
        "123456789012345678901234567890",
        "1e308",
        "1e400",
    ] {
        let value = read(lexeme).unwrap();
        assert_eq!(value.as_number().unwrap().lexeme(), lexeme);
        assert_eq!(write_compact(&value), lexeme);
    }
    assert_ne!(read("1.5").unwrap(), read("1.50").unwrap());
}

#[test]
fn every_malformed_number_is_refused_and_its_valid_neighbour_is_not() {
    for (bad, good) in [
        ("+1", "1"),
        ("01", "0"),
        ("-01", "-0"),
        (".5", "0.5"),
        ("1.", "1.0"),
        ("1.e3", "1.0e3"),
        ("1e", "1e1"),
        ("1e+", "1e+1"),
        ("-", "-1"),
        ("NaN", "0"),
        ("Infinity", "0"),
        ("-Infinity", "-1"),
        ("0x10", "0"),
        ("1_000", "1000"),
    ] {
        assert!(read(bad).is_err(), "{bad:?} must be refused");
        assert!(Number::from_lexeme(bad).is_err(), "{bad:?}");
        assert_eq!(read(good).unwrap(), number(good), "{good:?} must read");
    }
    assert_eq!(
        read("[01]").unwrap_err(),
        Error::new(ErrorKind::Expected("no digit after a leading zero"), 2)
    );
}

#[test]
fn integer_accessors_decide_the_lexeme_exactly() {
    let n = |lexeme: &str| Number::from_lexeme(lexeme).unwrap();
    assert_eq!(n("18446744073709551615").as_u64(), Some(u64::MAX));
    assert_eq!(n("18446744073709551616").as_u64(), None);
    assert_eq!(n("-9223372036854775808").as_i64(), Some(i64::MIN));
    assert_eq!(n("-9223372036854775809").as_i64(), None);
    assert_eq!(
        n("-170141183460469231731687303715884105728").as_i128(),
        Some(i128::MIN)
    );
    assert_eq!(n("-0").as_i64(), Some(0));
    assert_eq!(n("-0").as_u64(), None);
    assert_eq!(n("1.0").as_u64(), None);
    assert_eq!(n("1e2").as_i64(), None);
    assert_eq!(n("0.1").as_f64(), 0.1);
    assert_eq!(n("1e400").as_f64(), f64::INFINITY);
    assert_eq!(n("-1e-400").as_f64().to_bits(), (-0.0_f64).to_bits());
    assert_eq!(read("[7]").unwrap()[0], 7_u8);
}

#[test]
fn binary_floats_are_spelled_in_the_shortest_round_trip_layout() {
    for (value, lexeme) in [
        (0.0, "0.0"),
        (-0.0, "-0.0"),
        (1.0, "1.0"),
        (-2.5, "-2.5"),
        (0.1, "0.1"),
        (123_456_789.0, "123456789.0"),
        (1e15, "1000000000000000.0"),
        (1e16, "1e+16"),
        (1.25e16, "1.25e+16"),
        (1e-5, "0.00001"),
        (1.5e-5, "0.000015"),
        (1e-6, "1e-6"),
        (1.5e-7, "1.5e-7"),
        (f64::MAX, "1.7976931348623157e+308"),
        (5e-324, "5e-324"),
    ] {
        let spelled = Number::from_f64(value).unwrap();
        assert_eq!(spelled.lexeme(), lexeme, "{value:e}");
        assert_eq!(spelled.as_f64().to_bits(), value.to_bits(), "{lexeme}");
    }
    assert_eq!(Number::from_f32(0.1).unwrap().lexeme(), "0.1");
    assert_eq!(Number::from_f32(1e13).unwrap().lexeme(), "1e+13");
    assert_eq!(Number::from_f32(1e12).unwrap().lexeme(), "1000000000000.0");
    assert_eq!(Value::from(f64::NAN), Value::Null);
    assert_eq!(Value::from(f64::INFINITY), Value::Null);
}

// ---- strings: escapes, surrogates, controls -------------------------------

#[test]
fn a_lone_surrogate_is_refused_and_a_valid_pair_is_accepted() {
    assert_eq!(
        read(r#"["\ud800"]"#).unwrap_err(),
        Error::new(ErrorKind::Escape(JsonEscapeErrorKind::UnpairedHigh), 8)
    );
    assert_eq!(
        kind(r#""\udc00""#),
        ErrorKind::Escape(JsonEscapeErrorKind::UnpairedLow)
    );
    assert_eq!(
        kind(r#"{"\ud800x": 1}"#),
        ErrorKind::Escape(JsonEscapeErrorKind::UnpairedHigh)
    );
    assert_eq!(read(r#""\ud83d\ude00""#).unwrap(), "\u{1f600}");
    assert_eq!(read(r#""\uD83D\uDE00""#).unwrap(), "\u{1f600}");
    assert_eq!(read("\"é中😀\"").unwrap(), "é中😀");
}

#[test]
fn a_raw_control_character_is_refused_and_its_escape_is_accepted() {
    for (bad, good, decoded) in [
        ("\"a\u{1}b\"", r#""a\u0001b""#, "a\u{1}b"),
        ("\"a\tb\"", r#""a\tb""#, "a\tb"),
        ("\"\n\"", r#""\n""#, "\n"),
    ] {
        assert_eq!(
            read(bad).unwrap_err(),
            Error::new(ErrorKind::RawControl, 2 - usize::from(bad.len() == 3))
        );
        assert_eq!(read(good).unwrap(), decoded);
    }
    // DEL and the C1 block are not controls RFC 8259 requires escaped.
    assert_eq!(read("\"\u{7f}\u{85}\"").unwrap(), "\u{7f}\u{85}");
}

#[test]
fn every_short_escape_decodes_and_an_unknown_one_is_refused() {
    assert_eq!(
        read(r#""\"\\\/\b\f\n\r\t\u0041""#).unwrap(),
        "\"\\/\u{8}\u{c}\n\r\tA"
    );
    assert_eq!(
        kind(r#""\x41""#),
        ErrorKind::Escape(JsonEscapeErrorKind::BadEscape)
    );
    assert_eq!(
        kind(r#""\u12G4""#),
        ErrorKind::Escape(JsonEscapeErrorKind::BadHex)
    );
    assert_eq!(
        kind("\"\\"),
        ErrorKind::Escape(JsonEscapeErrorKind::Truncated)
    );
    assert_eq!(
        kind("\"abc"),
        ErrorKind::Expected("the `\"` that closes a string")
    );
}

#[test]
fn non_utf8_bytes_are_refused_and_utf8_bytes_are_read() {
    assert_eq!(
        read_slice(b"[\"a\xff\"]", Limits::DEFAULT).unwrap_err(),
        Error::new(ErrorKind::InvalidUtf8, 3)
    );
    assert_eq!(
        read_slice(b"[\"a\xc3\xa9\"]", Limits::DEFAULT).unwrap()[0],
        "aé"
    );
}

/// A fixed-seed generator (SplitMix64), so every run draws the same inputs.
struct SplitMix(u64);

impl SplitMix {
    const fn next(&mut self) -> u64 {
        purrdf_testkit::rng::splitmix64_next(&mut self.0)
    }

    fn below(&mut self, n: usize) -> usize {
        usize::try_from(self.next() % n as u64).expect("below n")
    }
}

/// The per-byte string grammar, as the oracle for the chunked scan: the byte
/// after the closing quote, or the refusal and its offset.
fn string_oracle(bytes: &[u8]) -> Result<usize, (ErrorKind, usize)> {
    let mut at = 1;
    loop {
        match bytes.get(at) {
            None => return Err((ErrorKind::Expected("the `\"` that closes a string"), at)),
            Some(b'"') => return Ok(at + 1),
            Some(b'\\') => match bytes.get(at + 1) {
                Some(b'"' | b'\\' | b'/' | b'b' | b'f' | b'n' | b'r' | b't') => at += 2,
                Some(b'u') => {
                    for k in 0..4 {
                        match bytes.get(at + 2 + k) {
                            None => {
                                return Err((
                                    ErrorKind::Escape(JsonEscapeErrorKind::Truncated),
                                    at + 2 + k,
                                ));
                            }
                            Some(b) if !b.is_ascii_hexdigit() => {
                                return Err((
                                    ErrorKind::Escape(JsonEscapeErrorKind::BadHex),
                                    at + 2 + k,
                                ));
                            }
                            Some(_) => {}
                        }
                    }
                    at += 6;
                }
                None => return Err((ErrorKind::Escape(JsonEscapeErrorKind::Truncated), at)),
                Some(_) => return Err((ErrorKind::Escape(JsonEscapeErrorKind::BadEscape), at)),
            },
            Some(&b) if b < 0x20 => return Err((ErrorKind::RawControl, at)),
            Some(_) => at += 1,
        }
    }
}

/// The chunked string scan agrees with the per-byte grammar — end position, or
/// the same refusal at the same offset — on fixed-seed bodies holding every
/// special byte, whole and malformed escapes, DEL and the C1 block (lawful
/// raw), and non-ASCII in every UTF-8 width, at lengths 0-70 and past several
/// chunks.
#[test]
fn chunked_string_scan_agrees_with_the_per_byte_grammar() {
    const PIECES: &[&str] = &[
        "\"",
        "\\",
        "\\\"",
        "\\\\",
        "\\/",
        "\\b",
        "\\n",
        "\\u0001",
        "\\uD83D\\uDE00",
        "\\u12",
        "\\x",
        "\u{0}",
        "\u{1}",
        "\t",
        "\n",
        "\u{1f}",
        "\u{7f}",
        " ",
        "\u{85}",
        "\u{e9}",
        "\u{4e2d}",
        "\u{1f600}",
    ];
    let mut rng = SplitMix(0x0150_05CA_9000_0001);
    let (mut ok, mut refused) = (0_usize, 0_usize);
    for len in (0..=70).chain([127, 128, 129, 1000, 4099]) {
        for round in 0..40 {
            let density = if round % 2 == 0 { 4 } else { 60 };
            let mut text = String::from("\"");
            for _ in 0..len {
                if rng.below(density) == 0 {
                    text.push_str(PIECES[rng.below(PIECES.len())]);
                } else {
                    text.push('s');
                }
            }
            if round % 5 != 0 {
                text.push('"');
            }
            text.push_str(", 1");
            let mut reader = Reader::new(&text, Limits::DEFAULT);
            let got = reader
                .skip_value()
                .map(|span| span.end)
                .map_err(|error| (error.kind(), error.offset()));
            assert_eq!(got, string_oracle(text.as_bytes()), "{text:?}");
            if got.is_ok() {
                ok += 1;
            } else {
                refused += 1;
            }
        }
    }
    assert!(ok > 0 && refused > 0, "{ok} {refused}");
}

// ---- objects ---------------------------------------------------------------

#[test]
fn repeated_member_names_are_retained_in_document_order() {
    let value = read(r#"{"b":1,"a":2,"b":3}"#).unwrap();
    let object = value.as_object().unwrap();
    assert_eq!(object.len(), 3);
    assert_eq!(object.count("b"), 2);
    assert_eq!(object.get("b").unwrap().as_u64(), Some(1));
    assert_eq!(object.first_duplicate(), Some("b"));
    assert_eq!(
        object.keys().map(String::as_str).collect::<Vec<_>>(),
        ["b", "a", "b"]
    );
    assert_eq!(write_compact(&value), r#"{"b":1,"a":2,"b":3}"#);
}

#[test]
fn unique_members_refuses_a_decoded_repeat_and_accepts_distinct_names() {
    let limits = Limits {
        unique_members: true,
        ..Limits::DEFAULT
    };
    assert_eq!(
        read_with(r#"{"a":1,"\u0061":2}"#, limits).unwrap_err(),
        Error::new(ErrorKind::DuplicateMember, 7)
    );
    assert_eq!(
        read_with(r#"{"a":{"a":1},"b":[{"a":2}]}"#, limits)
            .unwrap()
            .pointer("/b/0/a")
            .unwrap()
            .as_u64(),
        Some(2)
    );
}

#[test]
fn members_and_elements_are_refused_where_the_grammar_forbids_them() {
    for (bad, expected) in [
        ("[1,]", "a JSON value"),
        ("{\"a\":1,}", "a `\"`-quoted member name"),
        ("{\"a\" 1}", "`:` after a member name"),
        ("[1 2]", "`,` or `]` in an array"),
        ("{\"a\":1 \"b\":2}", "`,` or `}` in an object"),
        ("{ a:1}", "a `\"`-quoted member name"),
        ("[", "a JSON value"),
        ("{", "a `\"`-quoted member name"),
        ("tru", "`true`"),
    ] {
        assert_eq!(kind(bad), ErrorKind::Expected(expected), "{bad:?}");
    }
    assert!(read("[1,2]").is_ok());
    assert!(read("{\"a\":1,\"b\":2}").is_ok());
}

// ---- bounds (refusals and their neighbours) --------------------------------

#[test]
fn the_depth_cap_accepts_depth_n_and_refuses_n_plus_one() {
    for cap in [0_usize, 1, 2, 32, 128] {
        let nested = |depth: usize| format!("{}1{}", "[".repeat(depth), "]".repeat(depth));
        let limits = Limits::with_depth(cap);
        assert!(
            read_with(&nested(cap), limits).is_ok(),
            "{cap} arrays under a cap of {cap}"
        );
        assert_eq!(
            read_with(&nested(cap + 1), limits).unwrap_err(),
            Error::new(ErrorKind::Depth { limit: cap }, cap),
            "{} arrays under a cap of {cap}",
            cap + 1
        );
        let objects = |depth: usize| format!("{}1{}", "{\"a\":".repeat(depth), "}".repeat(depth));
        assert!(read_with(&objects(cap), limits).is_ok());
        assert!(read_with(&objects(cap + 1), limits).is_err());
    }
    // The default cap is 128, and the same cap holds while skipping.
    assert!(read(&format!("{}{}", "[".repeat(128), "]".repeat(128))).is_ok());
    assert_eq!(
        kind(&format!("{}{}", "[".repeat(129), "]".repeat(129))),
        ErrorKind::Depth { limit: 128 }
    );
    let deep = format!("{}{}", "[".repeat(129), "]".repeat(129));
    assert!(Reader::new(&deep, Limits::DEFAULT).skip_value().is_err());
}

#[test]
fn the_value_cap_counts_every_value_and_not_member_names() {
    let limits = Limits {
        max_values: 3,
        ..Limits::DEFAULT
    };
    assert!(read_with("[1,2]", limits).is_ok());
    assert!(read_with(r#"{"a":1,"b":2}"#, limits).is_ok());
    assert_eq!(
        read_with("[1,2,3]", limits).unwrap_err(),
        Error::new(ErrorKind::Values { limit: 3 }, 5)
    );
}

#[test]
fn the_string_cap_bounds_decoded_bytes_not_escaped_ones() {
    let limits = Limits {
        max_string_bytes: 3,
        ..Limits::DEFAULT
    };
    // Six escaped bytes decode to one: within the cap.
    assert_eq!(read_with(r#"["\u0061bc"]"#, limits).unwrap()[0], "abc");
    assert_eq!(
        read_with(r#"["abcd"]"#, limits).unwrap_err().kind(),
        ErrorKind::StringBytes { limit: 3 }
    );
    assert!(read_with(r#"{"abcd":1}"#, limits).is_err());
    assert!(read_with(r#"{"abc":1}"#, limits).is_ok());
}

// ---- no machine-stack recursion -------------------------------------------

#[test]
fn a_hundred_thousand_deep_document_reads_clones_compares_writes_and_drops() {
    purrdf_stack::on_stack(256 * 1024, || {
        const DEPTH: usize = 100_000;
        let arrays = format!("{}0{}", "[".repeat(DEPTH), "]".repeat(DEPTH));
        let objects = format!("{}0{}", "{\"k\":".repeat(DEPTH), "}".repeat(DEPTH));
        let unbounded = Limits::with_depth(usize::MAX);
        for text in [arrays, objects] {
            let value = read_with(&text, unbounded).expect("depth is bounded by the caller");
            assert_eq!(write_compact(&value), text);
            let copy = value.clone();
            assert_eq!(copy, value);
            let mut sorted = copy.clone();
            sorted.sort_keys();
            assert_eq!(sorted, value);
            assert_eq!(format!("{value:?}").len(), text.len());
            assert_eq!(
                Reader::new(&text, unbounded).skip_value().unwrap(),
                0..text.len()
            );
            assert_eq!(occurrences(&text, unbounded).unwrap().len(), DEPTH + 1);
            drop(copy);
            drop(sorted);
            drop(value);
        }
    })
    .expect("a 256 KiB stack thread runs the walk");
}

// ---- the pull reader --------------------------------------------------------

#[test]
fn events_name_their_offsets_in_document_order() {
    let text = r#" {"a": [1, true], "b": null} "#;
    let mut reader = Reader::new(text, Limits::DEFAULT);
    let mut events = Vec::new();
    loop {
        let event = reader.next_event().unwrap();
        events.push(event);
        if event == Event::End {
            break;
        }
    }
    let keys: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            Event::Key(name) => Some((name.raw(), name.span())),
            _ => None,
        })
        .collect();
    assert_eq!(keys, [("a", 3..4), ("b", 19..20)]);
    assert_eq!(events[0], Event::BeginObject { at: 1 });
    assert_eq!(events[2], Event::BeginArray { at: 7 });
    assert_eq!(events[3], Event::Number { lexeme: "1", at: 8 });
    assert_eq!(
        events[4],
        Event::Bool {
            value: true,
            at: 11
        }
    );
    assert_eq!(events[5], Event::EndArray { at: 15 });
    assert_eq!(events[7], Event::Null { at: 23 });
    assert_eq!(events[8], Event::EndObject { at: 27 });
    assert_eq!(reader.next_event().unwrap(), Event::End);
}

/// The streaming shape a bounded results decoder takes: find one member, build
/// only a prefix of its array, syntax-check the rest without building it.
#[test]
fn a_streaming_decoder_builds_a_prefix_and_skips_the_rest() {
    let text = r#"{"head":{"x":[{}]},"rows":[{"v":1},{"v":2},{"v":[3,{"deep":true}]}],"tail":"t"}"#;
    let mut reader = Reader::new(text, Limits::DEFAULT);
    reader.begin_object().unwrap();
    let mut rows = Vec::new();
    let mut skipped = Vec::new();
    while let Some(name) = reader.next_key().unwrap() {
        if name.raw() == "rows" {
            reader.begin_array().unwrap();
            while reader.next_item().unwrap() {
                if rows.len() < 2 {
                    rows.push(reader.read_value().unwrap());
                } else {
                    skipped.push(reader.skip_value().unwrap());
                }
            }
        } else {
            skipped.push(reader.skip_value().unwrap());
        }
    }
    reader.finish().unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[1]["v"], 2_u8);
    assert_eq!(&text[skipped[1].clone()], r#"{"v":[3,{"deep":true}]}"#);
    assert_eq!(&text[skipped[2].clone()], r#""t""#);
    // A syntax error inside a skipped value is still refused.
    let mut bad = Reader::new(r#"{"a":[1,,2]}"#, Limits::DEFAULT);
    bad.begin_object().unwrap();
    bad.next_key().unwrap();
    assert!(bad.skip_value().is_err());
    // Misuse is a typed error, not a panic.
    let mut misuse = Reader::new("[1]", Limits::DEFAULT);
    assert!(misuse.next_key().is_err());
    assert!(Reader::new("[1]", Limits::DEFAULT).begin_object().is_err());
}

#[test]
fn occurrences_cover_spans_parents_ordinals_and_sizes() {
    let text = r#"{"a":[1,"x"],"a":{},"\n":false}"#;
    let all = occurrences(text, Limits::DEFAULT).unwrap();
    let kinds: Vec<_> = all.iter().map(|o| o.kind).collect();
    assert_eq!(
        kinds,
        [
            Kind::Object,
            Kind::Array,
            Kind::Number,
            Kind::String,
            Kind::Object,
            Kind::False
        ]
    );
    assert_eq!(all[0].span, 0..text.len());
    assert_eq!(all[0].size, 3);
    assert_eq!((all[1].size, all[1].span.clone()), (2, 5..12));
    assert_eq!(
        (all[3].parent, all[3].ordinal, all[3].span.clone()),
        (Some(1), 1, 9..10)
    );
    assert_eq!((all[4].ordinal, all[4].key.clone()), (1, Some(14..15)));
    assert_eq!(&text[all[5].key.clone().unwrap()], "\\n");
    assert_eq!(all[5].span.clone(), 25..30);
}

// ---- the writer -------------------------------------------------------------

#[test]
fn compact_and_pretty_layouts_match_serde_json() {
    let value = read(
        r#"{"s":"a\"b\\c\u0008\u000c\n\r\t\u0001\u007f/é","n":[1,-0.5e3,{}],"e":[],"o":{"t":true,"f":false,"z":null}}"#,
    )
    .unwrap();
    assert_eq!(
        write_compact(&value),
        "{\"s\":\"a\\\"b\\\\c\\b\\f\\n\\r\\t\\u0001\u{7f}/é\",\"n\":[1,-0.5e3,{}],\"e\":[],\"o\":{\"t\":true,\"f\":false,\"z\":null}}"
    );
    assert_eq!(
        write_pretty(&value),
        "{\n  \"s\": \"a\\\"b\\\\c\\b\\f\\n\\r\\t\\u0001\u{7f}/é\",\n  \"n\": [\n    1,\n    -0.5e3,\n    {}\n  ],\n  \"e\": [],\n  \"o\": {\n    \"t\": true,\n    \"f\": false,\n    \"z\": null\n  }\n}"
    );
    let one_space = Format {
        indent: Some(" "),
        escapes: JsonEscapes::Minimal,
    };
    assert_eq!(
        write(&read(r#"{"a":["\b"]}"#).unwrap(), one_space),
        "{\n \"a\": [\n  \"\\u0008\"\n ]\n}"
    );
    assert_eq!(write_pretty(&Value::from(3_u8)), "3");
    assert_eq!(format!("{value:#}"), write_pretty(&value));
    assert_eq!(value.to_string(), write_compact(&value));
}

#[test]
fn what_is_written_reads_back_to_the_same_value() {
    for text in [
        r#"{"a":[1,2.50,"x\u0000y",{"b":null}],"a":true,"":"\ud834\udd1e"}"#,
        "[[[],{}],[{\"\":[]}]]",
        "\"\u{2028}\u{2029}\"",
    ] {
        let value = read(text).unwrap();
        for escapes in [
            JsonEscapes::Minimal,
            JsonEscapes::ShortForms,
            JsonEscapes::Controls,
            JsonEscapes::Ascii,
        ] {
            for indent in [None, Some("  "), Some("\t")] {
                let written = write(&value, Format { indent, escapes });
                assert_eq!(read(&written).unwrap(), value, "{written}");
            }
        }
    }
}

#[test]
fn builders_and_conversions_assemble_what_the_reader_would_read() {
    let built = Value::from(
        Object::new()
            .with("id", 7_u64)
            .with("neg", -3_i32)
            .with("big", u128::MAX)
            .with("ratio", 0.25)
            .with("name", "x")
            .with("tags", vec!["a", "b"])
            .with("none", Option::<u8>::None)
            .with("nested", Value::object([("k", Value::from(true))])),
    );
    let text = r#"{"id":7,"neg":-3,"big":340282366920938463463374607431768211455,"ratio":0.25,"name":"x","tags":["a","b"],"none":null,"nested":{"k":true}}"#;
    assert_eq!(write_compact(&built), text);
    assert_eq!(read(text).unwrap(), built);
    let mut edited = built;
    edited["name"] = Value::from("y");
    edited["added"]["inner"] = Value::from(1_u8);
    edited["tags"][1] = Value::Null;
    assert_eq!(edited["name"], "y");
    assert_eq!(edited.pointer("/added/inner").unwrap().as_u64(), Some(1));
    assert_eq!(edited["tags"][1], Value::Null);
    assert!(edited["missing"].is_null());
    assert!(edited["tags"][9].is_null());
    let collected: Value = (1_u8..=3).collect();
    assert_eq!(write_compact(&collected), "[1,2,3]");
    let object: Object = [("x", 1_u8), ("y", 2)].into_iter().collect();
    assert_eq!(write_compact(&object.into()), r#"{"x":1,"y":2}"#);
}

#[test]
fn sort_keys_orders_every_object_by_name_and_keeps_repeats_in_order() {
    let mut value = read(r#"{"b":{"z":1,"a":2},"a":[{"y":1,"x":2}],"b":0}"#).unwrap();
    value.sort_keys();
    assert_eq!(
        write_compact(&value),
        r#"{"a":[{"x":2,"y":1}],"b":{"a":2,"z":1},"b":0}"#
    );
}

#[test]
fn object_insert_replaces_in_place_and_push_appends() {
    let mut object = Object::new();
    assert_eq!(object.insert("a", 1_u8), None);
    object.push("b", 2_u8);
    object.push("a", 3_u8);
    assert_eq!(object.insert("a", 4_u8).unwrap(), 1_u8);
    assert_eq!(
        write_compact(&object.clone().into()),
        r#"{"a":4,"b":2,"a":3}"#
    );
    assert_eq!(object.remove("a").unwrap(), 4_u8);
    assert_eq!(write_compact(&object.into()), r#"{"b":2,"a":3}"#);
}
