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
use purrdf_iri::{BaseInScope, BaseIri, BaseOrigin, BaseScope, IriError, parse};

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

/// Absolute references that carry dot segments — the shapes the §5.4 table never
/// writes, and the ones that expose whether the base layer normalizes behind the
/// document's back.
const ABSOLUTE_WITH_DOT_SEGMENTS: &[&str] = &[
    "http://a/b/../c",
    "http://a/./b",
    "http://a/b/c/../../d",
    "http://a/../../g",
    "http://a/b/c/.",
    "http://a/b/c/..",
    "http://a/b/../c?x=../y#../z",
    "g:./h",
    "http:g/../h",
    // The exact spellings the W3C JSON-LD REC vectors 0122/0123 pin.
    "http://a/bb/ccc/./d;p?q",
    "http://a/bb/ccc/../d;p?y",
];

/// Every ABSOLUTE reference — the §5.4 corpus's absolute inputs, every resolved
/// output fed back in, and the dot-bearing spellings above — must produce the
/// IDENTICAL IRI with and without a base in scope.
///
/// This was the defect: with a base in scope the reference went through RFC-3986
/// §5.2.2, whose scheme-bearing branch applies `remove_dot_segments(R.path)`, and
/// without one it did not. `<http://a/b/../c>` therefore interned as `http://a/c`
/// in a document with a `@base` and `http://a/b/../c` in the same document without
/// one: identical bytes, two graphs, two RDFC-1.0 digests.
#[test]
fn absolute_references_resolve_identically_with_and_without_a_base() {
    let rooted = rooted_scope();
    let empty = BaseScope::empty();

    let corpus = NORMAL
        .iter()
        .chain(ABNORMAL.iter())
        .flat_map(|(reference, expected)| [*reference, *expected])
        // The relative half of the corpus is meaningless without a base; the
        // absolute half is what must agree.
        .filter(|s| parse(s).is_ok_and(|iri| iri.scheme().is_some()))
        .chain(ABSOLUTE_WITH_DOT_SEGMENTS.iter().copied());

    let mut checked = 0usize;
    for reference in corpus {
        let with = rooted
            .resolve(reference)
            .unwrap_or_else(|e| panic!("rooted resolve({reference:?}) failed: {e}"));
        let without = empty
            .resolve(reference)
            .unwrap_or_else(|e| panic!("empty resolve({reference:?}) failed: {e}"));
        assert_eq!(
            without.as_str(),
            with.as_str(),
            "absolute reference {reference:?} resolved differently without a base"
        );
        checked += 1;
    }
    assert!(checked >= 40, "corpus collapsed to {checked} references");
}

/// …and the agreed answer is the reference VERBATIM, in both grammar families.
///
/// The RDF grammars resolve relative IRIs only; `remove_dot_segments` on an IRI a
/// document spelled absolutely is RFC-3986 §6.2.2.3 syntax-based normalization,
/// which RDF Concepts §3.2 forbids ("Further normalization MUST NOT be performed").
/// The W3C JSON-LD REC vectors pin it directly: 0122/0123 require
/// `<http://a/bb/ccc/../d;p?q>` to come out intact, and `crates/rdf`'s N-Quads
/// oracle re-parses exactly those bytes.
#[test]
fn an_absolute_reference_is_never_normalized_by_either_grammar_family() {
    let empty = BaseScope::empty();
    let rooted = rooted_scope();

    let absolute = NORMAL
        .iter()
        .chain(ABNORMAL.iter())
        .flat_map(|(reference, expected)| [*reference, *expected])
        .filter(|s| parse(s).is_ok_and(|iri| iri.scheme().is_some()))
        .chain(ABSOLUTE_WITH_DOT_SEGMENTS.iter().copied())
        // A near-miss set: `.`/`..` inside a segment is ordinary path data and must
        // survive too, so a sloppy normalizer cannot pass by trimming those instead.
        .chain(["http://a/b/.c/..d/e./f..", "http://a/b%2E%2E/c"]);

    for reference in absolute {
        for (label, scope) in [("empty", &empty), ("rooted", &rooted)] {
            assert_eq!(
                &resolve_through_scope(scope, reference),
                reference,
                "resolve({reference:?}) rewrote an absolute IRI in the {label} scope"
            );
            assert_eq!(
                scope
                    .resolve_absolute_only(reference)
                    .unwrap_or_else(|e| panic!("resolve_absolute_only({reference:?}): {e}"))
                    .as_str(),
                reference,
                "resolve_absolute_only({reference:?}) rewrote an absolute IRI \
                 in the {label} scope"
            );
        }
    }
}

/// A `@base` directive is a reference too, and obeys the same rule: an absolute one
/// establishes exactly what it says, whether or not a base preceded it.
#[test]
fn an_absolute_base_directive_is_taken_verbatim_with_or_without_a_predecessor() {
    let directive = "http://a/bb/ccc/./d;p?q";

    let mut from_empty = BaseScope::empty();
    from_empty
        .rebind(directive, BaseOrigin::Directive { line: 1, column: 1 })
        .expect("absolute directive roots the scope");

    let mut from_rooted = rooted_scope();
    from_rooted
        .rebind(directive, BaseOrigin::Directive { line: 2, column: 1 })
        .expect("absolute directive replaces the base");

    for scope in [&from_empty, &from_rooted] {
        assert_eq!(
            scope.current().expect("a base is in force").iri().as_str(),
            directive
        );
    }

    // And the base's own dot segments survive resolution of a relative reference
    // against it — RFC-3986 §5.2.2 copies the base path verbatim for an empty-path
    // reference, which is what the W3C JSON-LD vectors pin.
    assert_eq!(
        &resolve_through_scope(&from_rooted, "?y"),
        "http://a/bb/ccc/./d;p?y"
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
                matches!(&err, IriError::NotAbsoluteByGrammar { reference: r, .. } if r == reference),
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

/// The base in force, and where it came from, must appear in the rendered message.
///
/// Every consumer in the workspace prints `{err}` and nothing else, so provenance
/// that lives only in a `BaseOrigin` accessor reaches no user — which is what made
/// nine production sites construct one that nothing ever read. These assertions are
/// on `format!("{err}")` for exactly that reason: they fail if the delivery is
/// removed, not merely if the field is.
#[test]
fn the_base_in_force_and_its_provenance_are_rendered_in_the_diagnostic() {
    // A caller-supplied base — the `--base` case, where a user needs to be told the
    // base is in scope and simply never applied by this syntax.
    let caller = rooted_scope();
    let err = caller
        .resolve_absolute_only("foo")
        .expect_err("grammar admits only absolute IRIs");
    let rendered = format!("{err}");
    assert!(
        rendered.contains("the caller-supplied base"),
        "message names no provenance: {rendered}"
    );
    assert!(
        rendered.contains(&format!("<{BASE}>")),
        "message names no base: {rendered}"
    );
    assert!(
        rendered.contains("never applied here"),
        "message does not say the base is not applied: {rendered}"
    );
    assert_eq!(
        BaseInScope::of(&caller),
        BaseInScope::InForce {
            iri: BASE.to_owned(),
            origin: BaseOrigin::Caller,
        }
    );

    // A base established by a directive names the directive's line and column.
    let mut directive = BaseScope::empty();
    directive
        .rebind(
            "http://example.org/dir/",
            BaseOrigin::Directive { line: 3, column: 1 },
        )
        .expect("absolute directive roots the scope");
    let rendered = format!(
        "{}",
        directive
            .resolve_absolute_only("foo")
            .expect_err("grammar admits only absolute IRIs")
    );
    assert!(
        rendered.contains("the `@base` at line 3 column 1"),
        "message names no directive position: {rendered}"
    );
    assert!(
        rendered.contains("<http://example.org/dir/>"),
        "message names no base: {rendered}"
    );

    // A base inherited from an enclosing lexical scope says so.
    let mut enclosing = rooted_scope();
    enclosing.push(
        BaseIri::parse("http://example.org/inner/").expect("base parses"),
        BaseOrigin::Enclosing,
    );
    let rendered = format!(
        "{}",
        enclosing
            .resolve_absolute_only("foo")
            .expect_err("grammar admits only absolute IRIs")
    );
    assert!(
        rendered.contains("the enclosing scope's base"),
        "message names no provenance: {rendered}"
    );

    // With nothing in scope, the diagnostic says exactly that — RFC-3986 §5.1.4 —
    // in both grammar families, with one wording.
    let empty = BaseScope::empty();
    assert_eq!(BaseInScope::of(&empty), BaseInScope::Absent);
    for rendered in [
        format!("{}", empty.resolve("foo").expect_err("needs a base")),
        format!(
            "{}",
            empty
                .resolve_absolute_only("foo")
                .expect_err("grammar admits only absolute IRIs")
        ),
    ] {
        assert!(
            rendered.contains("no base IRI is in scope"),
            "message does not state the absent base: {rendered}"
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
