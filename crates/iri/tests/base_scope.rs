// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Base-scope conformance: the RFC-3986 §5.4 tables driven through the public
//! [`BaseScope`]/[`BaseIri`] layer rather than through [`purrdf_iri::Iri::resolve`]
//! directly.
//!
//! `tests/resolution.rs` proves the resolution ALGORITHM against the normative
//! table. This file proves the base LAYER that codecs actually call: that it
//! delegates to that one algorithm without drift, that a missing base is a typed
//! hard failure instead of a silently-interned relative IRI, that the two grammar
//! families are kept apart, and that [`BaseIri::relativize`] is a true inverse.
//!
//! Source of truth: RFC-3986 §5.4.1 (normal examples) and §5.4.2 (abnormal
//! examples), base `http://a/b/c/d;p?q`. See `tests/PROVENANCE.md`.

use pretty_assertions::assert_eq;
use purrdf_iri::{BaseIri, BaseOrigin, BaseScope, IriError, parse};

const BASE: &str = "http://a/b/c/d;p?q";

/// RFC-3986 §5.4.1 — the normal examples.
const NORMAL: &[(&str, &str)] = &[
    ("g:h", "g:h"),
    ("g", "http://a/b/c/g"),
    ("./g", "http://a/b/c/g"),
    ("g/", "http://a/b/c/g/"),
    ("/g", "http://a/g"),
    ("//g", "http://g"),
    ("?y", "http://a/b/c/d;p?y"),
    ("g?y", "http://a/b/c/g?y"),
    ("#s", "http://a/b/c/d;p?q#s"),
    ("g#s", "http://a/b/c/g#s"),
    ("g?y#s", "http://a/b/c/g?y#s"),
    (";x", "http://a/b/c/;x"),
    ("g;x", "http://a/b/c/g;x"),
    ("g;x?y#s", "http://a/b/c/g;x?y#s"),
    ("", "http://a/b/c/d;p?q"),
    (".", "http://a/b/c/"),
    ("./", "http://a/b/c/"),
    ("..", "http://a/b/"),
    ("../", "http://a/b/"),
    ("../g", "http://a/b/g"),
    ("../..", "http://a/"),
    ("../../", "http://a/"),
    ("../../g", "http://a/g"),
];

/// RFC-3986 §5.4.2 — the abnormal examples.
const ABNORMAL: &[(&str, &str)] = &[
    // Extra "../" that would back up past the root are ignored.
    ("../../../g", "http://a/g"),
    ("../../../../g", "http://a/g"),
    // Dot-segments where a complete path segment was expected.
    ("/./g", "http://a/g"),
    ("/../g", "http://a/g"),
    ("g.", "http://a/b/c/g."),
    (".g", "http://a/b/c/.g"),
    ("g..", "http://a/b/c/g.."),
    ("..g", "http://a/b/c/..g"),
    // Nonsensical but legal dot-segment sequences.
    ("./../g", "http://a/b/g"),
    ("./g/.", "http://a/b/c/g/"),
    ("g/./h", "http://a/b/c/g/h"),
    ("g/../h", "http://a/b/c/h"),
    ("g;x=1/./y", "http://a/b/c/g;x=1/y"),
    ("g;x=1/../y", "http://a/b/c/y"),
    // Dot-segments only matter in the path, not in query or fragment.
    ("g?y/./x", "http://a/b/c/g?y/./x"),
    ("g?y/../x", "http://a/b/c/g?y/../x"),
    ("g#s/./x", "http://a/b/c/g#s/./x"),
    ("g#s/../x", "http://a/b/c/g#s/../x"),
    // Strict resolution: a same-scheme reference is NOT treated as relative.
    ("http:g", "http:g"),
];

fn rooted_scope() -> BaseScope {
    BaseScope::rooted(
        BaseIri::parse(BASE).expect("base parses"),
        BaseOrigin::Caller,
    )
}

fn resolve_through_scope(scope: &BaseScope, reference: &str) -> String {
    scope
        .resolve(reference)
        .unwrap_or_else(|e| panic!("resolve({reference:?}) failed: {e}"))
        .as_str()
        .to_owned()
}

#[test]
fn base_scope_matches_rfc3986_5_4_1_normal_examples() {
    let scope = rooted_scope();
    for (reference, expected) in NORMAL {
        assert_eq!(
            &resolve_through_scope(&scope, reference),
            expected,
            "ref = {reference:?}"
        );
    }
}

#[test]
fn base_scope_matches_rfc3986_5_4_2_abnormal_examples() {
    let scope = rooted_scope();
    for (reference, expected) in ABNORMAL {
        assert_eq!(
            &resolve_through_scope(&scope, reference),
            expected,
            "ref = {reference:?}"
        );
    }
}

/// The bug that motivated this layer: `<>` is the same-document reference, which
/// keeps the base's query and drops its fragment (RFC-3986 §4.4 / §5.4.1).
#[test]
fn empty_reference_is_the_same_document_reference() {
    let scope = rooted_scope();
    assert_eq!(&resolve_through_scope(&scope, ""), "http://a/b/c/d;p?q");

    // The base's own fragment is NOT carried into the resolved reference.
    let with_fragment = BaseScope::rooted(
        BaseIri::parse("http://a/b/c/d;p?q#frag").expect("base parses"),
        BaseOrigin::Caller,
    );
    assert_eq!(
        &resolve_through_scope(&with_fragment, ""),
        "http://a/b/c/d;p?q"
    );
}

/// A network-path reference keeps the base's scheme and replaces the authority.
#[test]
fn network_path_reference_keeps_scheme_replaces_authority() {
    let scope = rooted_scope();
    assert_eq!(&resolve_through_scope(&scope, "//g"), "http://g");
    assert_eq!(
        &resolve_through_scope(&scope, "//g/x?y#z"),
        "http://g/x?y#z"
    );
}

#[test]
fn relative_reference_without_a_base_is_no_base() {
    let scope = BaseScope::empty();
    assert!(scope.is_empty());

    for reference in ["", "foo", "./foo", "../foo", "/foo", "#frag", "?q"] {
        let err = scope
            .resolve(reference)
            .expect_err("a relative reference needs a base");
        assert!(
            matches!(&err, IriError::NoBase { reference: r } if r == reference),
            "ref = {reference:?} produced {err:?}"
        );
        assert_eq!(err.diagnostic_code(), "iri-relative-no-base");
    }
}

/// With no base in scope an ABSOLUTE reference still resolves — the missing base
/// only matters for references that actually need one.
#[test]
fn absolute_reference_resolves_without_a_base() {
    let scope = BaseScope::empty();
    assert_eq!(
        scope
            .resolve("http://example.org/x?y#z")
            .expect("absolute reference needs no base")
            .as_str(),
        "http://example.org/x?y#z"
    );
}

/// `resolve_absolute_only` is for grammars whose syntax admits no relative
/// reference at all. The base is NEVER applied — the same input must fail
/// identically whether or not a base happens to be in scope.
#[test]
fn absolute_only_grammar_rejects_relative_references_with_and_without_a_base() {
    let empty = BaseScope::empty();
    let rooted = rooted_scope();

    for reference in ["", "foo", "./foo", "/foo", "#frag"] {
        for (label, scope) in [("empty", &empty), ("rooted", &rooted)] {
            let err = scope
                .resolve_absolute_only(reference)
                .expect_err("grammar admits only absolute IRIs");
            assert!(
                matches!(&err, IriError::NotAbsoluteByGrammar { reference: r } if r == reference),
                "ref = {reference:?} in {label} scope produced {err:?}"
            );
            assert_eq!(err.diagnostic_code(), "iri-not-absolute-by-grammar");
        }
    }

    // An absolute IRI passes in both scopes, verbatim (no base merging).
    for scope in [&empty, &rooted] {
        assert_eq!(
            scope
                .resolve_absolute_only("http://example.org/x")
                .expect("absolute IRI is admitted")
                .as_str(),
            "http://example.org/x"
        );
    }
}

/// RFC-3986 §4.2 `path-noscheme`: a relative reference's first segment may not
/// contain a ':'. That is a SYNTAX error about the reference itself, so it must be
/// reported as the parse-level variant — reporting `NoBase` would send the user off
/// to add a `@base` that cannot possibly fix it.
#[test]
fn path_noscheme_reference_is_a_parse_error_not_no_base() {
    // `1a:b` cannot be a scheme (schemes must start with ALPHA), so it parses as a
    // relative reference whose first segment illegally contains ':'.
    let empty = BaseScope::empty();
    let err = empty
        .resolve("1a:b")
        .expect_err("path-noscheme is rejected");
    assert!(
        matches!(err, IriError::DisallowedChar(':', _)),
        "expected the parse-level variant, got {err:?}"
    );
    assert_eq!(err.diagnostic_code(), "iri-disallowed-char");
    assert!(!matches!(err, IriError::NoBase { .. }));

    // Same verdict with a base in scope: the reference is malformed either way.
    let rooted = rooted_scope();
    let err = rooted
        .resolve("1a:b")
        .expect_err("path-noscheme is rejected");
    assert!(
        matches!(err, IriError::DisallowedChar(':', _)),
        "expected the parse-level variant, got {err:?}"
    );

    // A ':' in a LATER segment is legal and resolves normally.
    assert_eq!(
        &resolve_through_scope(&rooted, "foo/bar:baz"),
        "http://a/b/c/foo/bar:baz"
    );
}

/// A `@base` directive may itself be relative, resolved against the base already in
/// force (Turtle §6.1, RFC-3986 §5.1.1) — so directives compose.
#[test]
fn rebind_chains_three_deep() {
    let root = BaseIri::parse("http://example.org/a/b/c").expect("absolute base");
    let first = root.rebind("d/").expect("relative directive");
    assert_eq!(first.as_str(), "http://example.org/a/b/d/");
    let second = first.rebind("e/").expect("relative directive");
    assert_eq!(second.as_str(), "http://example.org/a/b/d/e/");
    let third = second.rebind("../f/").expect("relative directive");
    assert_eq!(third.as_str(), "http://example.org/a/b/d/f/");

    // The same chain through the scope, which replaces the top rather than nesting.
    let mut scope = BaseScope::rooted(
        BaseIri::parse("http://example.org/a/b/c").expect("absolute base"),
        BaseOrigin::Caller,
    );
    for (line, directive) in [(2u32, "d/"), (3, "e/"), (4, "../f/")] {
        scope
            .rebind(directive, BaseOrigin::Directive { line, column: 1 })
            .expect("directive rebinds");
        assert_eq!(scope.depth(), 1, "rebind must not nest");
    }
    let current = scope.current().expect("a base is in force");
    assert_eq!(current.iri().as_str(), "http://example.org/a/b/d/f/");
    assert_eq!(
        current.origin(),
        BaseOrigin::Directive { line: 4, column: 1 },
        "origin tracks the LAST directive that rebound the base"
    );

    // An absolute directive discards the chain entirely.
    scope
        .rebind(
            "http://other.example/z/",
            BaseOrigin::Directive { line: 5, column: 1 },
        )
        .expect("absolute directive rebinds");
    assert_eq!(
        scope.current().unwrap().iri().as_str(),
        "http://other.example/z/"
    );
}

/// `relativize` is the exact inverse of `resolve`: over the whole §5.4 corpus, any
/// relative spelling it produces must resolve back to the identical absolute IRI.
#[test]
fn relativize_round_trips_every_rfc3986_5_4_pair() {
    let base = BaseIri::parse(BASE).expect("base parses");
    let mut relativized = 0usize;

    for (reference, expected) in NORMAL.iter().chain(ABNORMAL.iter()) {
        let target = parse(expected).expect("expected value parses");
        if let Some(rel) = base.relativize(&target) {
            relativized += 1;
            let back = base
                .resolve(&rel)
                .unwrap_or_else(|e| panic!("resolve({rel:?}) failed: {e}"));
            assert_eq!(
                back.as_str(),
                *expected,
                "relativize({expected:?}) = {rel:?} (from ref {reference:?}) did not round-trip"
            );
        }
    }

    // Guard against a vacuous pass: `relativize` returning `None` everywhere would
    // satisfy the property above while being useless.
    assert!(
        relativized >= 30,
        "expected most §5.4 targets to relativize, got {relativized}"
    );
}

/// The `None` cases are semantic: there is genuinely no relative spelling.
#[test]
fn relativize_returns_none_when_no_relative_spelling_exists() {
    let base = BaseIri::parse(BASE).expect("base parses");

    // Different scheme.
    assert_eq!(base.relativize(&parse("https://a/b/c/g").unwrap()), None);
    assert_eq!(base.relativize(&parse("g:h").unwrap()), None);
    // Different authority.
    assert_eq!(base.relativize(&parse("http://g/b/c/d").unwrap()), None);
    // No authority at all where the base has one.
    assert_eq!(base.relativize(&parse("http:g").unwrap()), None);
}

/// The serializer cases the reported bug is about: `<>` and `<foo>` under a base.
#[test]
fn relativize_produces_the_turtle_spellings() {
    let base = BaseIri::parse("http://example.org/dir/doc.ttl").expect("base parses");

    let cases: &[(&str, &str)] = &[
        // The base itself is `<>`.
        ("http://example.org/dir/doc.ttl", ""),
        ("http://example.org/dir/other", "other"),
        ("http://example.org/dir/sub/x", "sub/x"),
        ("http://example.org/up", "../up"),
        ("http://example.org/dir/", "./"),
        ("http://example.org/dir/doc.ttl#frag", "#frag"),
        ("http://example.org/dir/doc.ttl?q", "?q"),
    ];
    for (target, expected_rel) in cases {
        let iri = parse(target).expect("target parses");
        assert_eq!(
            base.relativize(&iri).as_deref(),
            Some(*expected_rel),
            "target = {target:?}"
        );
        assert_eq!(
            base.resolve(expected_rel).expect("round trip").as_str(),
            *target
        );
    }
}

/// A target whose first segment contains ':' must be spelled `./x:y`, never `x:y`
/// (which would re-parse as a scheme) — RFC-3986 §4.2.
#[test]
fn relativize_guards_the_path_noscheme_case() {
    let base = BaseIri::parse("http://example.org/dir/doc.ttl").expect("base parses");
    let target = parse("http://example.org/dir/x:y").expect("target parses");
    let rel = base.relativize(&target).expect("a spelling exists");
    assert_eq!(rel, "./x:y");
    assert_eq!(
        base.resolve(&rel).expect("round trip").as_str(),
        "http://example.org/dir/x:y"
    );
}

/// Resolution must use the RFC-3987 IRI grammar, not the ASCII-only RFC-3986 URI
/// subset: Turtle hands us `UCHAR`-decoded non-ASCII (W3C test060) and an
/// ASCII-only parse would regress it.
#[test]
fn base_layer_accepts_non_ascii_iris() {
    let scope = BaseScope::rooted(
        BaseIri::parse("http://example.org/caf\u{e9}/").expect("non-ASCII base parses"),
        BaseOrigin::Caller,
    );
    assert_eq!(
        &resolve_through_scope(&scope, "na\u{ef}ve"),
        "http://example.org/caf\u{e9}/na\u{ef}ve"
    );
}
