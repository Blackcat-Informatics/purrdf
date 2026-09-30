// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use core::fmt::Write as _;

use super::{ErrorKind, Limits, read, read_with, write};
use crate::json::{self, Object, Value};

fn kind(text: &str) -> ErrorKind {
    read(text).expect_err(text).kind()
}

fn json(text: &str) -> Value {
    json::read(text).expect("a JSON fixture")
}

/// A JSON value, the bytes `serde_yaml` wrote for it, captured from
/// `serde_yaml_ng` 0.10 over the same value (members in insertion order). The
/// emitter reproduces each exactly.
const SERDE_YAML_BYTES: &[(&str, &str)] = &[
    ("{\"k\":\"true\"}", "k: 'true'\n"),
    ("{\"k\":\"True\"}", "k: 'True'\n"),
    ("{\"k\":\"TRUE\"}", "k: 'TRUE'\n"),
    ("{\"k\":\"false\"}", "k: 'false'\n"),
    ("{\"k\":\"null\"}", "k: 'null'\n"),
    ("{\"k\":\"Null\"}", "k: 'Null'\n"),
    ("{\"k\":\"~\"}", "k: '~'\n"),
    ("{\"k\":\"\"}", "k: ''\n"),
    ("{\"k\":\"1\"}", "k: '1'\n"),
    ("{\"k\":\"-1\"}", "k: '-1'\n"),
    ("{\"k\":\"+1\"}", "k: '+1'\n"),
    ("{\"k\":\"1.0\"}", "k: '1.0'\n"),
    ("{\"k\":\"1.5e3\"}", "k: '1.5e3'\n"),
    ("{\"k\":\".5\"}", "k: '.5'\n"),
    ("{\"k\":\"1.\"}", "k: '1.'\n"),
    ("{\"k\":\"0x1F\"}", "k: '0x1F'\n"),
    ("{\"k\":\"0o17\"}", "k: '0o17'\n"),
    ("{\"k\":\"0b101\"}", "k: '0b101'\n"),
    ("{\"k\":\"007\"}", "k: '007'\n"),
    ("{\"k\":\"-007\"}", "k: '-007'\n"),
    ("{\"k\":\"00\"}", "k: '00'\n"),
    ("{\"k\":\".inf\"}", "k: '.inf'\n"),
    ("{\"k\":\"-.inf\"}", "k: '-.inf'\n"),
    ("{\"k\":\".nan\"}", "k: '.nan'\n"),
    ("{\"k\":\"inf\"}", "k: inf\n"),
    ("{\"k\":\"nan\"}", "k: nan\n"),
    ("{\"k\":\"yes\"}", "k: yes\n"),
    ("{\"k\":\"no\"}", "k: no\n"),
    ("{\"k\":\"on\"}", "k: on\n"),
    ("{\"k\":\"off\"}", "k: off\n"),
    ("{\"k\":\"y\"}", "k: y\n"),
    ("{\"k\":\"n\"}", "k: n\n"),
    ("{\"k\":\"-\"}", "k: '-'\n"),
    ("{\"k\":\"- a\"}", "k: '- a'\n"),
    ("{\"k\":\"-a\"}", "k: -a\n"),
    ("{\"k\":\"a: b\"}", "k: 'a: b'\n"),
    ("{\"k\":\"a:b\"}", "k: a:b\n"),
    ("{\"k\":\":x\"}", "k: :x\n"),
    ("{\"k\":\"x:\"}", "k: 'x:'\n"),
    ("{\"k\":\"? x\"}", "k: '? x'\n"),
    ("{\"k\":\"?x\"}", "k: ?x\n"),
    ("{\"k\":\"#x\"}", "k: '#x'\n"),
    ("{\"k\":\"a #b\"}", "k: 'a #b'\n"),
    ("{\"k\":\"a#b\"}", "k: a#b\n"),
    ("{\"k\":\"[a]\"}", "k: '[a]'\n"),
    ("{\"k\":\"a[b]\"}", "k: a[b]\n"),
    ("{\"k\":\"{a}\"}", "k: '{a}'\n"),
    ("{\"k\":\"a,b\"}", "k: a,b\n"),
    ("{\"k\":\"*a\"}", "k: '*a'\n"),
    ("{\"k\":\"&a\"}", "k: '&a'\n"),
    ("{\"k\":\"!a\"}", "k: '!a'\n"),
    ("{\"k\":\"|\"}", "k: '|'\n"),
    ("{\"k\":\"|x\"}", "k: '|x'\n"),
    ("{\"k\":\">\"}", "k: '>'\n"),
    ("{\"k\":\"'q'\"}", "k: '''q'''\n"),
    ("{\"k\":\"\\\"q\\\"\"}", "k: '\"q\"'\n"),
    ("{\"k\":\"%x\"}", "k: '%x'\n"),
    ("{\"k\":\"@x\"}", "k: '@x'\n"),
    ("{\"k\":\"`x\"}", "k: '`x'\n"),
    ("{\"k\":\" lead\"}", "k: ' lead'\n"),
    ("{\"k\":\"trail \"}", "k: 'trail '\n"),
    ("{\"k\":\" both \"}", "k: ' both '\n"),
    ("{\"k\":\"tab\\there\"}", "k: \"tab\\there\"\n"),
    ("{\"k\":\"\\ttab\"}", "k: \"\\ttab\"\n"),
    ("{\"k\":\"ctrl\\u0001\"}", "k: \"ctrl\\x01\"\n"),
    ("{\"k\":\"bell\\u0007\"}", "k: \"bell\\a\"\n"),
    ("{\"k\":\"del\u{7f}\"}", "k: \"del\\x7F\"\n"),
    ("{\"k\":\"c1\u{85}x\"}", "k: \"c1\\Nx\"\n"),
    ("{\"k\":\"nbsp x\"}", "k: nbsp x\n"),
    ("{\"k\":\" lead\"}", "k:  lead\n"),
    ("{\"k\":\"é中😀\"}", "k: é中😀\n"),
    ("{\"k\":\"bom\u{feff}x\"}", "k: \"bom\\uFEFFx\"\n"),
    ("{\"k\":\"a\\nb\"}", "k: |-\n  a\n  b\n"),
    ("{\"k\":\"a\\n\"}", "k: |\n  a\n"),
    ("{\"k\":\"a\\n\\n\"}", "k: |+\n  a\n\n"),
    ("{\"k\":\"\\n\"}", "k: |2+\n\n"),
    ("{\"k\":\"\\n\\n\"}", "k: |2+\n\n\n"),
    ("{\"k\":\"\\na\"}", "k: |2-\n\n  a\n"),
    ("{\"k\":\" a\\nb\"}", "k: |2-\n   a\n  b\n"),
    ("{\"k\":\"a \\nb\"}", "k: \"a \\nb\"\n"),
    ("{\"k\":\"a\\n b\"}", "k: |-\n  a\n   b\n"),
    ("{\"k\":\"a\\r\\nb\"}", "k: \"a\\r\\nb\"\n"),
    ("{\"k\":\"a\\rb\"}", "k: \"a\\rb\"\n"),
    ("{\"k\":\"---\"}", "k: '---'\n"),
    ("{\"k\":\"...\"}", "k: '...'\n"),
    ("{\"k\":\"--- x\"}", "k: '--- x'\n"),
    ("{\"k\":\"... x\"}", "k: '... x'\n"),
    ("{\"k\":\"12:30\"}", "k: 12:30\n"),
    (
        "{\"k\":\"http://example.org/a#b\"}",
        "k: http://example.org/a#b\n",
    ),
    ("{\"k\":\"a, b\"}", "k: a, b\n"),
    ("{\"k\":\"plain text here\"}", "k: plain text here\n"),
    ("{\"k\":\"snowman ☃\"}", "k: snowman ☃\n"),
    ("{\"k\":\"eé\"}", "k: eé\n"),
    ("{\"k\":\"\\\\back\"}", "k: \\back\n"),
    ("{\"k\":\"q'uote\"}", "k: q'uote\n"),
    ("{\"k\":\"d\\\"q\"}", "k: d\"q\n"),
    ("{\"k\":\"1_000\"}", "k: 1_000\n"),
    ("{\"k\":\"0x\"}", "k: 0x\n"),
    ("{\"k\":\"0xG\"}", "k: 0xG\n"),
    ("{\"k\":\"+.5\"}", "k: '+.5'\n"),
    ("{\"k\":\"-.5\"}", "k: '-.5'\n"),
    ("{\"k\":\"1e5\"}", "k: '1e5'\n"),
    ("{\"k\":\"1E+5\"}", "k: '1E+5'\n"),
    ("{\"k\":\"-0\"}", "k: '-0'\n"),
    ("{\"k\":\"0\"}", "k: '0'\n"),
    ("{\"k\":\"0.0\"}", "k: '0.0'\n"),
    ("{\"k\":\"12e\"}", "k: 12e\n"),
    ("{\"k\":\"NULL\"}", "k: 'NULL'\n"),
    ("{\"k\":\"~x\"}", "k: ~x\n"),
    ("{\"k\":\"x~\"}", "k: x~\n"),
    ("[\"true\"]", "- 'true'\n"),
    ("[\"\"]", "- ''\n"),
    ("[\"- a\"]", "- '- a'\n"),
    ("[\"#x\"]", "- '#x'\n"),
    ("[\" lead\"]", "- ' lead'\n"),
    ("[\"a\\nb\"]", "- |-\n  a\n  b\n"),
    ("[\"a\\n\\n\"]", "- |+\n  a\n\n"),
    ("[\"\\n\"]", "- |2+\n\n"),
    ("{\"true\":1}", "'true': 1\n"),
    ("{\"- a\":1}", "'- a': 1\n"),
    ("{\"a: b\":1}", "'a: b': 1\n"),
    ("{\"#x\":1}", "'#x': 1\n"),
    ("{\" lead\":1}", "' lead': 1\n"),
    ("{\"a\\nb\":1}", "? |-\n  a\n  b\n: 1\n"),
    ("{\"\\n\":1}", "? |2+\n\n: 1\n"),
    ("{}", "{}\n"),
    ("[]", "[]\n"),
    ("{\"a\":{}}", "a: {}\n"),
    ("{\"a\":[]}", "a: []\n"),
    ("[[]]", "- []\n"),
    ("[{}]", "- {}\n"),
    (
        "{\"a\":[1,2],\"b\":{\"c\":[3,{\"d\":4}]}}",
        "a:\n- 1\n- 2\nb:\n  c:\n  - 3\n  - d: 4\n",
    ),
    (
        "[[1,2],[3,[4,5]]]",
        "- - 1\n  - 2\n- - 3\n  - - 4\n    - 5\n",
    ),
    (
        "[{\"a\":1,\"b\":2},{\"c\":[1,2]}]",
        "- a: 1\n  b: 2\n- c:\n  - 1\n  - 2\n",
    ),
    ("{\"a\":[[1],{\"b\":2}]}", "a:\n- - 1\n- b: 2\n"),
    (
        "{\"a\":{\"b\":{\"c\":\"d\\ne\"}}}",
        "a:\n  b:\n    c: |-\n      d\n      e\n",
    ),
    (
        "[\"x\\ny\",{\"k\":\"l\\nm\\n\"}]",
        "- |-\n  x\n  y\n- k: |\n    l\n    m\n",
    ),
    (
        "{\"t\":true,\"f\":false,\"n\":null}",
        "t: true\nf: false\nn: null\n",
    ),
    ("1", "1\n"),
    ("-2", "-2\n"),
    ("1.5", "1.5\n"),
    ("0.1", "0.1\n"),
    ("true", "true\n"),
    ("null", "null\n"),
    (
        "{\"kkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkk\":1}",
        "? kkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkk\n: 1\n",
    ),
    (
        "{\"kkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkk\":1}",
        "kkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkk: 1\n",
    ),
    ("{\"multi\\nline\":1}", "? |-\n  multi\n  line\n: 1\n"),
    ("{\"a\":\"  indented\\nx\"}", "a: |2-\n    indented\n  x\n"),
    ("{\"a\":\"x\\n\\n\"}", "a: |+\n  x\n\n"),
    ("{\"a\":\"x\\n\\n\",\"b\":1}", "a: |+\n  x\n\nb: 1\n"),
    ("{\"a\":{\"b\":\"  lead\\n\"}}", "a:\n  b: |2\n      lead\n"),
    ("[[\"a\\nb\"]]", "- - |-\n    a\n    b\n"),
    ("{\"x\":[[{\"y\":\"z\\n\"}]]}", "x:\n- - y: |\n      z\n"),
    ("{\"\":1}", "'': 1\n"),
    ("{\"true\":\"true\"}", "'true': 'true'\n"),
    ("{\"a\":[\"b\",{\"c\":[\"d\"]}]}", "a:\n- b\n- c:\n  - d\n"),
    ("[[[]]]", "- - []\n"),
    ("{\"a\":[{}]}", "a:\n- {}\n"),
    ("{\"k\":\"t\\tb\\nnext\"}", "k: \"t\\tb\\nnext\"\n"),
    ("{\"k\":\"tail \\nnext\"}", "k: \"tail \\nnext\"\n"),
    ("{\"k\":\"a\\n b\\n\"}", "k: |\n  a\n   b\n"),
    ("{\"k\":\"\u{feff}\"}", "k: \"\\uFEFF\"\n"),
];

#[test]
fn the_emitter_writes_serde_yamls_bytes_for_every_captured_form() {
    for &(input, expected) in SERDE_YAML_BYTES {
        let value = json(input);
        assert_eq!(write(&value), expected, "{input}");
        assert_eq!(read(expected).expect(expected), value, "{expected:?}");
    }
}

/// The emitter's deliberate departures from `serde_yaml`, each still reading
/// back to the value written.
#[test]
fn the_documented_departures_from_serde_yaml_read_back() {
    for (input, written) in [
        // A number is its lexeme; serde_yaml wrote `1.5`, `1e20` and `1e-7`.
        (r#"{"n":1.50}"#, "n: 1.50\n"),
        (r"[1e+20,1e-07]", "- 1e+20\n- 1e-07\n"),
        // A line separator is double-quoted; serde_yaml wrote it raw inside a
        // single-quoted scalar and indented the text after it.
        ("\"x\\u2028y\"", "\"x\\Ly\"\n"),
        ("{\"k\":\"a\\u2029\"}", "k: \"a\\P\"\n"),
        // A float spelling beyond binary64 is quoted; serde_yaml left it plain
        // because its resolver read it as a string.
        (r#"["1e400"]"#, "- '1e400'\n"),
    ] {
        let value = json(input);
        assert_eq!(write(&value), written, "{input}");
        assert_eq!(read(written).unwrap(), value, "{written:?}");
    }
}

// ---- the reader: the block and flow grammar --------------------------------

#[test]
fn block_collections_nest_by_indentation() {
    let value =
        read("a:\n  b:\n    c: d\n  e: f\ng:\n- 1\n- - x\n  - y\n- k: v\n  l: w\nh: []\ni: {}\n")
            .unwrap();
    assert_eq!(
        json::write_compact(&value),
        r#"{"a":{"b":{"c":"d"},"e":"f"},"g":[1,["x","y"],{"k":"v","l":"w"}],"h":[],"i":{}}"#
    );
    assert_eq!(
        read("x:\n   - 1\n   - 2\n").unwrap(),
        json(r#"{"x":[1,2]}"#)
    );
    assert_eq!(read("- a\n-\n- c\n").unwrap(), json(r#"["a",null,"c"]"#));
    assert_eq!(read("a:\nb: ~\n").unwrap(), json(r#"{"a":null,"b":null}"#));
}

#[test]
fn flow_collections_read_across_lines() {
    assert_eq!(
        read("- [a,\n   b, [c, {d: e}]]\n- {f: g,\n   h, \"i\": [1, 2.50, ]}\n").unwrap(),
        json(r#"[["a","b",["c",{"d":"e"}]],{"f":"g","h":null,"i":[1,2.50]}]"#)
    );
    assert_eq!(read("[a: 1, b]").unwrap(), json(r#"[{"a":1},"b"]"#));
    assert_eq!(
        read("{a:b, \"c\":d}").unwrap(),
        json(r#"{"a:b":null,"c":"d"}"#)
    );
    assert_eq!(
        read(r#"{"json": [1, 2.50, "s", null, true], "nested": {"a": {}}}"#).unwrap(),
        json(r#"{"json":[1,2.50,"s",null,true],"nested":{"a":{}}}"#)
    );
}

#[test]
fn scalars_fold_escape_and_chomp_by_their_style() {
    let value = read(concat!(
        "plain: this is\n  a multi line\n\n  plain scalar\n",
        "single: 'it''s\n  folded'\n",
        "double: \"tab\\tnew\\nline \\u00e9 \\x41 \\U0001F600 \\\n   joined\"\n",
        "literal: |\n  one\n   two\n\n  three\n",
        "folded: >\n  a\n  b\n\n  c\n    d\n  e\n",
        "strip: |-\n  x\n\n",
        "keep: |+\n  y\n\n",
        "indicator: |2\n    two\n  zero\n",
        "comment: value # not content\n",
        "hash: a#b\n",
    ))
    .unwrap();
    assert_eq!(value["plain"], "this is a multi line\nplain scalar");
    assert_eq!(value["single"], "it's folded");
    assert_eq!(value["double"], "tab\tnew\nline é A 😀 joined");
    assert_eq!(value["literal"], "one\n two\n\nthree\n");
    assert_eq!(value["folded"], "a b\nc\n  d\ne\n");
    assert_eq!(value["strip"], "x");
    assert_eq!(value["keep"], "y\n\n");
    assert_eq!(value["indicator"], "  two\nzero\n");
    assert_eq!(value["comment"], "value");
    assert_eq!(value["hash"], "a#b");
}

#[test]
fn plain_scalars_resolve_by_the_core_schema_and_keep_json_lexemes() {
    let value = read(concat!(
        "n: [~, null, '', !!null '']\n",
        "b: [true, False, TRUE, yes, no, on]\n",
        "i: [0, -0, 7, +5, -3, 0x1F, 0o17, 0b101, 007, 12:30]\n",
        "f: [1.50, .5, -.5, 1., 1e3, 1.5E-3, +1.5]\n",
        "s: [inf, nan, 0x, 1_000, 'true', \"1\"]\n",
    ))
    .unwrap();
    assert_eq!(
        json::write_compact(&value),
        r#"{"n":[null,null,"",null],"b":[true,false,true,"yes","no","on"],"i":[0,-0,7,5,-3,31,15,5,"007","12:30"],"f":[1.50,0.5,-0.5,1.0,1e3,1.5E-3,1.5],"s":["inf","nan","0x","1_000","true","1"]}"#
    );
}

#[test]
fn explicit_keys_anchors_and_aliases_are_read() {
    assert_eq!(
        read("? a\n: 1\n? |-\n  multi\n  key\n: 2\n? bare\n").unwrap(),
        json(r#"{"a":1,"multi\nkey":2,"bare":null}"#)
    );
    assert_eq!(
        read("base: &b {x: 1, y: [2]}\ncopy: *b\n&k key: v\nref: *k\nseq: &s\n- 1\nagain: *s\n")
            .unwrap(),
        json(
            r#"{"base":{"x":1,"y":[2]},"copy":{"x":1,"y":[2]},"key":"v","ref":"key","seq":[1],"again":[1]}"#
        )
    );
}

#[test]
fn documents_markers_directives_and_comments_frame_one_node() {
    for (text, expected) in [
        ("", "null"),
        ("# only a comment\n", "null"),
        ("--- plain doc\n", "\"plain doc\""),
        ("%YAML 1.2\n---\na: 1\n...\n# trailing\n", r#"{"a":1}"#),
        ("\u{feff}a: 1", r#"{"a":1}"#),
        ("a: 1\r\nb:\r\n- 2\r\n", r#"{"a":1,"b":[2]}"#),
    ] {
        assert_eq!(read(text).unwrap(), json(expected), "{text:?}");
    }
}

// ---- refusals and their valid neighbours -----------------------------------

#[test]
fn a_non_core_tag_is_refused_and_every_core_tag_is_honoured() {
    for bad in [
        "a: !custom x\n",
        "a: !!binary aGk=\n",
        "a: !!int x\n",
        "a: !!seq x\n",
        "!!seq {a: 1}\n",
        "!<tag:example.org,2026:x> 1\n",
    ] {
        assert_eq!(kind(bad), ErrorKind::Tag, "{bad:?}");
    }
    assert_eq!(
        read("a: !!str 1\nb: !!int '7'\nc: !!float 2\nd: !!bool true\ne: !!null ~\nf: !!seq [1]\ng: !!map {x: y}\nh: ! 12\ni: !<tag:yaml.org,2002:str> 5\n!!str 9: k\n").unwrap(),
        json(r#"{"a":"1","b":7,"c":2,"d":true,"e":null,"f":[1],"g":{"x":"y"},"h":"12","i":"5","9":"k"}"#)
    );
}

#[test]
fn a_repeated_key_is_refused_and_distinct_keys_are_accepted() {
    for bad in [
        "a: 1\na: 2\n",
        "{a: 1, a: 2}",
        "a: 1\n\"a\": 2\n",
        "? a\n? a\n",
    ] {
        assert_eq!(kind(bad), ErrorKind::DuplicateKey, "{bad:?}");
    }
    assert_eq!(
        read("a: 1\nb: {a: 2}\n").unwrap(),
        json(r#"{"a":1,"b":{"a":2}}"#)
    );
}

#[test]
fn a_key_that_is_not_a_string_is_refused_and_its_quoted_spelling_accepted() {
    for bad in [
        "1: x\n",
        "true: x\n",
        "~: x\n",
        ": x\n",
        "[a]: x\n",
        "{a: b}: x\n",
        "{1: x}",
        "[1: x]",
    ] {
        assert_eq!(kind(bad), ErrorKind::NonStringKey, "{bad:?}");
    }
    assert_eq!(
        read("'1': x\n\"true\": y\n'~': z\n").unwrap(),
        json(r#"{"1":"x","true":"y","~":"z"}"#)
    );
}

#[test]
fn infinity_and_nan_are_refused_and_their_quoted_spellings_accepted() {
    for bad in ["a: .inf\n", "- -.Inf\n", "[.nan]"] {
        assert_eq!(kind(bad), ErrorKind::NonFinite, "{bad:?}");
    }
    assert_eq!(
        read("a: '.inf'\nb: inf\nc: 1e400\n").unwrap(),
        json(r#"{"a":".inf","b":"inf","c":1e400}"#)
    );
}

#[test]
fn a_second_document_is_refused_and_one_ended_document_is_accepted() {
    for bad in ["--- a\n--- b\n", "a: 1\n---\nb: 2\n", "a\n...\nb\n"] {
        assert_eq!(kind(bad), ErrorKind::MultipleDocuments, "{bad:?}");
    }
    assert_eq!(read("--- a\n...\n").unwrap(), "a");
}

#[test]
fn tab_indentation_is_refused_and_a_tab_separator_accepted() {
    assert_eq!(kind("a:\n\tb: 1\n"), ErrorKind::Tab);
    assert_eq!(kind("k: |\n\tx\n"), ErrorKind::Tab);
    assert_eq!(
        read("a:\tb\nc:\t[1,\t2]\n").unwrap(),
        json(r#"{"a":"b","c":[1,2]}"#)
    );
}

#[test]
fn malformed_structure_is_refused() {
    for bad in [
        "a: b\n  c: d\n",
        "a:\n - b\n c: d\n",
        "a: b: c\n",
        "key: - a\n",
        "[a, b\n",
        "{a: 1\n",
        "'unterminated\n",
        "a: \"bad \\q escape\"\n",
        "*unknown\n",
        "[a, , b]\n",
        "a: |0\n  x\n",
    ] {
        assert!(read(bad).is_err(), "{bad:?}");
    }
    assert_eq!(kind("*unknown\n"), ErrorKind::UnknownAlias);
    assert_eq!(kind("a: \"\\ud800\"\n"), ErrorKind::Escape);
}

#[test]
fn aliases_are_refused_when_off_and_read_when_on() {
    let off = Limits {
        aliases: false,
        ..Limits::DEFAULT
    };
    assert_eq!(
        read_with("a: &x 1\nb: *x\n", off).unwrap_err().kind(),
        ErrorKind::Alias
    );
    assert_eq!(
        read_with("a: 1\nb: '&x *x'\n", off).unwrap(),
        json(r#"{"a":1,"b":"&x *x"}"#)
    );
    assert_eq!(read("a: &x 1\nb: *x\n").unwrap(), json(r#"{"a":1,"b":1}"#));
}

#[test]
fn an_alias_bomb_is_refused_by_the_node_bound() {
    let mut bomb = String::from("a0: &a0 [x, x, x, x, x, x, x, x, x, x]\n");
    for level in 1..12 {
        let previous = format!("*a{}", level - 1);
        let _ = writeln!(
            bomb,
            "a{level}: &a{level} [{}]",
            vec![previous; 10].join(", ")
        );
    }
    assert!(matches!(kind(&bomb), ErrorKind::Nodes { .. }));
    let small = "a: &a [x, x]\nb: [*a, *a]\n";
    assert!(
        read_with(
            small,
            Limits {
                max_nodes: 13,
                ..Limits::DEFAULT
            }
        )
        .is_ok()
    );
    assert!(
        read_with(
            small,
            Limits {
                max_nodes: 12,
                ..Limits::DEFAULT
            }
        )
        .is_err()
    );
}

#[test]
fn the_depth_cap_accepts_n_collections_and_refuses_n_plus_one() {
    for cap in [1_usize, 2, 16] {
        let limits = Limits {
            max_depth: cap,
            ..Limits::DEFAULT
        };
        let flow = |depth: usize| format!("{}1{}", "[".repeat(depth), "]".repeat(depth));
        let block = |depth: usize| format!("{}1\n", "- ".repeat(depth));
        for build in [flow, block] {
            assert!(read_with(&build(cap), limits).is_ok(), "{:?}", build(cap));
            assert_eq!(
                read_with(&build(cap + 1), limits).unwrap_err().kind(),
                ErrorKind::Depth { limit: cap }
            );
        }
    }
}

// ---- both directions ---------------------------------------------------------

/// A fixed-seed generator (SplitMix64), so every run draws the same values.
struct SplitMix(u64);

impl SplitMix {
    const fn next(&mut self) -> u64 {
        purrdf_testkit::rng::splitmix64_next(&mut self.0)
    }

    fn below(&mut self, n: u64) -> usize {
        (self.next() % n) as usize
    }
}

const TEXTS: &[&str] = &[
    "",
    "a",
    "true",
    "null",
    "~",
    "1",
    "-1.5",
    "007",
    "0x1F",
    " lead",
    "trail ",
    "a: b",
    "- x",
    "# c",
    "x #y",
    "[a]",
    "{b}",
    "'q'",
    "\"d\"",
    "\\",
    "é中😀",
    "tab\there",
    "ctl\u{1}",
    "a\nb",
    "a\n",
    "a\n\n",
    "\n",
    " a\nb",
    "a \nb",
    "a\n b",
    "cr\rlf",
    "\u{85}",
    "\u{a0}",
    "\u{feff}",
    "x\u{2028}y",
    "---",
    "...",
    "? k",
    ": v",
    "%x",
    "@x",
    "`x",
    "*x",
    "&x",
    "!x",
    "|",
    ">",
];

fn arbitrary(rng: &mut SplitMix, depth: usize) -> Value {
    let leaf = depth == 0 || rng.below(3) == 0;
    match rng.below(if leaf { 5 } else { 7 }) {
        0 => Value::Null,
        1 => Value::Bool(rng.below(2) == 0),
        2 => Value::from(["0", "-3", "1.50", "2e+9", "123456789012345678901234"][rng.below(5)])
            .as_str()
            .map(|lexeme| Value::Number(lexeme.parse().expect("a lexeme")))
            .expect("a string"),
        3 | 4 => Value::from(TEXTS[rng.below(TEXTS.len() as u64)]),
        5 => Value::Array(
            (0..rng.below(4))
                .map(|_| arbitrary(rng, depth - 1))
                .collect(),
        ),
        _ => {
            let mut object = Object::new();
            for _ in 0..rng.below(4) {
                let name =
                    TEXTS[rng.below(TEXTS.len() as u64)].to_owned() + &rng.below(3).to_string();
                object.insert(name, arbitrary(rng, depth - 1));
            }
            Value::Object(object)
        }
    }
}

/// Whatever the emitter writes, the reader reads back to the same value.
#[test]
fn every_written_document_reads_back_to_its_value() {
    let mut rng = SplitMix(0x7A31_2026_0000_0001);
    for _ in 0..3000 {
        let value = arbitrary(&mut rng, 4);
        let written = write(&value);
        assert_eq!(
            read(&written).unwrap_or_else(|e| panic!("{e}: {written:?}")),
            value,
            "{written:?}"
        );
    }
    let long_key = "k".repeat(200);
    let value = Value::object([
        (long_key.as_str(), Value::from(1_u8)),
        ("m\nn", Value::from("x")),
    ]);
    let written = write(&value);
    assert!(written.starts_with("? "));
    assert_eq!(read(&written).unwrap(), value);
}

#[test]
fn a_hundred_thousand_deep_document_is_written_and_read_without_overflow() {
    purrdf_stack::on_stack(256 * 1024, || {
        const DEPTH: usize = 100_000;
        let unbounded = Limits {
            max_depth: usize::MAX,
            ..Limits::DEFAULT
        };
        let text = format!("{}0{}", "[".repeat(DEPTH), "]".repeat(DEPTH));
        let value = read_with(&text, unbounded).unwrap();
        let written = write(&value);
        assert_eq!(written, format!("{}0\n", "- ".repeat(DEPTH)));
        assert_eq!(read_with(&written, unbounded).unwrap(), value);
    })
    .expect("a 256 KiB stack thread runs the walk");
}
