// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Parse/validate + normalize + CURIE conformance.
//!
//! Vectors are curated from RFC-3987 §3.1 (IRI examples), RFC-3986 §1.1.2 / §3
//! (URI examples and component grammar), RFC-3986 §6.2.2 (normalization), and the
//! W3C `rdf-tests` IRI handling cases. The CURIE cases pin the semantics this crate
//! subsumes from the SSSOM serializer (`sssom::curie_prefix`/`resolve_iri`).

use pretty_assertions::assert_eq;
use purrdf_iri::{contract, curie_prefix, expand_curie, parse, parse_uri, resolve, PrefixMap};

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
fn rejects_port_out_of_u16_range() {
    assert!(parse("http://h:8080/p").is_ok());
    // Empty port is grammar-legal (`port = *DIGIT`).
    assert!(parse("http://h:/p").is_ok());
    // 99999 > 65535 -> reject rather than silently accept.
    assert!(parse("http://h:99999/p").is_err());
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
    let only_empty: PrefixMap = [("", "http://example.org/")].into_iter().collect();
    assert_eq!(contract("http://example.org/Thing", &only_empty), None);
}
