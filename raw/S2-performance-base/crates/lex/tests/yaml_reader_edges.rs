// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The defects the yaml-test-suite exposed in the reader (`yaml_test_suite.rs`
//! grades it wholesale), each pinned with the input it refuses and the
//! neighbouring input that must still be read: a refusal is a claim, so every
//! tightened rule is shown to leave its valid neighbour alone.

use purrdf_lex::json::write_compact;
use purrdf_lex::yaml::{ErrorKind, read};

fn ok(text: &str) -> String {
    match read(text) {
        Ok(value) => write_compact(&value),
        Err(error) => panic!("{text:?} refused: {error}"),
    }
}

fn refused(text: &str) -> ErrorKind {
    match read(text) {
        Ok(value) => panic!("{text:?} accepted as {}", write_compact(&value)),
        Err(error) => error.kind(),
    }
}

fn syntax(text: &str) {
    assert!(
        matches!(refused(text), ErrorKind::Syntax(_)),
        "{text:?}: {:?}",
        refused(text)
    );
}

#[test]
fn a_tab_is_separation_before_a_flow_node_or_scalar_and_never_block_indentation() {
    assert_eq!(ok("\t[a]\n"), r#"["a"]"#);
    assert_eq!(ok("\t{}\n"), "{}");
    assert_eq!(ok("foo:\n \tbar\n"), r#"{"foo":"bar"}"#);
    assert_eq!(ok("- \tbar\n"), r#"["bar"]"#);
    assert_eq!(refused("\tk: v\n"), ErrorKind::Tab);
    assert_eq!(refused("k:\n\t- x\n"), ErrorKind::Tab);
    assert_eq!(refused("- a\n\t- b\n"), ErrorKind::Tab);
    // After a block indicator the next block structure is indented by spaces.
    assert_eq!(ok("-\tx\n"), r#"["x"]"#);
    assert_eq!(ok("- - x\n"), r#"[["x"]]"#);
    assert_eq!(refused("-\t-\n"), ErrorKind::Tab);
    assert_eq!(refused("- \t-\n"), ErrorKind::Tab);
    assert_eq!(refused("- \tk: v\n"), ErrorKind::Tab);
}

#[test]
fn a_tab_in_a_block_scalar_is_content_once_the_indentation_is_there() {
    assert_eq!(ok("foo: |-\n \tbar\n"), r#"{"foo":"\tbar"}"#);
    assert_eq!(ok("foo: |-\n \tbar"), r#"{"foo":"\tbar"}"#);
    assert_eq!(ok("foo: |\n \t\nbar: 1\n"), r#"{"foo":"\t\n","bar":1}"#);
    assert_eq!(ok("- >\n \t\n detected\n"), r#"["\t\ndetected\n"]"#);
    // Short of the indentation a tab stands where a space must.
    assert_eq!(refused("foo: |\n\t\nbar: 1\n"), ErrorKind::Tab);
    assert_eq!(refused("a: |\n  x\n \ty\n"), ErrorKind::Tab);
}

#[test]
fn a_top_level_block_scalar_may_be_zero_indented_and_ends_at_a_marker() {
    assert_eq!(
        ok("--- >\nline1\nline2\nline3\n"),
        r#""line1 line2 line3\n""#
    );
    assert_eq!(
        ok("--- >\nline1\n# no comment\nline3\n"),
        r#""line1 # no comment line3\n""#
    );
    assert_eq!(ok("--- |\nx\n...\n"), r#""x\n""#);
    // Nested, the content must still be indented past its owner.
    assert_eq!(ok("k: |\n x\n"), r#"{"k":"x\n"}"#);
    assert_eq!(ok("k: |\nx: 1\n"), r#"{"k":"","x":1}"#);
}

#[test]
fn a_block_scalar_ending_the_stream_ends_its_last_line() {
    assert_eq!(ok("foo: |\n  x\n   "), r#"{"foo":"x\n \n"}"#);
    assert_eq!(ok("- |+\n   "), r#"["\n"]"#);
    // A last line with text is not closed by the end of the stream.
    assert_eq!(ok("foo: |\n  x"), r#"{"foo":"x"}"#);
    assert_eq!(ok("foo: |-\n  x"), r#"{"foo":"x"}"#);
    assert_eq!(ok("foo: |\n  x\n"), r#"{"foo":"x\n"}"#);
}

#[test]
fn an_empty_line_may_not_hold_more_spaces_than_the_first_content_line() {
    syntax("a: >\n \n  \n   \n # comment\n");
    // Fewer or equal spaces on the empty lines are fine.
    assert_eq!(ok("a: >\n \n  text\n"), r#"{"a":"\ntext\n"}"#);
    assert_eq!(ok("a: |\n  \n  text\n"), r#"{"a":"\ntext\n"}"#);
}

#[test]
fn a_quoted_scalar_continues_on_lines_indented_past_its_block_and_holds_no_marker() {
    syntax("quoted: \"a\nb\nc\"\n");
    syntax("quoted: 'a\nb'\n");
    assert_eq!(ok("quoted: \"a\n b\n c\"\n"), r#"{"quoted":"a b c"}"#);
    assert_eq!(ok("\"a\nb\"\n"), r#""a b""#);
    assert_eq!(ok("- 'a\n b'\n"), r#"["a b"]"#);
    syntax("---\n\"\n---\n\"\n");
    syntax("---\n'\n...\n'\n");
    syntax("--- \"a\n... x\nb\"\n");
    assert_eq!(ok("--- \"a\n...x\nb\"\n"), r#""a ...x b""#);
    assert_eq!(ok("\"a\n---x\nb\"\n"), r#""a ---x b""#);
    syntax("foo: \"bar\n\tbaz\"\n");
}

#[test]
fn a_flow_collection_continues_on_lines_indented_past_its_block() {
    syntax("flow: [a,\nb,\nc]\n");
    syntax("k: {\nk\n:\nv\n}\n");
    syntax("- [\n\tfoo,\n foo\n ]\n");
    assert_eq!(ok("flow: [a,\n b,\n c]\n"), r#"{"flow":["a","b","c"]}"#);
    assert_eq!(ok("[a,\nb]\n"), r#"["a","b"]"#);
    assert_eq!(ok("k: {\n k\n : v\n }\n"), r#"{"k":{"k":"v"}}"#);
    // A document marker inside a flow collection ends nothing: it is refused.
    syntax("[\n--- ,\n...\n]\n");
    assert_eq!(ok("[\n  a,\n  b\n]\n"), r#"["a","b"]"#);
}

#[test]
fn an_implicit_key_of_a_flow_sequence_pair_is_on_one_line() {
    syntax("---\n[ key\n  : value ]\n");
    syntax("[ \"key\"\n  :value ]\n");
    assert_eq!(ok("[ key: value ]\n"), r#"[{"key":"value"}]"#);
    assert_eq!(ok("[ \"key\":value ]\n"), r#"[{"key":"value"}]"#);
    // An explicit key may span lines, and a flow mapping's implicit key may.
    assert_eq!(ok("[ ? key\n  : value ]\n"), r#"[{"key":"value"}]"#);
    assert_eq!(ok("{ key\n  : value }\n"), r#"{"key":"value"}"#);
}

#[test]
fn a_dash_before_a_flow_indicator_is_not_a_plain_scalar() {
    syntax("[-]\n");
    syntax("---\n- [-, -]\n");
    assert_eq!(ok("[-a, a-b, -1]\n"), r#"["-a","a-b",-1]"#);
    assert_eq!(ok("- [a, b]\n"), r#"[["a","b"]]"#);
}

#[test]
fn a_flow_entry_of_only_properties_or_only_an_indicator_is_an_empty_node_not_nothing() {
    assert_eq!(
        ok("{\n  foo : !!str,\n  !!str : bar,\n}\n"),
        r#"{"foo":"","":"bar"}"#
    );
    assert_eq!(ok("[!!str , a]\n"), r#"["","a"]"#);
    assert_eq!(ok("[!!null , a]\n"), r#"[null,"a"]"#);
    assert_eq!(ok("{a: &x , b: *x}\n"), r#"{"a":null,"b":null}"#);
    // An empty explicit key is a null key, which is not a string; it is not dropped.
    assert_eq!(
        refused("{\n? explicit: entry,\nimplicit: entry,\n?\n}\n"),
        ErrorKind::NonStringKey
    );
    assert_eq!(refused("{ ? }\n"), ErrorKind::NonStringKey);
    assert_eq!(
        ok("{\n? explicit: entry,\nimplicit: entry,\n? last\n}\n"),
        r#"{"explicit":"entry","implicit":"entry","last":null}"#
    );
}

#[test]
fn directives_are_checked() {
    syntax("%YAML 1.2 foo\n---\n");
    syntax("%YAML 1.1#...\n---\n");
    syntax("%YAML 1.2\n%YAML 1.2\n---\n");
    syntax("%YAML 2.0\n---\n");
    syntax("%YAML\n---\n");
    syntax("%TAG !e! tag:a,2000:\n%TAG !e! tag:b,2000:\n---\n");
    syntax("%TAG !e!\n---\n");
    assert_eq!(ok("%YAML 1.2\n---\na\n"), r#""a""#);
    assert_eq!(ok("%YAML  1.1\n---\na\n"), r#""a""#);
    assert_eq!(ok("%YAML \t 1.1\n---\na\n"), r#""a""#);
    assert_eq!(ok("%YAML 1.1  # comment\n---\na\n"), r#""a""#);
    assert_eq!(ok("%YAM 1.1\n---\na\n"), r#""a""#);
    assert_eq!(ok("%YAMLL 1.1\n---\na\n"), r#""a""#);
    assert_eq!(
        ok("%TAG !e! tag:a,2000:\n%TAG !f! tag:b,2000:\n---\na\n"),
        r#""a""#
    );
    assert_eq!(ok("%FOO bar baz\n---\na\n"), r#""a""#);
}

#[test]
fn an_anchor_may_not_precede_a_block_sequence_entry_on_its_line() {
    syntax("&anchor - sequence entry\n");
    syntax("- &a - x\n");
    assert_eq!(ok("&anchor\n- x\n"), r#"["x"]"#);
    assert_eq!(ok("k: &a\n- x\nr: *a\n"), r#"{"k":["x"],"r":["x"]}"#);
    assert_eq!(ok("- &a x\n- *a\n"), r#"["x","x"]"#);
}

#[test]
fn properties_on_several_lines_split_between_a_mapping_and_its_first_key() {
    assert_eq!(
        ok("top: &m\n  &k key: v\nrk: *k\nrm: *m\n"),
        r#"{"top":{"key":"v"},"rk":"key","rm":{"key":"v"}}"#
    );
    assert_eq!(
        ok("---\ntop4:\n  &node4\n  &k4 key4: four\ntop6: &val6\n  six\ntop7:\n  &val7 seven\n"),
        r#"{"top4":{"key4":"four"},"top6":"six","top7":"seven"}"#
    );
    assert_eq!(
        ok("a: &x k\ntop: &n\n  *x : v\nb: *n\n"),
        r#"{"a":"k","top":{"k":"v"},"b":{"k":"v"}}"#
    );
    // One node takes one anchor and one tag, wherever they are written.
    assert_eq!(ok("&a\n!!str x\n"), r#""x""#);
    assert_eq!(ok("!!str\n&a x\n"), r#""x""#);
    syntax("&a\n&b x\n");
    syntax("&a &b x\n");
    syntax("!!str\n!!str x\n");
    // An alias carries no properties of its own.
    syntax("a: &x 1\n&y *x\n");
    syntax("a: &x 1\nb: &y\n  *x\n");
    // Two property sets before a flow key are a mapping's and a key's; JSON
    // holds no such key, and the refusal names it so.
    assert_eq!(
        refused("&mapping\n&key [ &item a, b, c ]: value\n"),
        ErrorKind::NonStringKey
    );
}

#[test]
fn a_stream_with_no_document_reads_as_null() {
    for empty in [
        "",
        "\n",
        "# only a comment\n",
        "  # c\n   \n\n",
        "---\n",
        "...\n",
        "# c\n...\n",
    ] {
        assert_eq!(ok(empty), "null", "{empty:?}");
    }
}
