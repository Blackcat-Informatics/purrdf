// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use super::*;

fn matches(pattern: &str, input: &str) -> bool {
    compile(pattern).expect("pattern compiles").is_match(input)
}

#[test]
fn class_escapes_are_the_ecma_sets_not_the_unicode_ones() {
    assert!(matches(r"^\d$", "7"));
    assert!(!matches(r"^\d$", "\u{07C0}"));
    assert!(matches(r"^\D$", "\u{07C0}"));
    assert!(matches(r"^\w$", "_"));
    assert!(!matches(r"^\w$", "é"));
    for space in [
        "\t", "\u{0B}", "\u{0C}", " ", "\u{A0}", "\u{FEFF}", "\n", "\u{2029}", "\u{2003}",
    ] {
        assert!(matches(r"^\s$", space), "{space:?}");
    }
    assert!(!matches(r"^\s$", "\u{0001}"));
    assert!(!matches(r"^\s$", "\u{2013}"));
    assert!(!matches(r"^\s$", "\u{180E}"));
}

#[test]
fn anchors_and_dot_follow_ecma() {
    assert!(!matches("^abc$", "abc\n"));
    assert!(matches("^abc$", "abc"));
    assert!(!matches("^.$", "\n"));
    assert!(!matches("^.$", "\u{2028}"));
    assert!(matches("^.$", "\u{85}"));
    assert!(matches(r"\bfoo\b", "a foo."));
    assert!(!matches(r"\bfoo\b", "afoo"));
    // `é` is not an ECMA-262 word character, so no boundary follows it.
    assert!(!matches(r"é\b", "é"));
    assert!(matches(r"e\b", "é e"));
}

#[test]
fn escapes_decode_to_their_code_points() {
    assert!(matches(r"^\t$", "\t"));
    assert!(matches(r"^\cC$", "\u{3}"));
    assert!(matches(r"^\cc$", "\u{3}"));
    assert!(matches(r"^\x41B\u{43}$", "ABC"));
    assert!(matches(r"^🐲$", "\u{1F432}"));
    assert!(!matches(r"\uD83D", "\u{1F432}"));
    assert!(matches(r"^[\uD800-￿]$", "\u{E000}"));
    assert!(matches(r"^[^\uD800]$", "x"));
    assert!(matches(r"^\0$", "\0"));
}

#[test]
fn properties_accept_exact_aliases_only() {
    assert!(matches(r"^\p{Letter}+$", "éa"));
    assert!(matches(r"^\p{L}$", "é"));
    assert!(matches(r"^\p{digit}+$", "\u{9EA}\u{9E8}"));
    assert!(matches(r"^\p{Script=Greek}$", "α"));
    assert!(matches(r"^\p{sc=Grek}$", "α"));
    assert!(matches(r"^\P{Any}|x$", "x"));
    assert!(matches(r"^\p{ASCII}$", "~"));
    assert!(parse(r"\p{letter}").is_err());
    assert!(parse(r"\p{Is_Latin}").is_err());
    assert!(parse(r"\p{Script=latin}").is_err());
    assert!(parse(r"\p{Block=Basic_Latin}").is_err());
}

#[test]
fn annex_b_leniencies_are_syntax_errors_under_the_u_flag() {
    for invalid in [
        r"\a",
        "]",
        "{",
        "}",
        "a{",
        "a{,2}",
        r"\-",
        r"\1",
        r"[\1]",
        r"\c1",
        "(",
        "a)",
        "*",
        "a**",
        "^*",
        r"\k<a>",
        "[b-a]",
        r"[\d-z]",
        r"\x4",
        r"\u{110000}",
        "(?<a>x)(?<a>y)",
        "(?i)x",
    ] {
        assert!(
            matches!(parse(invalid), Err(PatternError::Syntax { .. })),
            "{invalid:?} must be a syntax error"
        );
    }
    for valid in [
        "[]",
        "[^]",
        "a{2,3}?",
        "(?<a>x)|(?<a>y)",
        r"[\-\b]",
        "[a-]",
        r"\/",
        "(?:)",
        "a|",
        "(?=x)",
    ] {
        assert!(parse(valid).is_ok(), "{valid:?} must parse");
    }
}

#[test]
fn unsupported_constructs_are_typed_refusals() {
    for (pattern, construct) in [
        ("(?=a)", "lookahead"),
        ("(?!a)", "lookahead"),
        ("(?<=a)", "lookbehind"),
        ("(?<!a)", "lookbehind"),
        (r"(a)\1", "backreference"),
        (r"(?<n>a)\k<n>", "backreference"),
        ("(?i:a)", "modifier group"),
    ] {
        match compile(pattern) {
            Err(PatternError::Unsupported { construct: got, .. }) => assert_eq!(got, construct),
            other => panic!("{pattern:?}: {other:?}"),
        }
        assert!(is_valid_syntax(pattern));
    }
    assert!(compile("(a)").is_ok());
}

#[test]
fn empty_classes_and_alternatives_translate() {
    assert!(!matches("[]", "a"));
    assert!(matches("^[^]$", "a"));
    assert!(matches("^(?:a|)$", ""));
    assert!(matches("^a{2}$", "aa"));
    assert!(!matches("^a{2}$", "aaa"));
}
