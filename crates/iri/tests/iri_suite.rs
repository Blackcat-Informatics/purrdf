// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Parse/validate + normalize + CURIE conformance.
//!
//! Vectors are curated from RFC-3987 §3.1 (IRI examples), RFC-3986 §1.1.2 / §3
//! (URI examples and component grammar), RFC-3986 §6.2.2 (normalization), and the
//! W3C `rdf-tests` IRI handling cases. The CURIE cases pin the semantics this crate
//! subsumes from the SSSOM serializer (`sssom::curie_prefix`/`resolve_iri`).

use purrdf_iri::{PrefixMap, contract, curie_prefix, expand_curie, parse, parse_uri, resolve};

#[test]
fn parses_valid_iris_and_splits_components() {
    let iri = parse("http://user@example.com:8080/over/there?name=ferret#nose").expect("valid IRI");
    assert_eq!(iri.scheme(), Some("http"));
    assert_eq!(iri.authority(), Some("user@example.com:8080"));
    assert_eq!(iri.path(), "/over/there");
    assert_eq!(iri.query(), Some("name=ferret"));
    assert_eq!(iri.fragment(), Some("nose"));
    assert!(iri.has_scheme());
}

#[test]
fn parses_assorted_valid_forms() {
    // urn, mailto (no authority), empty authority, IPv6 literal, percent-encoding.
    let valid = [
        "urn:example:animal:ferret:nose",
        "mailto:John.Doe@example.com",
        "file:///etc/hosts",
        "ldap://[2001:db8::7]/c=GB?objectClass?one",
        "http://example.com/a%20b",
        "tel:+1-816-555-1212",
        "foo://example.com:8042/over/there?name=ferret#nose",
        "a:b",
    ];
    for s in valid {
        assert!(parse(s).is_ok(), "expected {s:?} to parse");
    }
}

#[test]
fn parses_relative_references() {
    let r = parse("../g;x?y#s").expect("relative ref parses");
    assert_eq!(r.scheme(), None);
    assert_eq!(r.path(), "../g;x");
    assert_eq!(r.query(), Some("y"));
    assert_eq!(r.fragment(), Some("s"));
    assert!(!r.has_scheme());
}

#[test]
fn accepts_non_ascii_in_iri_mode_only() {
    // RFC-3987 ucschar is valid in an IRI...
    let iri = parse("http://例え.テスト/ぱす").expect("IRI permits ucschar");
    assert_eq!(iri.path(), "/ぱす");
    // ...but the same string is NOT a valid (ASCII-only) URI.
    assert!(parse_uri("http://例え.テスト/ぱす").is_err());
    // A pure-ASCII string parses in both modes.
    assert!(parse_uri("http://example.test/path").is_ok());
}

#[test]
fn rejects_malformed_input() {
    // Empty.
    assert!(parse("").is_err());
    // Truncated percent-encoding.
    assert!(parse("http://a/%2").is_err());
    assert!(parse("http://a/%zz").is_err());
    // Disallowed raw characters in a URI path (space, backtick, caret).
    assert!(parse_uri("http://a/foo bar").is_err());
    assert!(parse("http://a/foo`bar").is_err());
    // Unterminated IPv6 literal.
    assert!(parse("http://[2001:db8/path").is_err());
}

#[test]
fn rejects_colon_in_first_segment_of_relative_path() {
    // RFC-3986 §4.2 path-noscheme: a scheme-less, authority-less reference must
    // not carry a ':' in its first path segment (it would be ambiguous with a
    // scheme). `3com`/`+a`/`_x` are not valid scheme starts, so these are NOT
    // schemes — they are relative refs and must be rejected.
    assert!(parse("3com:x").is_err());
    assert!(parse("+a:b").is_err());
    assert!(parse("_x:y").is_err());
    // A ':' in a LATER segment is fine.
    assert!(parse("foo/bar:baz").is_ok());
    // A genuine scheme (`a:b`) is still accepted (ALPHA-led, so it IS a scheme).
    assert!(parse("a:b").is_ok());
}

#[test]
fn generic_port_syntax_is_any_ascii_digit_string() {
    // RFC 3986 §3.2.3 specifies *DIGIT, not a transport endpoint's u16 range.
    for port in [
        "",
        "0",
        "8080",
        "65535",
        "65536",
        "99999",
        "000000000000000000000000000000000000000001",
        "999999999999999999999999999999999999999999",
    ] {
        let text = format!("http://h:{port}/p");
        assert_eq!(parse(&text).expect("generic port syntax").as_str(), text);
    }
    for port in ["-1", "+1", "80x", "８０", "1:2", "%38%30"] {
        assert!(parse(&format!("http://h:{port}/p")).is_err(), "{port:?}");
    }
}

#[test]
fn normalize_case_and_pct_and_dots() {
    // §6.2.2.1 case: scheme + host lower-cased. §6.2.2.2: %-hex upper-cased and
    // unreserved %-encodings decoded. §6.2.2.3: dot segments removed.
    let n = parse("HTTP://Example.COM/a/./b/../c/%7euser?Q=%2d")
        .expect("parses")
        .normalize();
    assert_eq!(n.as_str(), "http://example.com/a/c/~user?Q=-");
}

#[test]
fn normalize_is_idempotent() {
    let n1 = parse("HTTP://Host/a/%7e/../b").unwrap().normalize();
    let n2 = n1.normalize();
    assert_eq!(n1.as_str(), n2.as_str());
}

/// RFC 3986 §3.3: with no authority a path may not begin with `//`, or the
/// rebuilt text reads as an authority. Normalization must therefore keep a path
/// that dot-segment removal reduced to `//...` apart from the scheme. Protects
/// `Iri::normalize` from panicking on (or silently re-reading as an authority)
/// an IRI that `parse` accepted.
#[test]
fn normalize_keeps_an_authorityless_double_slash_path_a_path() {
    for input in [
        "a:/..//x:y",
        "a:/.//x:y",
        "a:/b/..//x:y",
        "a:b/..//x:y",
        "a:/.//x",
        "a:/.//",
    ] {
        let original = parse(input).expect("input parses");
        let n = original.normalize();
        assert_eq!(n.scheme(), Some("a"), "{input}");
        assert_eq!(n.authority(), None, "{input}: path became an authority");
        assert!(n.path().starts_with("/.//"), "{input}: {}", n.as_str());
        let reparsed = parse(n.as_str()).expect("normal form re-parses");
        assert_eq!(reparsed.path(), n.path(), "{input}");
        // The normal form is a fixed point, so idempotence holds for the class.
        assert_eq!(n.normalize().as_str(), n.as_str(), "{input}");
    }
    assert_eq!(
        parse("a:/..//x:y").unwrap().normalize().as_str(),
        "a:/.//x:y"
    );
}

/// The neighbours of the case above must keep their current result: a `//` path
/// under an authority is legal and untouched, and an authority-less path that
/// does not start with `//` after dot removal gains no guard.
#[test]
fn normalize_adds_no_guard_where_none_is_needed() {
    for (input, expected) in [
        ("http://h/..//x", "http://h//x"),
        ("http://h//x", "http://h//x"),
        ("a:/b//x", "a:/b//x"),
        ("a:/./b", "a:/b"),
        ("a:..//x", "a:/x"),
        ("a:///x", "a:///x"),
    ] {
        assert_eq!(
            parse(input).unwrap().normalize().as_str(),
            expected,
            "{input}"
        );
    }
}

/// Bases that separate the spellings a relative reference could be confused
/// with: a deep hierarchical path, one with a query, an authority with an empty
/// path, an authority-less rooted path, and paths with no directory part.
const DISTINGUISHING_BASES: [&str; 7] = [
    "http://h/x/y/z",
    "http://h/x/y/z?bq",
    "http://h",
    "a:/p/q",
    "urn:x",
    "urn:x/y",
    "a:",
];

/// RFC 3986 §4.1 and §5.2.4: dot segments in a relative reference are
/// interpreted by resolution against a base, so `normalize` must not remove the
/// ones only a base can interpret. Asserts the exact normal form, that it is a
/// fixed point, and that it resolves like the original against each base.
fn assert_normal_form_keeps_meaning(input: &str, expected: &str) {
    let original = parse(input).expect("input parses");
    let n = original.normalize();
    assert_eq!(n.as_str(), expected, "{input}");
    assert_eq!(
        n.normalize().as_str(),
        expected,
        "{input}: not a fixed point"
    );
    assert_eq!(n.scheme(), None, "{input}: became absolute");
    assert_eq!(n.authority(), None, "{input}: gained an authority");
    for base in DISTINGUISHING_BASES {
        let base = parse(base).expect("base parses");
        assert_eq!(
            base.resolve(n.as_str())
                .expect("normal form resolves")
                .as_str(),
            base.resolve(input).expect("input resolves").as_str(),
            "{input} against {}",
            base.as_str()
        );
    }
}

/// Protects the meaning of a reference with leading `..` segments: `../a` names
/// a sibling of the base's directory and `a` a child of it, so the leading
/// segments stay, however they were spelled.
#[test]
fn normalize_keeps_leading_parent_segments_of_a_relative_reference() {
    for (input, expected) in [
        ("../a", "../a"),
        ("../../a/./b", "../../a/b"),
        ("./../a", "../a"),
        ("../a/b/../c", "../a/c"),
        ("..", ".."),
        ("../", ".."),
        ("./..", ".."),
        (".././", ".."),
        ("../..", "../.."),
        ("..?q#f", "..?q#f"),
    ] {
        assert_normal_form_keeps_meaning(input, expected);
    }
}

/// Protects the difference between the base's directory (`.`) and the base
/// document itself (the empty path): a reference that reduces to its directory
/// keeps the path `.`, with or without a query or fragment. An empty path with a
/// query or fragment is a different reference and is not rewritten.
#[test]
fn normalize_keeps_the_directory_reference_apart_from_the_empty_path() {
    for (input, expected) in [
        (".", "."),
        ("./", "."),
        ("./.", "."),
        (".?q", ".?q"),
        ("./?q", ".?q"),
        ("./#f", ".#f"),
        ("a/.", "a/"),
        ("a/b/..", "a/"),
        ("?q", "?q"),
        ("#f", "#f"),
    ] {
        assert_normal_form_keeps_meaning(input, expected);
    }
}

/// Protects a relative-path reference from becoming an absolute-path one: an
/// empty first segment stays behind `./` or `..`, where dropping the dot segment
/// would leave a leading `/`.
#[test]
fn normalize_keeps_a_relative_reference_with_an_empty_segment_relative() {
    for (input, expected) in [
        (".//x", ".//x"),
        ("..//x", "..//x"),
        (".//", ".//"),
        ("./a//b", "a//b"),
    ] {
        assert_normal_form_keeps_meaning(input, expected);
    }
}

/// Protects resolution against a base whose path has no directory part (`urn:x`):
/// there RFC 3986 §5.2.4 turns the reference's own first segment followed by
/// `..` into a leading `/`, so that pair is kept, while every later pair and
/// every `.` collapses.
#[test]
fn normalize_collapses_only_the_dot_segments_every_base_agrees_on() {
    for (input, expected) in [
        ("a/./b", "a/b"),
        ("a/b/../c", "a/c"),
        ("./a", "a"),
        ("x", "x"),
        ("a/b/c/../../d", "a/d"),
        ("a/../b", "a/../b"),
        ("a/..", "a/.."),
        ("a/../", "a/.."),
        ("a/b/../../c", "a/../c"),
        ("a/../b/../c", "a/../c"),
        ("a/../../b", "a/../../b"),
    ] {
        assert_normal_form_keeps_meaning(input, expected);
    }
}

/// Protects a rootless reference that holds a percent-encoded dot. RFC 3986
/// §5.2.4 recognises only a literal `.` or `..`, so resolution reads `%2e` as an
/// ordinary segment; decoding it during normalization would make it a dot
/// segment and move the reference. Such a segment stays encoded (`%2E`,
/// `%2E%2E`), may be removed by a later `..` like any segment, and the form is a
/// fixed point. Resolved results are compared in normal form, because the
/// original keeps its own hex case.
#[test]
fn normalize_keeps_a_percent_encoded_dot_an_ordinary_segment_when_unresolved() {
    for (input, expected) in [
        ("%2e", "%2E"),
        ("%2e%2e", "%2E%2E"),
        (".%2E", "%2E%2E"),
        ("%2e.", "%2E%2E"),
        ("%2e%2e/a", "%2E%2E/a"),
        ("a/%2e/b", "a/%2E/b"),
        ("a/%2e/../b", "a/b"),
        ("x/%2e%2e/../a", "x/a"),
        ("x/%2e/..", "x/"),
        ("%2e%2e/..", "%2E%2E/.."),
        ("%2e/%2e/..", "%2E/"),
        ("./%2e/%2E%2e/..", "%2E/"),
        ("../%2e", "../%2E"),
        ("a/%2E%2E", "a/%2E%2E"),
        ("%2e?q#f", "%2E?q#f"),
        // Neighbours: a segment that only contains an encoded dot decodes as
        // before, and a literal dot segment beside one keeps its meaning.
        ("x%2e/%2ey", "x./.y"),
        ("%2e%2e%2e", "..."),
        ("./%7e/../a", "~/../a"),
    ] {
        let n = parse(input).expect("input parses").normalize();
        assert_eq!(n.as_str(), expected, "{input}");
        assert_eq!(
            n.normalize().as_str(),
            expected,
            "{input}: not a fixed point"
        );
        for base in DISTINGUISHING_BASES {
            let base = parse(base).expect("base parses");
            assert_eq!(
                base.resolve(expected)
                    .expect("normal form resolves")
                    .normalize()
                    .as_str(),
                base.resolve(input)
                    .expect("input resolves")
                    .normalize()
                    .as_str(),
                "{input} against {}",
                base.as_str()
            );
        }
    }
    // The upper-case spelling is already the normal form, so there the resolved
    // texts are equal byte for byte.
    let base = parse("http://h/d/e/f").unwrap();
    assert_eq!(
        base.resolve("a/%2E/../b").unwrap().as_str(),
        "http://h/d/e/a/b"
    );
    assert_eq!(parse("a/%2E/../b").unwrap().normalize().as_str(), "a/b");
}

/// Neighbours of the relative-path rule that resolution treats as already
/// rooted, and so keep full RFC 3986 §5.2.4 removal: an absolute IRI, a
/// reference with an authority, and an absolute-path reference.
#[test]
fn normalize_removes_every_dot_segment_under_a_scheme_authority_or_root() {
    for (input, expected) in [
        ("http://h/../a", "http://h/a"),
        ("http://h/a/../b", "http://h/b"),
        ("a:../b", "a:b"),
        ("a:b/../c", "a:/c"),
        ("//h/../a", "//h/a"),
        ("//h/a/./b", "//h/a/b"),
        ("/a/../b", "/b"),
        ("/../a", "/a"),
        ("/.", "/"),
        // A percent-encoded dot segment is decoded first here, as before.
        ("/a/%2e%2e/../b", "/b"),
        ("http://h/a/%2e/b", "http://h/a/b"),
        ("//h/a/%2E%2E/b", "//h/b"),
    ] {
        assert_eq!(
            parse(input).unwrap().normalize().as_str(),
            expected,
            "{input}"
        );
    }
}

/// RFC 3986 §4.2: the first segment of a scheme-less relative path may not hold
/// `:`. Dot-segment removal can expose one (`./a:b` -> `a:b`), which would
/// re-parse as scheme `a`; the normal form keeps the `./` so the reference stays
/// relative, and is a fixed point.
#[test]
fn normalize_keeps_a_relative_reference_with_a_colon_segment_relative() {
    for (input, expected) in [
        ("./a:b", "./a:b"),
        ("./a:b?q#f", "./a:b?q#f"),
        ("./x:y/z", "./x:y/z"),
    ] {
        let n = parse(input).unwrap().normalize();
        assert_eq!(n.as_str(), expected, "{input}");
        assert_eq!(n.scheme(), None, "{input}: became absolute");
        assert_eq!(n.normalize().as_str(), expected, "{input}");
    }
    // A leading `..` stays, so the colon is not in the first segment and needs no
    // `./`; a retained first segment that holds one still does.
    assert_normal_form_keeps_meaning("../a:b", "../a:b");
    assert_normal_form_keeps_meaning("./a:b/../c", "./a:b/../c");
    assert_normal_form_keeps_meaning("./a/../b:c", "a/../b:c");
    // Neighbours: no prefix where the colon is not in the first segment, where
    // the path is rooted, or where a scheme is present.
    for (input, expected) in [
        ("a:b", "a:b"),
        ("./a", "a"),
        ("a/b:c", "a/b:c"),
        ("/a:b", "/a:b"),
        ("/./a:b", "/a:b"),
        ("a:./b:c", "a:b:c"),
    ] {
        assert_eq!(
            parse(input).unwrap().normalize().as_str(),
            expected,
            "{input}"
        );
    }
}

/// Resolution recomposes components the same way: a relative reference whose
/// merged path reduces to `//...` against a base with no authority must stay a
/// path, never become a wrong authority or a typed error for a valid IRI.
#[test]
fn resolve_keeps_an_authorityless_double_slash_path_a_path() {
    let base = parse("a:/p").unwrap();
    for reference in [".//x", ".//x:y", ".././/x:y", "a:/..//x:y"] {
        let r = base
            .resolve(reference)
            .unwrap_or_else(|e| panic!("{reference}: {e}"));
        assert_eq!(r.authority(), None, "{reference}: {}", r.as_str());
        let again = parse(r.as_str()).unwrap();
        assert_eq!(again.path(), r.path(), "{reference}");
        assert_eq!(again.authority(), None, "{reference}");
    }
    assert_eq!(base.resolve(".//x").unwrap().as_str(), "a:/.//x");
    // Neighbours: a network-path reference does carry an authority, and an
    // ordinary relative reference is unaffected.
    assert_eq!(base.resolve("//x").unwrap().authority(), Some("x"));
    assert_eq!(base.resolve("q/r").unwrap().as_str(), "a:/q/r");
    assert_eq!(
        parse("http://h/p")
            .unwrap()
            .resolve(".//x")
            .unwrap()
            .as_str(),
        "http://h//x"
    );
}

#[test]
fn curie_prefix_detection_matches_sssom_semantics() {
    // A real CURIE.
    assert_eq!(curie_prefix("foaf:Person"), Some("foaf"));
    // An absolute IRI must NOT be read as an `http`/`https` CURIE prefix.
    assert_eq!(curie_prefix("http://example.org/Thing"), None);
    assert_eq!(curie_prefix("https://example.org/x"), None);
    // Empty prefix is not a CURIE.
    assert_eq!(curie_prefix(":local"), None);
    // No colon at all.
    assert_eq!(curie_prefix("bareword"), None);
}

#[test]
fn expand_resolve_and_contract_round_trip() {
    let prefixes: PrefixMap = [
        ("foaf", "http://xmlns.com/foaf/0.1/"),
        ("ex", "http://example.org/"),
    ]
    .into_iter()
    .collect();

    // expand_curie: only when prefix is declared.
    assert_eq!(
        expand_curie("foaf:Person", &prefixes).as_deref(),
        Some("http://xmlns.com/foaf/0.1/Person")
    );
    assert_eq!(expand_curie("unknown:x", &prefixes), None);
    assert_eq!(expand_curie("http://a/b", &prefixes), None);

    // resolve: verbatim fallback for undeclared/non-CURIE (the SSSOM behavior).
    assert_eq!(resolve("unknown:x", &prefixes), "unknown:x");
    assert_eq!(resolve("ex:Widget", &prefixes), "http://example.org/Widget");

    // contract: the inverse, longest-namespace-match.
    assert_eq!(
        contract("http://xmlns.com/foaf/0.1/Person", &prefixes).as_deref(),
        Some("foaf:Person")
    );
    assert_eq!(contract("http://nomatch.example/x", &prefixes), None);
}

#[test]
fn contract_prefers_longest_namespace() {
    let prefixes: PrefixMap = [
        ("base", "http://example.org/"),
        ("sub", "http://example.org/sub/"),
    ]
    .into_iter()
    .collect();
    // The longer namespace wins so the CURIE is the most specific available.
    assert_eq!(
        contract("http://example.org/sub/Thing", &prefixes).as_deref(),
        Some("sub:Thing")
    );
}

#[test]
fn contract_skips_empty_prefix_binding() {
    // An empty-prefix binding must NOT yield a leading-colon ":X" (which would
    // not round-trip through curie_prefix). A non-empty match still wins.
    let prefixes: PrefixMap = [("", "http://example.org/"), ("ex", "http://example.org/")]
        .into_iter()
        .collect();
    assert_eq!(
        contract("http://example.org/Thing", &prefixes).as_deref(),
        Some("ex:Thing")
    );
    // With ONLY an empty-prefix binding, contraction yields nothing (not ":X").
    let only_empty: PrefixMap = std::iter::once(("", "http://example.org/")).collect();
    assert_eq!(contract("http://example.org/Thing", &only_empty), None);
}
