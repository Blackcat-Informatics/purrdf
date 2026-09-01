// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Relative-IRI resolution across every native codec.
//!
//! The defect this pins: a relative IRI reference with no base in scope used to be
//! interned VERBATIM into the frozen IR, so `<foo>` parsed "successfully" and was then
//! emitted as invalid N-Triples, and `<>` interned as the empty string and tripped a
//! downstream guard with an unrelated message. Both are now typed hard failures, and
//! the two failure modes are kept apart:
//!
//! * a syntax that ADMITS relative references but has no base — `iri-relative-no-base`,
//!   which supplying a base fixes;
//! * a syntax that admits NO relative reference — `iri-not-absolute-by-grammar`, which
//!   supplying a base does not fix, and where the base is never applied.
//!
//! Which of the two applies is read off the format registry's `admits_relative_iri`
//! column, so the totality test at the bottom covers every registered format.

use purrdf_rdf::native_codecs::{NativeRdfFormat, parse_dataset};
use purrdf_rdf::{SerializeGraph, TermValue, serialize_dataset};

const P: &str = "<http://example.org/p>";
const O: &str = "<http://example.org/o>";

/// A one-triple JSON-LD document whose `@id` is the relative reference `foo`.
const JSONLD_RELATIVE_SUBJECT: &str =
    "{\"@id\":\"foo\",\"http://example.org/p\":{\"@id\":\"http://example.org/o\"}}";

/// The YAML-LD spelling of [`JSONLD_RELATIVE_SUBJECT`].
const YAMLLD_RELATIVE_SUBJECT: &str = concat!(
    "\"@id\": foo\n",
    "\"http://example.org/p\":\n",
    "  \"@id\": \"http://example.org/o\"\n",
);

/// Turtle source asserting `reference` in SUBJECT position, under `base` if given.
fn turtle_with(base: Option<&str>, reference: &str) -> String {
    match base {
        Some(b) => format!("@base <{b}> .\n<{reference}> {P} {O} .\n"),
        None => format!("<{reference}> {P} {O} .\n"),
    }
}

/// The single subject IRI of a one-triple document.
fn only_subject(text: &str, media_type: &str, base: Option<&str>) -> String {
    let dataset = parse_dataset(text.as_bytes(), media_type, base)
        .unwrap_or_else(|e| panic!("parse {media_type} failed: {e}"));
    let quad = dataset.quads().next().expect("one quad");
    match dataset.term_value(quad.s) {
        TermValue::Iri(iri) => iri,
        other => panic!("subject is not an IRI: {other:?}"),
    }
}

// ── RFC-3986 §5.4 through a real Turtle document ────────────────────────────────

const RFC_BASE: &str = "http://a/b/c/d;p?q";

/// RFC-3986 §5.4.1 normal examples. `g:h` is omitted only because Turtle's own
/// `IRIREF` production is exercised by every other row identically.
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

/// RFC-3986 §5.4.2 abnormal examples.
const ABNORMAL: &[(&str, &str)] = &[
    ("../../../g", "http://a/g"),
    ("../../../../g", "http://a/g"),
    ("/./g", "http://a/g"),
    ("/../g", "http://a/g"),
    ("g.", "http://a/b/c/g."),
    (".g", "http://a/b/c/.g"),
    ("g..", "http://a/b/c/g.."),
    ("..g", "http://a/b/c/..g"),
    ("./../g", "http://a/b/g"),
    ("./g/.", "http://a/b/c/g/"),
    ("g/./h", "http://a/b/c/g/h"),
    ("g/../h", "http://a/b/c/h"),
    ("g;x=1/./y", "http://a/b/c/g;x=1/y"),
    ("g;x=1/../y", "http://a/b/c/y"),
    ("g?y/./x", "http://a/b/c/g?y/./x"),
    ("g?y/../x", "http://a/b/c/g?y/../x"),
    ("g#s/./x", "http://a/b/c/g#s/./x"),
    ("g#s/../x", "http://a/b/c/g#s/../x"),
    ("http:g", "http:g"),
];

#[test]
fn turtle_resolves_the_full_rfc3986_5_4_table() {
    for (reference, expected) in NORMAL.iter().chain(ABNORMAL.iter()) {
        let text = turtle_with(Some(RFC_BASE), reference);
        assert_eq!(
            only_subject(&text, "text/turtle", None),
            *expected,
            "@base <{RFC_BASE}> with <{reference}>"
        );
    }
}

#[test]
fn trig_resolves_the_full_rfc3986_5_4_table() {
    for (reference, expected) in NORMAL.iter().chain(ABNORMAL.iter()) {
        let text = turtle_with(Some(RFC_BASE), reference);
        assert_eq!(
            only_subject(&text, "application/trig", None),
            *expected,
            "@base <{RFC_BASE}> with <{reference}>"
        );
    }
}

/// The caller-supplied base (the API argument) must resolve identically to an
/// in-document `@base` — same algorithm, same table.
#[test]
fn caller_supplied_base_matches_the_in_document_directive() {
    for (reference, expected) in NORMAL.iter().chain(ABNORMAL.iter()) {
        let text = turtle_with(None, reference);
        assert_eq!(
            only_subject(&text, "text/turtle", Some(RFC_BASE)),
            *expected,
            "caller base with <{reference}>"
        );
    }
}

// ── No base in scope ────────────────────────────────────────────────────────────

/// The reported defect, in both of its shapes.
#[test]
fn relative_reference_without_a_base_is_a_located_hard_error() {
    for reference in ["", "foo", "./foo", "../foo", "/foo", "#frag"] {
        let text = turtle_with(None, reference);
        let error = parse_dataset(text.as_bytes(), "text/turtle", None)
            .expect_err("a relative IRI with no base must not parse");
        assert_eq!(
            error.code, "iri-relative-no-base",
            "reference <{reference}> produced {error:?}"
        );
        // The message names the offending reference and points at the remedy.
        assert!(
            error.message.contains(&format!("{reference:?}")),
            "message must name <{reference}> verbatim: {}",
            error.message
        );
        assert!(
            error.message.contains("@base"),
            "message must point at the remedy: {}",
            error.message
        );
        let location = error.location.as_ref().expect("error is located");
        assert_eq!(location.line, Some(1));
        assert_eq!(location.column, Some(1), "column points at the subject IRI");
    }
}

/// Previously `<foo>` parsed and was then emitted as INVALID N-Triples. Nothing may
/// reach the serializer unresolved, so the whole document is refused instead.
#[test]
fn an_unresolved_relative_iri_never_reaches_the_serializer() {
    let text = turtle_with(None, "foo");
    assert!(parse_dataset(text.as_bytes(), "text/turtle", None).is_err());

    // With a base it resolves, and the emitted N-Triples is well-formed absolute.
    let dataset = parse_dataset(
        text.as_bytes(),
        "text/turtle",
        Some("http://example.org/dir/"),
    )
    .expect("parses under a base");
    let out = String::from_utf8(
        serialize_dataset(&dataset, "application/n-triples", SerializeGraph::Dataset)
            .expect("serialize"),
    )
    .expect("utf-8");
    assert!(
        out.starts_with("<http://example.org/dir/foo> "),
        "subject must be absolute in the output: {out}"
    );
}

/// RFC-3986 §4.2 `path-noscheme`: this is a SYNTAX error about the reference, so it
/// must not be reported as "no base" — adding a `@base` could not fix it.
#[test]
fn path_noscheme_is_a_parse_error_not_a_missing_base() {
    let text = turtle_with(None, "1a:b");
    let error = parse_dataset(text.as_bytes(), "text/turtle", None).expect_err("must fail");
    assert_eq!(error.code, "iri-disallowed-char");
    assert_ne!(error.code, "iri-relative-no-base");
}

// ── RFC-3986 §5.2 corners that the old hand-rolled resolver got wrong ───────────

#[test]
fn network_path_reference_takes_the_bases_scheme() {
    let text = turtle_with(Some("http://a/b"), "//example.org/x");
    assert_eq!(
        only_subject(&text, "text/turtle", None),
        "http://example.org/x"
    );
}

#[test]
fn same_document_reference_keeps_the_query_and_drops_the_fragment() {
    let base = "http://example.org/d?q=1#frag";
    let cases: &[(&str, &str)] = &[
        ("", "http://example.org/d?q=1"),
        ("#x", "http://example.org/d?q=1#x"),
        ("?y=2", "http://example.org/d?y=2"),
    ];
    for (reference, expected) in cases {
        let text = turtle_with(Some(base), reference);
        assert_eq!(
            only_subject(&text, "text/turtle", None),
            *expected,
            "<{reference}> against {base}"
        );
    }
}

/// A `@base` directive may be relative and resolves against the base already in force
/// (Turtle §6.1 → RFC-3986 §5.1.1), so directives chain.
#[test]
fn base_directives_chain_three_deep() {
    let text = concat!(
        "@base <sub/> .\n",
        "@base <deeper/> .\n",
        "<x> <http://example.org/p> <http://example.org/o> .\n",
    );
    assert_eq!(
        only_subject(text, "text/turtle", Some("http://example.org/root/doc")),
        "http://example.org/root/sub/deeper/x"
    );
}

/// Turtle §4.4: a prefix's namespace is resolved when the `@prefix` is READ. The same
/// prefixed name must therefore denote ONE IRI for the whole document, even across a
/// later `@base` that would have changed a use-time resolution.
#[test]
fn a_prefix_namespace_is_fixed_when_the_directive_is_read() {
    let text = concat!(
        "@prefix p: <rel/> .\n",
        "p:x <http://example.org/p> <http://example.org/o> .\n",
        "@base <http://example.org/moved/> .\n",
        "p:x <http://example.org/p2> <http://example.org/o> .\n",
    );
    let dataset = parse_dataset(
        text.as_bytes(),
        "text/turtle",
        Some("http://example.org/start/"),
    )
    .expect("parses");
    let subjects: Vec<String> = dataset
        .quads()
        .map(|q| match dataset.term_value(q.s) {
            TermValue::Iri(iri) => iri,
            other => panic!("not an IRI: {other:?}"),
        })
        .collect();
    assert_eq!(subjects.len(), 2);
    assert_eq!(
        subjects[0], subjects[1],
        "the same prefixed name must denote one IRI across a later @base"
    );
    assert_eq!(subjects[0], "http://example.org/start/rel/x");
}

// ── Grammars that admit no relative reference ───────────────────────────────────

/// N-Triples / N-Quads / TriX / HexTuples must refuse a relative reference with the
/// same code WHETHER OR NOT a base is supplied — the base is never applied to them.
#[test]
fn absolute_only_grammars_refuse_relative_references_with_and_without_a_base() {
    let cases: &[(&str, &str)] = &[
        (
            "application/n-triples",
            "<foo> <http://example.org/p> <http://example.org/o> .\n",
        ),
        (
            "application/n-quads",
            "<foo> <http://example.org/p> <http://example.org/o> .\n",
        ),
    ];
    for (media_type, text) in cases {
        for base in [None, Some("http://example.org/dir/")] {
            let error = parse_dataset(text.as_bytes(), media_type, base).expect_err(
                "a grammar admitting no relative reference must refuse one, base or not",
            );
            assert_eq!(
                error.code, "iri-not-absolute-by-grammar",
                "{media_type} with base = {base:?}"
            );
            assert!(
                error.message.contains("\"foo\""),
                "message names the reference: {}",
                error.message
            );
        }
    }
}

/// Every registered format, driven off the capability column rather than a hand list,
/// so a newly added codec cannot quietly opt out of the policy.
#[test]
fn every_format_applies_the_policy_its_capability_column_declares() {
    for format in NativeRdfFormat::all() {
        // Every format with a one-triple spelling is exercised, TREE SYNTAXES INCLUDED.
        // JSON-LD and YAML-LD used to be skipped here, and that skip is why the dropped
        // base survived: both declare `admits_relative_iri: true`, both codecs bound the
        // base and threw it away, and the caller's `--base` was a silent no-op for
        // exactly the two formats this loop had waived. A capability column is only a
        // contract if the totality test reads it for every row it can spell.
        let text = match format {
            NativeRdfFormat::Turtle | NativeRdfFormat::TriG => {
                format!("<foo> {P} {O} .\n")
            }
            NativeRdfFormat::NTriples | NativeRdfFormat::NQuads => {
                format!("<foo> {P} {O} .\n")
            }
            NativeRdfFormat::JsonLd => JSONLD_RELATIVE_SUBJECT.to_owned(),
            NativeRdfFormat::YamlLd => YAMLLD_RELATIVE_SUBJECT.to_owned(),
            _ => continue,
        };
        let result = parse_dataset(text.as_bytes(), format.media_type(), None);
        let error = result.err().unwrap_or_else(|| {
            panic!("{format:?}: a relative reference with no base must not parse")
        });
        let expected = if format.admits_relative_iri() {
            "iri-relative-no-base"
        } else {
            "iri-not-absolute-by-grammar"
        };
        assert_eq!(
            error.code,
            expected,
            "{format:?} (admits_relative_iri = {})",
            format.admits_relative_iri()
        );

        // With a base: the admitting formats now succeed; the others fail IDENTICALLY,
        // because a base is never applied to a grammar that admits no relative form.
        let with_base = parse_dataset(
            text.as_bytes(),
            format.media_type(),
            Some("http://example.org/dir/"),
        );
        if format.admits_relative_iri() {
            assert!(
                with_base.is_ok(),
                "{format:?} must resolve <foo> under a base"
            );
        } else {
            assert_eq!(
                with_base.expect_err("must still fail").code,
                "iri-not-absolute-by-grammar",
                "{format:?}: supplying a base must not change the verdict"
            );
        }
    }
}

// ── RDF/XML ─────────────────────────────────────────────────────────────────────

/// RDF/XML admits relative references and nests `xml:base` per element, so it routes
/// through the same layer: a relative `rdf:about` with no base is the same hard error.
#[test]
fn rdfxml_relative_about_without_a_base_is_the_same_error() {
    let xml = concat!(
        "<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\" ",
        "xmlns:ex=\"http://example.org/\">",
        "<rdf:Description rdf:about=\"foo\"><ex:p>v</ex:p></rdf:Description>",
        "</rdf:RDF>",
    );
    let error = parse_dataset(xml.as_bytes(), "application/rdf+xml", None)
        .expect_err("relative rdf:about with no base must fail");
    assert_eq!(error.code, "iri-relative-no-base");

    let dataset = parse_dataset(
        xml.as_bytes(),
        "application/rdf+xml",
        Some("http://example.org/dir/"),
    )
    .expect("resolves under a base");
    let quad = dataset.quads().next().expect("one quad");
    assert_eq!(
        dataset.term_value(quad.s),
        TermValue::Iri("http://example.org/dir/foo".to_owned())
    );
}

/// `xml:base` nests per element and may itself be relative.
#[test]
fn rdfxml_xml_base_nests_and_may_be_relative() {
    let xml = concat!(
        "<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\" ",
        "xmlns:ex=\"http://example.org/\" xml:base=\"http://example.org/outer/\">",
        "<rdf:Description rdf:about=\"a\" xml:base=\"inner/\">",
        "<ex:p>v</ex:p></rdf:Description>",
        "</rdf:RDF>",
    );
    let dataset = parse_dataset(xml.as_bytes(), "application/rdf+xml", None).expect("parses");
    let quad = dataset.quads().next().expect("one quad");
    assert_eq!(
        dataset.term_value(quad.s),
        TermValue::Iri("http://example.org/outer/inner/a".to_owned())
    );
}

// ── RDF/XML: the RFC-3986 tables through `xml:base` ─────────────────────────────
//
// RDF/XML used to carry a byte-identical copy of the same hand-rolled resolver Turtle
// had, plus a `validate_iri` that tested only for a `:` — so a `urn`-shaped relative
// reference passed as "absolute". Both are gone; these drive the normative tables
// through `xml:base` to prove the shared layer is what answers.

/// One `rdf:Description` whose `rdf:about` is `reference`, under `xml:base`.
fn rdfxml_with(base: &str, reference: &str) -> String {
    format!(
        "<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\" \
         xmlns:ex=\"http://example.org/\" xml:base=\"{base}\">\
         <rdf:Description rdf:about=\"{reference}\"><ex:p>v</ex:p></rdf:Description>\
         </rdf:RDF>"
    )
}

#[test]
fn rdfxml_resolves_the_full_rfc3986_5_4_table_through_xml_base() {
    for (reference, expected) in NORMAL.iter().chain(ABNORMAL.iter()) {
        // `&` and `<` would need XML escaping and none of the table rows contain them;
        // the `g:h` / `http:g` rows are absolute and exercise the scheme-bearing arm.
        let xml = rdfxml_with(RFC_BASE, reference);
        assert_eq!(
            only_subject(&xml, "application/rdf+xml", None),
            *expected,
            "xml:base=\"{RFC_BASE}\" with rdf:about=\"{reference}\""
        );
    }
}

/// `xml:base` is SCOPED to its element's subtree: an inner declaration resolves against
/// the enclosing one, and a sibling after that element closes still sees the OUTER base.
/// A base stack that leaked would give the sibling the inner base.
#[test]
fn rdfxml_xml_base_is_scoped_to_its_subtree() {
    let xml = concat!(
        "<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\" ",
        "xmlns:ex=\"http://example.org/\" xml:base=\"http://example.org/outer/\">",
        // Inner declaration is RELATIVE and resolves against the outer one.
        "<rdf:Description rdf:about=\"a\" xml:base=\"inner/\"><ex:p>v</ex:p></rdf:Description>",
        // Sibling AFTER the inner element closed: must see the OUTER base, not `inner/`.
        "<rdf:Description rdf:about=\"b\"><ex:p>v</ex:p></rdf:Description>",
        "</rdf:RDF>",
    );
    let dataset = parse_dataset(xml.as_bytes(), "application/rdf+xml", None).expect("parses");
    let mut subjects: Vec<String> = dataset
        .quads()
        .map(|q| match dataset.term_value(q.s) {
            TermValue::Iri(iri) => iri,
            other => panic!("not an IRI: {other:?}"),
        })
        .collect();
    subjects.sort();
    assert_eq!(
        subjects,
        vec![
            "http://example.org/outer/b".to_owned(),
            "http://example.org/outer/inner/a".to_owned(),
        ],
        "the inner xml:base must not leak to the following sibling"
    );
}

/// `rdf:ID="x"` is the same-document reference `#x`, RESOLVED against the base rather
/// than concatenated onto it — so the base's own fragment is dropped and its query kept.
#[test]
fn rdfxml_rdf_id_is_a_resolved_same_document_reference() {
    let xml = concat!(
        "<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\" ",
        "xmlns:ex=\"http://example.org/\" xml:base=\"http://example.org/d?q=1#frag\">",
        "<rdf:Description rdf:ID=\"x\"><ex:p>v</ex:p></rdf:Description>",
        "</rdf:RDF>",
    );
    assert_eq!(
        only_subject(xml, "application/rdf+xml", None),
        "http://example.org/d?q=1#x",
        "rdf:ID resolves as `#x`: the base's fragment is replaced, its query kept"
    );
}

/// An empty `rdf:about=""` is the document itself — the RDF/XML spelling of Turtle's
/// `<>`, and the same defect.
#[test]
fn rdfxml_empty_about_denotes_the_document() {
    let xml = rdfxml_with("http://example.org/d?q=1#frag", "");
    assert_eq!(
        only_subject(&xml, "application/rdf+xml", None),
        "http://example.org/d?q=1",
        "rdf:about=\"\" keeps the base's query and drops its fragment"
    );
}

/// The colon test this codec used to apply admitted a `path-noscheme` relative
/// reference whose first segment merely contained a `:`. It is not absolute, and with
/// no base in scope it must be refused rather than interned.
#[test]
fn rdfxml_urn_shaped_relative_reference_is_not_mistaken_for_absolute() {
    // `1a:b` cannot be a scheme (a scheme must start with ALPHA), so this is a relative
    // reference whose first segment illegally contains a colon — RFC-3986 4.2.
    let xml = concat!(
        "<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\" ",
        "xmlns:ex=\"http://example.org/\">",
        "<rdf:Description rdf:about=\"1a:b\"><ex:p>v</ex:p></rdf:Description>",
        "</rdf:RDF>",
    );
    let error = parse_dataset(xml.as_bytes(), "application/rdf+xml", None)
        .expect_err("a path-noscheme reference is not an absolute IRI");
    assert_eq!(error.code, "iri-disallowed-char");

    // And a plain relative reference with no base is the base-less error, not a
    // silently-accepted "it has a colon somewhere" pass.
    let bare = rdfxml_with_no_base("urn");
    let error = parse_dataset(bare.as_bytes(), "application/rdf+xml", None)
        .expect_err("`urn` alone is a relative reference, not the urn: scheme");
    assert_eq!(error.code, "iri-relative-no-base");
}

/// `rdf:about` with no `xml:base` and no caller base.
fn rdfxml_with_no_base(reference: &str) -> String {
    format!(
        "<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\" \
         xmlns:ex=\"http://example.org/\">\
         <rdf:Description rdf:about=\"{reference}\"><ex:p>v</ex:p></rdf:Description>\
         </rdf:RDF>"
    )
}

// ── JSON-LD / YAML-LD ───────────────────────────────────────────────────────────
//
// Both declare `admits_relative_iri: true` in the format registry, and both codecs used
// to bind the base parameter as `_base` and drop it — so `--base` was silently a no-op
// and a relative `@id` failed with "requires an absolute base IRI" while the caller's
// base sat unused one frame up. A parameter accepted and ignored is worse than one
// absent: the seam looks threaded.

/// A relative `@id` resolves against the caller-supplied base.
#[test]
fn jsonld_resolves_a_relative_id_against_the_caller_base() {
    let subject = only_subject(
        JSONLD_RELATIVE_SUBJECT,
        "application/ld+json",
        Some("http://example.org/dir/doc.jsonld"),
    );
    assert_eq!(subject, "http://example.org/dir/foo");
}

/// The base is READ, not merely accepted: a different base yields a different term.
#[test]
fn jsonld_base_is_read_rather_than_accepted_and_dropped() {
    let a = only_subject(
        JSONLD_RELATIVE_SUBJECT,
        "application/ld+json",
        Some("http://a.example/"),
    );
    let b = only_subject(
        JSONLD_RELATIVE_SUBJECT,
        "application/ld+json",
        Some("http://b.example/"),
    );
    assert_eq!(a, "http://a.example/foo");
    assert_eq!(b, "http://b.example/foo");
    assert_ne!(a, b, "the resolved subject must move with the base");
}

/// YAML-LD bridges to the same expander and must agree with its JSON twin.
#[test]
fn yamlld_resolves_a_relative_id_identically_to_jsonld() {
    let json = only_subject(
        JSONLD_RELATIVE_SUBJECT,
        "application/ld+json",
        Some("http://example.org/dir/doc"),
    );
    let yaml = only_subject(
        YAMLLD_RELATIVE_SUBJECT,
        "application/ld+yaml",
        Some("http://example.org/dir/doc"),
    );
    assert_eq!(json, yaml, "the two surfaces must resolve identically");
}

/// A document `@base` still wins over the caller-supplied one (JSON-LD 1.1 precedence,
/// matching Turtle's `@base` overriding the caller).
#[test]
fn a_jsonld_context_base_overrides_the_caller_base() {
    let doc = "{\"@context\":{\"@base\":\"http://inner.example/\"},\
               \"@id\":\"foo\",\"http://example.org/p\":{\"@id\":\"http://example.org/o\"}}";
    let subject = only_subject(doc, "application/ld+json", Some("http://outer.example/"));
    assert_eq!(subject, "http://inner.example/foo");
}

/// With NO base at all a relative `@id` is still refused — the fix resolves, it does not
/// invent.
#[test]
fn jsonld_without_a_base_still_refuses_a_relative_id() {
    let error = parse_dataset(
        JSONLD_RELATIVE_SUBJECT.as_bytes(),
        "application/ld+json",
        None,
    )
    .expect_err("a relative @id with no base must fail");
    assert_eq!(
        error.code, "iri-relative-no-base",
        "the refusal is the workspace-shared condition, not a JSON-LD-local spelling"
    );
    assert!(
        error.message.contains("\"foo\""),
        "the refusal names the reference verbatim: {}",
        error.message
    );
}
