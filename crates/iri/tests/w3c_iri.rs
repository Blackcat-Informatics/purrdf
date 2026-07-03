// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! W3C / RFC IRI-validity conformance vectors.
//!
//! These are the IRI *syntax* (validity) cases — the positive/negative corpus the
//! W3C `rdf-tests` IRI suite and RFC-3987 §3.1 / RFC-3986 §1.1.2 are built from,
//! curated here as committed offline vectors (the crate is zero-dep and tests must
//! be deterministic — we do not fetch the suite at test time). This complements
//! the RFC-3986 §5.4 *resolution* table in `resolution.rs`: together they cover the
//! two halves of the acceptance criterion ("parse/validate" + "base resolution").
//!
//! Provenance of each case is noted inline; the authoritative source map (which
//! RFC section each vector is verbatim/faithful to, and why there is no vendored
//! W3C IRI manifest for this zero-dep crate) is `tests/PROVENANCE.md`. Where a
//! string is a valid IRI but not a valid (ASCII-only) URI, it appears in
//! [`valid_iri_only`].

use purrdf_iri::{parse, parse_uri};

/// Absolute references that are valid in BOTH URI and IRI mode (pure ASCII).
#[test]
fn valid_uris_and_iris() {
    // RFC-3986 §1.1.2 worked examples + common scheme forms.
    let valid = [
        "ftp://ftp.is.co.za/rfc/rfc1808.txt",
        "http://www.ietf.org/rfc/rfc2396.txt",
        "ldap://[2001:db8::7]/c=GB?objectClass?one",
        "mailto:John.Doe@example.com",
        "news:comp.infosystems.www.servers.unix",
        "tel:+1-816-555-1212",
        "telnet://192.0.2.16:80/",
        "urn:oasis:names:specification:docbook:dtd:xml:4.1.2",
        // W3C rdf-tests representative absolute IRIs.
        "http://example.org/",
        "http://example.org/path?query=1#frag",
        "http://a.example/AZaz09-._~",
        "http://example.org/%E2%9C%93", // percent-encoded UTF-8 check-mark
        "http://example.org/p;params",
        "http://example.com:8080/over/there",
        "https://example.com/a//b///c", // empty segments are legal
    ];
    for s in valid {
        assert!(parse(s).is_ok(), "IRI parse should accept {s:?}");
        assert!(parse_uri(s).is_ok(), "URI parse should accept {s:?}");
    }
}

/// Valid as an IRI (RFC-3987 ucschar) but NOT as an ASCII-only URI.
#[test]
fn valid_iri_only() {
    let iri_only = [
        "http://例え.テスト/ぱす",  // RFC-3987 §3.1 (Japanese)
        "http://धर्म.org/",          // Devanagari host
        "http://example.org/Дюрst", // Cyrillic in path
        "http://r\u{00e4}ksm\u{00f6}rg\u{00e5}s.example", // Latin-1 supplement host
    ];
    for s in iri_only {
        assert!(parse(s).is_ok(), "IRI parse should accept {s:?}");
        assert!(
            parse_uri(s).is_err(),
            "URI parse must reject non-ASCII {s:?}"
        );
    }
}

/// Strings the IRI grammar must REJECT (negative corpus). Each is invalid in IRI
/// mode (the most permissive mode), hence invalid in URI mode too.
#[test]
fn invalid_iris_are_rejected() {
    let invalid = [
        "",                              // empty
        "http://exa mple.org/",          // raw space in authority
        "http://example.org/a b",        // raw space in path
        "http://example.org/%",          // truncated percent
        "http://example.org/%2",         // truncated percent
        "http://example.org/%2G",        // non-hex percent
        "http://[2001:db8::7/",          // unterminated IP-literal
        "http://example.org/a^b",        // '^' is not in the IRI grammar
        "http://example.org/a|b",        // '|' disallowed
        "http://example.org/a\\b",       // '\' disallowed
        "http://example.org/a`b",        // backtick disallowed
        "http://example.org/a<b",        // '<' disallowed (rdf-tests negative)
        "http://example.org/a>b",        // '>' disallowed (rdf-tests negative)
        "http://example.org/a\"b",       // '\"' disallowed (rdf-tests negative)
        "http://example.org/a{b}",       // '{' '}' disallowed
        "http://example.org/a\u{0000}b", // NUL control char disallowed
        "http://h:99999/",               // port out of u16 range
        "3com:foo",                      // path-noscheme ':' in first segment
    ];
    for s in invalid {
        assert!(parse(s).is_err(), "IRI parse must reject {s:?}");
    }
}
