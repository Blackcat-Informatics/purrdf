// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use super::*;

#[test]
fn deeply_nested_groups_fail_as_a_resource_error() {
    let at_limit = format!("{}x{}", "(".repeat(250), ")".repeat(250));
    assert!(compile(&at_limit).is_ok());
    let over_limit = format!("{}x{}", "(".repeat(251), ")".repeat(251));
    assert!(matches!(
        parse(&over_limit),
        Err(PatternError::Resource { .. })
    ));
    assert!(matches!(
        compile(&over_limit),
        Err(PatternError::Resource { .. })
    ));
    assert!(matches!(
        is_valid_syntax(&over_limit),
        Err(PatternError::Resource { .. })
    ));
    assert!(matches!(
        compile("a{4294967296}"),
        Err(PatternError::Resource { .. })
    ));
}

#[test]
fn valid_pattern_exceeding_regular_engine_size_uses_bounded_vm() {
    let pattern = compile("a{100000000}").expect("valid ECMA repetition");
    assert!(matches!(pattern.backend, Backend::Vm(..)));
    let result = pattern.is_match(
        "a",
        &mut MatchLimits {
            steps: 100,
            states: 100,
        },
    );
    assert!(matches!(
        result,
        Ok(false) | Err(PatternError::Resource { .. })
    ));
}

#[test]
fn vm_limits_count_nested_assertions_and_pending_alternatives() {
    let pattern = compile(r"(?=a)(a|aa)*b").expect("valid pattern");
    assert!(matches!(
        pattern.is_match(
            "aaaa",
            &mut MatchLimits {
                steps: 10_000,
                states: 2
            }
        ),
        Err(PatternError::Resource { .. })
    ));
    assert!(matches!(
        pattern.is_match(
            "aaaa",
            &mut MatchLimits {
                steps: 2,
                states: 100
            }
        ),
        Err(PatternError::Resource { .. })
    ));
}

#[test]
fn vm_rejects_large_pending_program_before_copying_it() {
    let ast = Ast::Concat(vec![Ast::Char(u32::from('a')); 100_001]);
    assert!(matches!(
        vm::is_match(&ast, "a", &mut MatchLimits::default(), 0),
        Err(PatternError::Resource { .. })
    ));
    let pattern = compile("(?=a)a").expect("valid assertion");
    assert_eq!(
        pattern.is_match(
            "a",
            &mut MatchLimits {
                steps: 50,
                states: 1
            }
        ),
        Ok(true)
    );
    let safe_scan = compile("(?=a)b").expect("valid assertion");
    assert_eq!(
        safe_scan.is_match(&"x".repeat(100_001), &mut MatchLimits::default()),
        Ok(false)
    );
}

fn matches(pattern: &str, input: &str) -> bool {
    compile(pattern)
        .expect("pattern compiles")
        .is_match(input, &mut MatchLimits::default())
        .expect("within budget")
}

#[test]
fn nonregular_constructs_execute_in_ecma_order() {
    assert!(matches(r"^(?=ab)a", "ab"));
    assert!(!matches(r"^(?!ab)a", "ab"));
    assert!(matches(r"(?<=a)b", "ab"));
    assert!(!matches(r"(?<!a)b", "ab"));
    assert!(matches(r"^(a)\1$", "aa"));
    assert!(matches(r"^(?<n>a)\k<n>$", "aa"));
    assert!(matches(r"^(?i:a)b$", "Ab"));
    assert!(!matches(r"^(?i:a)b$", "AB"));
}

#[test]
fn node_26_nonregular_oracle_cases() {
    // Independently captured with `new RegExp(pattern, 'u').test(input)` on Node 26.10.0.
    for (pattern, input, expected) in [
        (r"^(a)?\1$", "", true),
        (r"^(a)?\1$", "a", false),
        (r"^(?=(a|ab))\1b$", "abb", false),
        (r"^(?=(a|ab))\1b$", "ab", true),
        (r"(?<=a{1,3})b", "aaab", true),
        (r"^(a|ab)\1$", "abab", true),
        (r"^(a*)\1$", "aaaa", true),
        (r"^(?i:a)b$", "Ab", true),
        (r"^(?i:a)b$", "AB", false),
        (r"^(?s:.)$", "\n", true),
        (r"(?<=(a))b\1", "aba", true),
        (r"^(a?)*b$", "b", true),
        (r"^(a?)*b$", "ab", true),
        (r"^(a(b)?)+\2$", "aba", true),
        (r"^(a(b)?)+\2$", "abab", false),
        (r"^(?i:k)$", "\u{212A}", true),
        (r"^(?i:s)$", "ſ", true),
        (r"^(?i:\w)$", "\u{212A}", true),
        (r"^(?i:(k)\1)$", "k\u{212A}", true),
        (r"^(?i:(k))\1$", "k\u{212A}", false),
        (r"^(?:(?<n>a)|(?<n>b))\k<n>$", "aa", true),
        (r"^(?:(?<n>a)|(?<n>b))\k<n>$", "bb", true),
        (r"^(?:(?<n>a)|(?<n>b))\k<n>$", "ab", false),
        (r"^(?i:\p{Uppercase_Letter})$", "\u{16EBB}", true),
        (r"(?<=([ab]+))c\1", "abcab", true),
        (r"(?<=([ab]+))c\1", "abca", false),
    ] {
        assert_eq!(
            matches(pattern, input),
            expected,
            "{pattern:?} on {input:?}"
        );
    }
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
fn unicode_17_properties_and_identifier_names_execute_from_vendored_ucd() {
    let beria = "\u{16EA0}";
    assert!(matches(r"^\p{Script=Beria_Erfe}$", beria));
    assert!(matches(r"^\p{Letter}$", beria));
    assert!(matches(r"^\p{ID_Start}$", beria));
    assert!(matches(r"^\p{Script_Extensions=Latin}$", "·"));
    assert!(!matches(r"^\p{Script=Latin}$", "·"));
    let named = format!("^(?<{beria}>a)\\k<{beria}>$");
    assert!(matches(&named, "aa"));
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
fn nonregular_constructs_are_valid_syntax_and_compile() {
    for pattern in [
        "(?=a)",
        "(?!a)",
        "(?<=a)",
        "(?<!a)",
        r"(a)\1",
        r"(?<n>a)\k<n>",
        "(?i:a)",
    ] {
        assert_eq!(is_valid_syntax(pattern), Ok(true));
        assert!(compile(pattern).is_ok(), "{pattern:?}");
    }
}

#[test]
fn empty_classes_and_alternatives_translate() {
    assert!(!matches("[]", "a"));
    assert!(matches("^[^]$", "a"));
    assert!(matches("^(?:a|)$", ""));
    assert!(matches("^a{2}$", "aa"));
    assert!(!matches("^a{2}$", "aaa"));
}
