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
//!
//! # The egress leg
//!
//! The serialize leg mirrors all of it, keyed on the registry's `emits_base` column: a
//! syntax that can express a document base emits the directive and relativizes against
//! it; one that cannot never applies the base and emits absolute IRIs. The egress
//! totality test drives every registered format off that column for the same reason the
//! ingress one drives every format off `admits_relative_iri` — a capability column is
//! only a contract if every row is read.

use purrdf_iri::BaseOrigin;
use purrdf_rdf::native_codecs::{
    NativeRdfFormat, parse_dataset, serialize_dataset_to_format,
    serialize_dataset_to_format_with_jsonld_options, transcode_under_document_base,
};
use purrdf_rdf::{
    JsonLdSerializeOptions, ParseOptions, SerializeGraph, TermValue, datasets_isomorphic,
    parse_dataset_from_reader, parse_dataset_with, serialize_dataset,
};

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

/// The TriX spelling of a one-triple document whose subject is the relative `foo`.
const TRIX_RELATIVE_SUBJECT: &str = concat!(
    "<TriX xmlns=\"http://www.w3.org/2004/03/trix/trix-1/\"><graph><triple>",
    "<uri>foo</uri><uri>http://example.org/p</uri><uri>http://example.org/o</uri>",
    "</triple></graph></TriX>",
);

/// The `HexTuples` spelling of the same one-triple document.
const HEXTUPLES_RELATIVE_SUBJECT: &str =
    "[\"foo\",\"http://example.org/p\",\"http://example.org/o\",\"globalId\",\"\",\"\"]\n";

/// [`JSONLD_RELATIVE_SUBJECT`] with an in-document `@context` `@base`.
const JSONLD_RELATIVE_SUBJECT_WITH_CONTEXT_BASE: &str = concat!(
    "{\"@context\":{\"@base\":\"http://inner.example/\"},",
    "\"@id\":\"foo\",\"http://example.org/p\":{\"@id\":\"http://example.org/o\"}}",
);

/// The YAML-LD spelling of [`JSONLD_RELATIVE_SUBJECT_WITH_CONTEXT_BASE`].
const YAMLLD_RELATIVE_SUBJECT_WITH_CONTEXT_BASE: &str = concat!(
    "\"@context\":\n",
    "  \"@base\": \"http://inner.example/\"\n",
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

/// An absolute-only grammar must refuse a relative reference with the same code WHETHER
/// OR NOT a base is supplied — the base is never applied to it.
///
/// The line family is spelled out here because the `base is irrelevant` half needs both
/// arms; TriX and HexTuples get the same treatment from the totality test below, which
/// drives every registered format rather than a hand list.
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
        // EVERY registered format is spelled — the match is exhaustive with no `_`
        // arm, so adding a codec to `FORMATS` fails to compile here until somebody
        // writes its one-triple spelling. It cannot be waived by omission.
        //
        // JSON-LD and YAML-LD used to be skipped, and that skip is why the dropped base
        // survived: both declare `admits_relative_iri: true`, both codecs bound the base
        // and threw it away, and the caller's `--base` was a silent no-op for exactly the
        // formats this loop had waived. RDF/XML, TriX and HexTuples were waived beside
        // them. A capability column is only a contract if the totality test reads it for
        // every row.
        let text = match format {
            NativeRdfFormat::Turtle | NativeRdfFormat::TriG => {
                format!("<foo> {P} {O} .\n")
            }
            NativeRdfFormat::NTriples | NativeRdfFormat::NQuads => {
                format!("<foo> {P} {O} .\n")
            }
            NativeRdfFormat::JsonLd => JSONLD_RELATIVE_SUBJECT.to_owned(),
            NativeRdfFormat::YamlLd => YAMLLD_RELATIVE_SUBJECT.to_owned(),
            NativeRdfFormat::RdfXml => rdfxml_with_no_base("foo"),
            NativeRdfFormat::TriX => TRIX_RELATIVE_SUBJECT.to_owned(),
            NativeRdfFormat::HexTuples => HEXTUPLES_RELATIVE_SUBJECT.to_owned(),
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

// ── Egress: the `emits_base` column, over every registered format ───────────────
//
// The parse leg resolved a relative reference against a base and the serialize leg threw
// the same base away, so `<>` could be read in and never written back out. `--base` is
// one flag serving both legs; these drive the egress half off the registry the way the
// tests above drive the ingress half.

/// The base every egress case is written under, and a document under it.
///
/// `IN_DIR` sits inside the base's directory (relative spelling `a`); `SIBLING` sits
/// beside it (`../dir2/b`); `PREDICATE` sits one level up (`../p`). Three different
/// relative shapes, so a codec that relativized only the subject is visible.
const EGRESS_BASE: &str = "http://example.org/dir/";
const EGRESS_IN_DIR: &str = "http://example.org/dir/a";
const EGRESS_SIBLING: &str = "http://example.org/dir2/b";
const EGRESS_PREDICATE: &str = "http://example.org/p";

/// The one-quad fixture every egress case serializes, as N-Triples source.
fn egress_source() -> String {
    format!("<{EGRESS_IN_DIR}> <{EGRESS_PREDICATE}> <{EGRESS_SIBLING}> .\n")
}

/// The base directive `format` writes, for a format that can express one.
///
/// Spelled per syntax rather than searched for loosely, so the assertion is that the
/// document declares its base in the place its own grammar reads a base FROM — not
/// merely that the base string appears somewhere in the bytes.
fn base_directive(format: NativeRdfFormat) -> Option<String> {
    match format {
        NativeRdfFormat::Turtle | NativeRdfFormat::TriG => Some(format!("@base <{EGRESS_BASE}> .")),
        NativeRdfFormat::RdfXml => Some(format!("xml:base=\"{EGRESS_BASE}\"")),
        NativeRdfFormat::JsonLd => Some(format!("\"@base\": \"{EGRESS_BASE}\"")),
        NativeRdfFormat::YamlLd => Some(format!("'@base': {EGRESS_BASE}")),
        NativeRdfFormat::NTriples
        | NativeRdfFormat::NQuads
        | NativeRdfFormat::TriX
        | NativeRdfFormat::HexTuples => None,
    }
}

/// EVERY registered format's egress base behaviour, read off `emits_base()`.
///
/// The match over [`NativeRdfFormat`] is exhaustive with no `_` arm, so a codec added to
/// the registry cannot reach `main` until somebody states here what base directive it
/// writes — or states that it writes none. The alternative shape, a hand list or a
/// `continue` past the awkward formats, is exactly how the ingress leg's JSON-LD hole
/// survived: a capability column that no test reads for every row is a comment.
#[test]
fn every_format_emits_a_base_exactly_when_its_capability_column_says_so() {
    let dataset = parse_dataset(
        egress_source().as_bytes(),
        NativeRdfFormat::NTriples.media_type(),
        None,
    )
    .expect("absolute fixture parses");

    for format in NativeRdfFormat::all() {
        let outcome = serialize_dataset_to_format(&dataset, format, Some(EGRESS_BASE))
            .unwrap_or_else(|e| panic!("{format:?}: serializing under a base failed: {e}"));
        let text = String::from_utf8(outcome.bytes).expect("utf-8 output");

        match base_directive(format) {
            Some(directive) => {
                assert!(
                    format.emits_base(),
                    "{format:?} declares a base directive but emits_base() is false"
                );
                assert!(
                    text.contains(&directive),
                    "{format:?} must declare its base as `{directive}`, got:\n{text}"
                );
                // Relativized, not merely declared: the subject's absolute spelling is
                // gone from the document.
                assert!(
                    !text.contains(EGRESS_IN_DIR),
                    "{format:?} declares a base but still writes the absolute \
                     `{EGRESS_IN_DIR}`:\n{text}"
                );
            }
            None => {
                assert!(
                    !format.emits_base(),
                    "{format:?} declares no base directive but emits_base() is true"
                );
                // A syntax that cannot express a base NEVER applies one: every IRI is
                // absolute, exactly as it would be with no base supplied at all.
                for absolute in [EGRESS_IN_DIR, EGRESS_PREDICATE, EGRESS_SIBLING] {
                    assert!(
                        text.contains(absolute),
                        "{format:?} cannot express a base, so `{absolute}` must be \
                         written absolutely:\n{text}"
                    );
                }
                // Stated as byte identity rather than as "no directive appears": a
                // format with no base surface must produce the SAME document it would
                // have produced with no base at all, which forecloses a declaration, a
                // relativization, and a reordering in one assertion.
                let unbased = serialize_dataset_to_format(&dataset, format, None)
                    .expect("serializing with no base")
                    .bytes;
                assert_eq!(
                    text.as_bytes(),
                    unbased.as_slice(),
                    "{format:?} has no base surface, so `--base` must not change one \
                     byte of its output"
                );
            }
        }
    }
}

/// A document written under a base re-parses to the same graph with NO caller base: it
/// carries its own.
///
/// This is the round trip the missing serialize leg made impossible — `<>` and `<a>`
/// could be READ under a base and never WRITTEN back under one, so a based document was
/// a one-way door.
#[test]
fn every_base_emitting_format_round_trips_through_its_own_declaration() {
    let dataset = parse_dataset(
        egress_source().as_bytes(),
        NativeRdfFormat::NTriples.media_type(),
        None,
    )
    .expect("absolute fixture parses");

    for format in NativeRdfFormat::all().filter(|format| format.emits_base()) {
        let bytes = serialize_dataset_to_format(&dataset, format, Some(EGRESS_BASE))
            .unwrap_or_else(|e| panic!("{format:?}: serialize under a base failed: {e}"))
            .bytes;
        // NO base is supplied on the way back in: the document must carry its own.
        let reparsed = parse_dataset(&bytes, format.media_type(), None).unwrap_or_else(|e| {
            panic!(
                "{format:?}: a document written under a base must re-parse with no \
                 caller base: {e}\n{}",
                String::from_utf8_lossy(&bytes)
            )
        });
        assert!(
            datasets_isomorphic(&dataset, &reparsed),
            "{format:?}: the base round trip must be isomorphic:\n{}",
            String::from_utf8_lossy(&bytes)
        );
    }
}

/// A caller's JSON-LD context and the egress base COMPOSE: every declared term survives
/// into the emitted `@context` and the base joins it.
///
/// The JSON-LD family expresses its base as `@context.@base`, so a base and a caller
/// context contend for the same slot. Appending the base as a later context member is
/// JSON-LD 1.1's own composition, and this pins that it neither drops the caller's terms
/// nor leaves the base unapplied.
#[test]
fn a_caller_jsonld_context_and_the_egress_base_compose() {
    let dataset = parse_dataset(
        egress_source().as_bytes(),
        NativeRdfFormat::NTriples.media_type(),
        None,
    )
    .expect("absolute fixture parses");
    let options = JsonLdSerializeOptions::context(
        &serde_json::json!({"ex": {"@id": "http://example.org/", "@prefix": true}}),
        None,
    )
    .expect("caller context compiles");

    let bytes = serialize_dataset_to_format_with_jsonld_options(
        &dataset,
        NativeRdfFormat::JsonLd,
        Some(EGRESS_BASE),
        &options,
    )
    .expect("configured JSON-LD under a base")
    .bytes;
    let text = String::from_utf8(bytes.clone()).expect("utf-8");

    assert!(
        text.contains("\"@base\": \"http://example.org/dir/\""),
        "the egress base joins the emitted context:\n{text}"
    );
    assert!(
        text.contains("\"ex\""),
        "the caller's own term must survive into the emitted context:\n{text}"
    );
    assert!(
        !text.contains(EGRESS_IN_DIR),
        "the subject must be relativized against the base:\n{text}"
    );
    let reparsed = parse_dataset(&bytes, NativeRdfFormat::JsonLd.media_type(), None)
        .expect("the emitted document carries its own context and base");
    assert!(
        datasets_isomorphic(&dataset, &reparsed),
        "the composed round trip must be isomorphic:\n{text}"
    );
}

/// A context that ALREADY declares a base keeps it: the document's own base wins over the
/// caller-supplied one, the same precedence the parse leg applies to an in-document
/// `@context.@base`.
#[test]
fn a_context_that_declares_its_own_base_keeps_it_over_the_egress_base() {
    let dataset = parse_dataset(
        egress_source().as_bytes(),
        NativeRdfFormat::NTriples.media_type(),
        None,
    )
    .expect("absolute fixture parses");
    let options = JsonLdSerializeOptions::context(
        &serde_json::json!({"@base": "http://example.org/dir2/"}),
        None,
    )
    .expect("caller context with its own base compiles");

    let text = String::from_utf8(
        serialize_dataset_to_format_with_jsonld_options(
            &dataset,
            NativeRdfFormat::JsonLd,
            Some(EGRESS_BASE),
            &options,
        )
        .expect("configured JSON-LD under a base")
        .bytes,
    )
    .expect("utf-8");

    assert!(
        text.contains("\"@base\": \"http://example.org/dir2/\""),
        "the context's own base is the one in force:\n{text}"
    );
    assert!(
        !text.contains(EGRESS_BASE),
        "the caller-supplied base must not override the document's own:\n{text}"
    );
}

/// A base that is not an absolute IRI is a hard failure for EVERY format — including the
/// ones that would not have applied it.
///
/// Validating only where the base is used would tell a caller their `--base` was fine
/// merely because they happened to target N-Triples; the mistake is in the argument, and
/// it is reported as the shared `purrdf-iri` condition rather than a serializer-local
/// spelling.
#[test]
fn a_non_absolute_egress_base_is_refused_by_every_format() {
    let dataset = parse_dataset(
        egress_source().as_bytes(),
        NativeRdfFormat::NTriples.media_type(),
        None,
    )
    .expect("absolute fixture parses");

    for format in NativeRdfFormat::all() {
        let error = serialize_dataset_to_format(&dataset, format, Some("/not/absolute"))
            .expect_err("a relative base is not a base");
        assert_eq!(
            error.code, "iri-non-absolute-base",
            "{format:?} must refuse a non-absolute base with the shared condition"
        );
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
    let subject = only_subject(
        JSONLD_RELATIVE_SUBJECT_WITH_CONTEXT_BASE,
        "application/ld+json",
        Some("http://outer.example/"),
    );
    assert_eq!(subject, "http://inner.example/foo");
}

// The base-precedence trio, stated for YAML-LD in its own right rather than only by
// agreement with JSON-LD. The YAML→JSON bridge is structural, so the two surfaces should
// answer identically — but "should" is the claim under test, and a bridge that dropped
// the base would still agree with a JSON-LD path that had also dropped it.

/// An external base resolves a relative `@id`, and is READ rather than merely accepted:
/// the resolved subject moves with the base.
#[test]
fn yamlld_resolves_a_relative_id_against_the_caller_base() {
    let a = only_subject(
        YAMLLD_RELATIVE_SUBJECT,
        "application/ld+yaml",
        Some("http://a.example/"),
    );
    let b = only_subject(
        YAMLLD_RELATIVE_SUBJECT,
        "application/ld+yaml",
        Some("http://b.example/"),
    );
    assert_eq!(a, "http://a.example/foo");
    assert_eq!(b, "http://b.example/foo");
    assert_ne!(a, b, "the resolved subject must move with the base");
}

/// With NO base at all, YAML-LD refuses exactly as JSON-LD does — the bridge carries the
/// absence of a base as faithfully as it carries one.
#[test]
fn yamlld_without_a_base_still_refuses_a_relative_id() {
    let error = parse_dataset(
        YAMLLD_RELATIVE_SUBJECT.as_bytes(),
        "application/ld+yaml",
        None,
    )
    .expect_err("a relative @id with no base must fail");
    assert_eq!(
        error.code, "iri-relative-no-base",
        "the refusal is the workspace-shared condition, not a YAML-LD-local spelling"
    );
    assert!(
        error.message.contains("\"foo\""),
        "the refusal names the reference verbatim: {}",
        error.message
    );
}

/// An in-document `@context` `@base` beats the external base here too.
#[test]
fn a_yamlld_context_base_overrides_the_caller_base() {
    let subject = only_subject(
        YAMLLD_RELATIVE_SUBJECT_WITH_CONTEXT_BASE,
        "application/ld+yaml",
        Some("http://outer.example/"),
    );
    assert_eq!(subject, "http://inner.example/foo");
}

/// The two surfaces agree on the precedence itself, not merely on the simple case.
#[test]
fn jsonld_and_yamlld_agree_on_context_base_precedence() {
    assert_eq!(
        only_subject(
            JSONLD_RELATIVE_SUBJECT_WITH_CONTEXT_BASE,
            "application/ld+json",
            Some("http://outer.example/"),
        ),
        only_subject(
            YAMLLD_RELATIVE_SUBJECT_WITH_CONTEXT_BASE,
            "application/ld+yaml",
            Some("http://outer.example/"),
        ),
    );
}

// ── The vocab position ──────────────────────────────────────────────────────────
//
// JSON-LD 1.1 Expansion §13.4.4 expands `@type` with BOTH `vocab` and `documentRelative`
// set, so a relative `@type` falls back to the document base when no `@vocab` is in
// scope. The codec passed `false` for the second flag, which made `@type` the ONE IRI
// position that could not see a base the document itself declared: the sibling `@id` in
// the very same document resolved, so the base was demonstrably in scope and simply not
// consulted. The upstream W3C `toRdf` vectors pinned under
// `tests/fixtures/jsonld-w3c-rec/` reach the base only through `@id` and `@type: @id`
// coercion (the `0120`-`0132` IRI-resolution family), never through a bare vocab-position
// term — which is exactly why the defect survived a 73/73 conformance pass, and why these
// direct tests exist.

/// A relative `@type` resolves against the document's own `@context` `@base`.
///
/// This is the library form of the production reproduction
/// `purrdf convert --from jsonld --to ntriples -`, which used to exit non-zero with
/// `jsonld-context-invalid: relative IRI `Thing` has no applicable @vocab or @base`.
const JSONLD_RELATIVE_TYPE_WITH_CONTEXT_BASE: &str =
    "{\"@context\":{\"@base\":\"http://example.org/\"},\"@id\":\"x\",\"@type\":\"Thing\"}";

/// The YAML-LD spelling of [`JSONLD_RELATIVE_TYPE_WITH_CONTEXT_BASE`].
const YAMLLD_RELATIVE_TYPE_WITH_CONTEXT_BASE: &str = concat!(
    "\"@context\":\n",
    "  \"@base\": \"http://example.org/\"\n",
    "\"@id\": x\n",
    "\"@type\": Thing\n",
);

/// The N-Triples both of the above must produce.
const RELATIVE_TYPE_EXPECTED: &str = concat!(
    "<http://example.org/x> ",
    "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type> ",
    "<http://example.org/Thing> .\n",
);

/// The whole document, as N-Triples, exactly as the CLI emits it.
fn ntriples_of(text: &str, media_type: &str, base: Option<&str>) -> String {
    let dataset = parse_dataset(text.as_bytes(), media_type, base)
        .unwrap_or_else(|e| panic!("parse {media_type} failed: {e}"));
    String::from_utf8(
        serialize_dataset(&dataset, "application/n-triples", SerializeGraph::Dataset)
            .expect("serialize N-Triples"),
    )
    .expect("utf-8")
}

#[test]
fn jsonld_relative_type_resolves_against_the_document_base() {
    assert_eq!(
        ntriples_of(
            JSONLD_RELATIVE_TYPE_WITH_CONTEXT_BASE,
            "application/ld+json",
            None,
        ),
        RELATIVE_TYPE_EXPECTED,
    );
}

/// The base is READ in vocab position, not merely present: the resolved type moves with
/// the caller-supplied base just as the subject does.
#[test]
fn jsonld_relative_type_resolves_against_the_caller_base() {
    let document = "{\"@id\":\"x\",\"@type\":\"Thing\"}";
    for base in ["http://a.example/", "http://b.example/"] {
        assert_eq!(
            ntriples_of(document, "application/ld+json", Some(base)),
            format!(
                "<{base}x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <{base}Thing> .\n"
            ),
            "@type must move with the base, exactly as @id does",
        );
    }
}

/// The asymmetry that made the defect visible: `@id` and `@type` sit in the same document
/// under the same base, so they must either BOTH resolve or BOTH refuse. They must never
/// disagree about whether a base is in scope.
#[test]
fn jsonld_id_and_type_agree_about_whether_a_base_is_in_scope() {
    let document = "{\"@id\":\"x\",\"@type\":\"Thing\"}";
    let with_base = parse_dataset(
        document.as_bytes(),
        "application/ld+json",
        Some("http://example.org/"),
    );
    let without_base = parse_dataset(document.as_bytes(), "application/ld+json", None);
    assert!(with_base.is_ok(), "a base in scope resolves both positions");
    assert!(
        without_base.is_err(),
        "no base in scope refuses both positions",
    );
}

/// An `@vocab` still wins over the base in vocab position — the document-relative leg is a
/// FALLBACK, not a replacement for vocabulary expansion.
#[test]
fn a_jsonld_vocab_still_beats_the_base_in_vocab_position() {
    let document = concat!(
        "{\"@context\":{\"@base\":\"http://base.example/\",",
        "\"@vocab\":\"http://vocab.example/\"},",
        "\"@id\":\"x\",\"@type\":\"Thing\"}",
    );
    assert_eq!(
        ntriples_of(document, "application/ld+json", None),
        concat!(
            "<http://base.example/x> ",
            "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type> ",
            "<http://vocab.example/Thing> .\n",
        ),
        "@id takes the base and @type takes the vocab",
    );
}

/// A value object's `@type` is the same §13.4.4 position, so a relative datatype resolves
/// against the document base too.
#[test]
fn a_jsonld_value_object_datatype_resolves_against_the_document_base() {
    let document = concat!(
        "{\"@context\":{\"@base\":\"http://example.org/\"},\"@id\":\"x\",",
        "\"http://example.org/p\":{\"@value\":\"v\",\"@type\":\"MyType\"}}",
    );
    assert_eq!(
        ntriples_of(document, "application/ld+json", None),
        "<http://example.org/x> <http://example.org/p> \"v\"^^<http://example.org/MyType> .\n",
    );
}

/// YAML-LD bridges to the same expander and must agree in vocab position too.
#[test]
fn yamlld_relative_type_resolves_identically_to_jsonld() {
    assert_eq!(
        ntriples_of(
            YAMLLD_RELATIVE_TYPE_WITH_CONTEXT_BASE,
            "application/ld+yaml",
            None,
        ),
        ntriples_of(
            JSONLD_RELATIVE_TYPE_WITH_CONTEXT_BASE,
            "application/ld+json",
            None,
        ),
    );
}

/// With no base and no `@vocab`, the refusal is the WORKSPACE-SHARED condition. It used to
/// be an eighth private spelling — `jsonld-context-invalid: relative IRI `Thing` has no
/// applicable @vocab or @base` — so a caller grepping for `iri-relative-no-base` believed
/// JSON-LD had no such failure mode in this position.
#[test]
fn jsonld_without_a_base_refuses_a_relative_type_with_the_shared_code() {
    let error = parse_dataset(
        "{\"@id\":\"http://example.org/x\",\"@type\":\"Thing\"}".as_bytes(),
        "application/ld+json",
        None,
    )
    .expect_err("a relative @type with no base and no @vocab must fail");
    assert_eq!(
        error.code, "iri-relative-no-base",
        "the refusal is the workspace-shared condition, not a JSON-LD-local spelling",
    );
    assert!(
        error.message.contains("\"Thing\""),
        "the refusal names the reference verbatim: {}",
        error.message,
    );
}

/// The vocab-ONLY positions — a term definition's `@id` — are a DIFFERENT condition, and
/// keep saying so. The base is never consulted there, so reporting `iri-relative-no-base`
/// would send an author off to add a `@base` that cannot help; the message must name the
/// remedy that works and must not mention `@base` as if it applied.
#[test]
fn a_vocab_only_position_is_not_reported_as_a_missing_base() {
    let error = parse_dataset(
        concat!(
            "{\"@context\":{\"@base\":\"http://example.org/\",\"t\":{\"@id\":\"bar\"}},",
            "\"@id\":\"http://example.org/x\",\"t\":\"v\"}",
        )
        .as_bytes(),
        "application/ld+json",
        None,
    )
    .expect_err("a term @id that no @vocab can expand must fail");
    assert_eq!(error.code, "jsonld-context-invalid");
    assert_ne!(
        error.code, "iri-relative-no-base",
        "a base is in scope and still cannot fix this, so it is not the no-base condition",
    );
    assert!(
        error.message.contains("@vocab"),
        "the message names the remedy that works: {}",
        error.message,
    );
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

// ── The parse leg reports the base the DOCUMENT ended up under ───────────────────
//
// The third leg of the base contract, and the one that was missing. A caller could
// supply a base on the way IN and (since the serializer base leg) a base on the way OUT,
// but could not ask what base the DOCUMENT ended up under — so a round trip that
// preserves a document's own `@base` could not be written without re-reading the source
// text by hand, which is a second, drifting base parser in every consumer.
//
// `ParseOutcome::document_base` is that answer, carrying `BaseOrigin` so the caller can
// tell "the document said so" from "you said so" from "nobody said so".

/// An absolute-IRI TriX document, for the base-less-grammar sweep below.
const TRIX_ABSOLUTE: &str = concat!(
    "<TriX xmlns=\"http://www.w3.org/2004/03/trix/trix-1/\"><graph><triple>",
    "<uri>http://example.org/s</uri><uri>http://example.org/p</uri>",
    "<uri>http://example.org/o</uri>",
    "</triple></graph></TriX>",
);

/// The `HexTuples` spelling of [`TRIX_ABSOLUTE`].
const HEXTUPLES_ABSOLUTE: &str = concat!(
    "[\"http://example.org/s\",\"http://example.org/p\",",
    "\"http://example.org/o\",\"globalId\",\"\",\"\"]\n",
);

/// The document base and its provenance, for a document of `media_type`.
fn document_base(text: &str, media_type: &str, base: Option<&str>) -> Option<(String, BaseOrigin)> {
    parse_dataset_with(text.as_bytes(), media_type, base, &ParseOptions::default())
        .unwrap_or_else(|e| panic!("parse {media_type} failed: {e}"))
        .document_base
        .map(|scoped| (scoped.iri().as_str().to_owned(), scoped.origin()))
}

/// A Turtle `@base` is reported, as a DIRECTIVE, at the position it was written.
#[test]
fn turtle_reports_its_own_base_directive_with_provenance() {
    let text = "@base <http://document.example/> .\n<foo> <http://example.org/p> <#o> .\n";
    let (iri, origin) = document_base(text, "text/turtle", None).expect("a base is in force");
    assert_eq!(iri, "http://document.example/");
    assert_eq!(
        origin,
        // Column 7 is the IRI token, not the `@` — the directive's position is where the
        // base VALUE was written, which is what a diagnostic wants to point at.
        BaseOrigin::Directive { line: 1, column: 7 },
        "the document declared it, and says where",
    );
}

/// `@base` rebinds, so the base in force at the END of the document is the LAST one — not
/// the first, and not a merge of the two.
#[test]
fn the_reported_base_is_the_one_in_force_at_the_end_of_the_document() {
    let text = concat!(
        "@base <http://first.example/> .\n",
        "<a> <http://example.org/p> <http://example.org/o> .\n",
        "@base <http://second.example/> .\n",
        "<b> <http://example.org/p> <http://example.org/o> .\n",
    );
    let (iri, origin) = document_base(text, "text/turtle", None).expect("a base is in force");
    assert_eq!(iri, "http://second.example/");
    assert_eq!(origin, BaseOrigin::Directive { line: 3, column: 7 });
}

/// A relative `@base` composes against the caller's, and the REPORTED base is the
/// composed one — the base references actually resolved against, not the raw directive.
#[test]
fn a_relative_base_directive_is_reported_already_composed() {
    let text = "@base <sub/> .\n<x> <http://example.org/p> <http://example.org/o> .\n";
    let (iri, _) = document_base(text, "text/turtle", Some("http://example.org/dir/"))
        .expect("a base is in force");
    assert_eq!(iri, "http://example.org/dir/sub/");
}

/// With no directive the caller's base is what is in force, and the origin says so — the
/// value is not `None` merely because the document stayed silent.
#[test]
fn a_caller_base_is_reported_as_the_callers() {
    let text = "<x> <http://example.org/p> <http://example.org/o> .\n";
    let (iri, origin) =
        document_base(text, "text/turtle", Some("http://caller.example/")).expect("caller base");
    assert_eq!(iri, "http://caller.example/");
    assert_eq!(origin, BaseOrigin::Caller);
}

/// `None` is reserved for a document that truly ended under no base at all.
#[test]
fn no_base_anywhere_is_reported_as_none() {
    let text = "<http://example.org/s> <http://example.org/p> <http://example.org/o> .\n";
    assert_eq!(document_base(text, "application/n-triples", None), None);
}

/// A syntax with no base directive reports the caller's base unchanged — its silence is
/// an answer, not a gap.
#[test]
fn a_base_less_grammar_still_reports_the_caller_base() {
    let lines = "<http://example.org/s> <http://example.org/p> <http://example.org/o> .\n";
    let cases: &[(&str, &str)] = &[
        ("application/n-triples", lines),
        ("application/n-quads", lines),
        ("application/trix", TRIX_ABSOLUTE),
        ("application/x-hextuples", HEXTUPLES_ABSOLUTE),
    ];
    for (media_type, document) in cases {
        assert_eq!(
            document_base(document, media_type, Some("http://caller.example/")),
            Some(("http://caller.example/".to_owned(), BaseOrigin::Caller)),
            "{media_type} must pass the caller base through untouched",
        );
    }
}

/// RDF/XML reports the ROOT `xml:base`, which is the document base.
#[test]
fn rdfxml_reports_the_root_xml_base() {
    let text = concat!(
        "<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\" ",
        "xmlns:ex=\"http://example.org/\" xml:base=\"http://document.example/\">",
        "<rdf:Description rdf:about=\"foo\"><ex:p rdf:resource=\"http://example.org/o\"/>",
        "</rdf:Description></rdf:RDF>",
    );
    let (iri, origin) =
        document_base(text, "application/rdf+xml", None).expect("a base is in force");
    assert_eq!(iri, "http://document.example/");
    assert_eq!(origin, BaseOrigin::Enclosing);
}

/// An `xml:base` on an inner element governs that SUBTREE, never the document, so it must
/// not be reported as the document base. Reporting it would hand a round trip a base that
/// never applied to most of the file.
#[test]
fn rdfxml_does_not_report_a_subtree_xml_base_as_the_documents() {
    let text = concat!(
        "<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\" ",
        "xmlns:ex=\"http://example.org/\" xml:base=\"http://outer.example/\">",
        "<rdf:Description rdf:about=\"a\" xml:base=\"http://inner.example/\">",
        "<ex:p rdf:resource=\"http://example.org/o\"/>",
        "</rdf:Description></rdf:RDF>",
    );
    // The inner base really is applied to the subtree it governs...
    assert_eq!(
        only_subject(text, "application/rdf+xml", None),
        "http://inner.example/a"
    );
    // ...and the DOCUMENT base is still the root's.
    let (iri, _) = document_base(text, "application/rdf+xml", None).expect("a base is in force");
    assert_eq!(iri, "http://outer.example/");
}

/// A JSON-LD `@context` `@base` is the document's own word on the matter, reported as the
/// context frame it is.
#[test]
fn jsonld_reports_its_context_base() {
    let (iri, origin) = document_base(
        JSONLD_RELATIVE_SUBJECT_WITH_CONTEXT_BASE,
        "application/ld+json",
        Some("http://caller.example/"),
    )
    .expect("a base is in force");
    assert_eq!(iri, "http://inner.example/", "the document's base wins");
    assert_eq!(origin, BaseOrigin::Enclosing);
}

/// YAML-LD bridges to the same expander and must report identically.
#[test]
fn yamlld_reports_its_context_base_identically_to_jsonld() {
    assert_eq!(
        document_base(
            YAMLLD_RELATIVE_SUBJECT_WITH_CONTEXT_BASE,
            "application/ld+yaml",
            Some("http://caller.example/"),
        ),
        document_base(
            JSONLD_RELATIVE_SUBJECT_WITH_CONTEXT_BASE,
            "application/ld+json",
            Some("http://caller.example/"),
        ),
    );
}

/// A JSON-LD `"@base": null` CLEARS the caller's base, and the report says so with `None`
/// rather than echoing a base that is no longer in force.
#[test]
fn a_jsonld_null_base_clears_the_caller_base_in_the_report() {
    let document = concat!(
        "{\"@context\":{\"@base\":null},\"@id\":\"http://example.org/x\",",
        "\"http://example.org/p\":\"v\"}",
    );
    assert_eq!(
        document_base(
            document,
            "application/ld+json",
            Some("http://caller.example/")
        ),
        None,
        "`@base: null` is the document saying there is no base",
    );
}

/// A document that declares nothing leaves the caller's base — and its `Caller`
/// provenance — untouched.
#[test]
fn a_jsonld_document_without_a_context_base_keeps_the_callers_provenance() {
    assert_eq!(
        document_base(
            JSONLD_RELATIVE_SUBJECT,
            "application/ld+json",
            Some("http://caller.example/"),
        ),
        Some(("http://caller.example/".to_owned(), BaseOrigin::Caller)),
    );
}

// ── The consumer: a round trip under the document's own base ────────────────────
//
// A producer with no consumer is not done. `transcode_under_document_base` is the join:
// the parse leg's answer becomes the serialize leg's base, so a document's own declared
// base survives a transcode instead of being flattened to absolute IRIs.

/// A Turtle document declaring its own `@base` comes back out under that same base — the
/// directive is re-emitted and the references stay relative — with NO base supplied by
/// the caller anywhere in the round trip.
#[test]
fn a_document_declared_base_survives_a_transcode_with_no_caller_base() {
    let source = concat!(
        "@base <http://document.example/dir/> .\n",
        "<a> <http://example.org/p> <b> .\n",
    );
    let bytes = transcode_under_document_base(
        source.as_bytes(),
        "text/turtle",
        NativeRdfFormat::Turtle,
        None,
    )
    .expect("transcode")
    .bytes;
    let text = String::from_utf8(bytes).expect("utf-8");
    assert!(
        text.contains("@base <http://document.example/dir/>"),
        "the document's own base must be re-declared: {text}",
    );
    assert!(
        text.contains("<a>") && text.contains("<b>"),
        "and the references must be relative to it again: {text}",
    );

    // The re-emitted document denotes the SAME graph, read back with no base at all.
    let original = parse_dataset(source.as_bytes(), "text/turtle", None).expect("original");
    let round_tripped = parse_dataset(text.as_bytes(), "text/turtle", None).expect("round trip");
    assert!(
        datasets_isomorphic(&original, &round_tripped),
        "a base-preserving transcode must not change what the document denotes",
    );
}

/// The base crosses SYNTAXES: a Turtle `@base` becomes a TriG `@base`, and a JSON-LD
/// `@context` `@base` becomes a Turtle `@base`. Neither is a special case in the
/// consumer — both come from the one `document_base` answer.
#[test]
fn the_document_base_crosses_syntaxes() {
    let turtle = concat!(
        "@base <http://document.example/dir/> .\n",
        "<a> <http://example.org/p> <b> .\n",
    );
    let trig = transcode_under_document_base(
        turtle.as_bytes(),
        "text/turtle",
        NativeRdfFormat::TriG,
        None,
    )
    .expect("turtle to trig")
    .bytes;
    assert!(
        String::from_utf8(trig)
            .expect("utf-8")
            .contains("@base <http://document.example/dir/>")
    );

    let jsonld = concat!(
        "{\"@context\":{\"@base\":\"http://document.example/dir/\"},",
        "\"@id\":\"a\",\"http://example.org/p\":{\"@id\":\"b\"}}",
    );
    let out = transcode_under_document_base(
        jsonld.as_bytes(),
        "application/ld+json",
        NativeRdfFormat::Turtle,
        None,
    )
    .expect("jsonld to turtle")
    .bytes;
    let text = String::from_utf8(out).expect("utf-8");
    assert!(
        text.contains("@base <http://document.example/dir/>"),
        "the JSON-LD context base must reach the Turtle directive: {text}",
    );
}

/// Without the document's base the same transcode emits ABSOLUTE IRIs — which is what
/// makes the test above a real difference rather than a coincidence of the fixture.
#[test]
fn the_same_transcode_without_the_document_base_emits_absolute_iris() {
    let source = concat!(
        "@base <http://document.example/dir/> .\n",
        "<a> <http://example.org/p> <b> .\n",
    );
    let dataset = parse_dataset(source.as_bytes(), "text/turtle", None).expect("parse");
    let plain = serialize_dataset_to_format(&dataset, NativeRdfFormat::Turtle, None)
        .expect("serialize with no base")
        .bytes;
    let plain = String::from_utf8(plain).expect("utf-8");
    assert!(
        !plain.contains("@base"),
        "no base means no directive: {plain}",
    );
    assert!(
        plain.contains("<http://document.example/dir/a>"),
        "and absolute IRIs: {plain}",
    );
}

/// A caller base is a FALLBACK, not an override: a document that declares its own base
/// still comes back out under the document's.
#[test]
fn the_document_base_beats_the_caller_base_through_the_transcode() {
    let source = "@base <http://document.example/> .\n<a> <http://example.org/p> <b> .\n";
    let text = String::from_utf8(
        transcode_under_document_base(
            source.as_bytes(),
            "text/turtle",
            NativeRdfFormat::Turtle,
            Some("http://caller.example/"),
        )
        .expect("transcode")
        .bytes,
    )
    .expect("utf-8");
    assert!(text.contains("@base <http://document.example/>"), "{text}");
    assert!(!text.contains("caller.example"), "{text}");
}

/// With no base anywhere the transcode is exactly an ordinary one — the seam adds no
/// base of its own.
#[test]
fn a_transcode_with_no_base_anywhere_invents_none() {
    let source = "<http://example.org/s> <http://example.org/p> <http://example.org/o> .\n";
    let text = String::from_utf8(
        transcode_under_document_base(
            source.as_bytes(),
            "application/n-triples",
            NativeRdfFormat::Turtle,
            None,
        )
        .expect("transcode")
        .bytes,
    )
    .expect("utf-8");
    assert!(!text.contains("@base"), "no base was ever in force: {text}");
}

// ── The refusal must not lie about the base ─────────────────────────────────────
//
// `iri-not-absolute-by-grammar` says "supplying a base will not help", which is true —
// but the codecs raising it built a LOCAL `BaseScope::empty()` at the call site instead
// of passing the caller's scope down, so the message also said "no base IRI is in scope"
// to a user who had just run `--base http://example.org/dir/`. A diagnostic that denies
// the state of the world sends the reader hunting for a dropped parameter instead of at
// the relative IRI in their document. The base is still never APPLIED; it is now VISIBLE.

/// The one-triple document each absolute-only grammar spells with a relative subject.
const RELATIVE_SUBJECT_BY_FORMAT: &[(&str, &str)] = &[
    (
        "application/n-triples",
        "<foo> <http://example.org/p> <http://example.org/o> .\n",
    ),
    (
        "application/n-quads",
        "<foo> <http://example.org/p> <http://example.org/o> .\n",
    ),
    ("application/trix", TRIX_RELATIVE_SUBJECT),
    ("application/x-hextuples", HEXTUPLES_RELATIVE_SUBJECT),
];

#[test]
fn an_absolute_only_refusal_names_the_base_in_scope_instead_of_denying_it() {
    for (media_type, text) in RELATIVE_SUBJECT_BY_FORMAT {
        let error = parse_dataset(text.as_bytes(), media_type, Some("http://example.org/dir/"))
            .expect_err("a relative reference is refused whatever the base");
        assert_eq!(error.code, "iri-not-absolute-by-grammar", "{media_type}");
        assert!(
            error.message.contains("<http://example.org/dir/>"),
            "{media_type} must name the base in scope: {}",
            error.message,
        );
        assert!(
            error.message.contains("is never applied here"),
            "{media_type} must say the base is deliberately not applied: {}",
            error.message,
        );
        assert!(
            !error.message.contains("no base IRI is in scope"),
            "{media_type} must not deny a base the caller supplied: {}",
            error.message,
        );
    }
}

/// With NO base the same refusal says so — the fix makes the message TRACK the scope, it
/// does not simply delete the "absent" wording.
#[test]
fn an_absolute_only_refusal_still_reports_an_absent_base_as_absent() {
    for (media_type, text) in RELATIVE_SUBJECT_BY_FORMAT {
        let error = parse_dataset(text.as_bytes(), media_type, None)
            .expect_err("a relative reference is refused with no base either");
        assert_eq!(error.code, "iri-not-absolute-by-grammar", "{media_type}");
        assert!(
            error.message.contains("no base IRI is in scope"),
            "{media_type} with nothing in scope must say so: {}",
            error.message,
        );
    }
}

/// RDF/XML's one non-reference IRI position — an element name built from a RELATIVE
/// `xmlns:` namespace — refused with the same denial. It resolves against nothing, which
/// is exactly why the message has to name the base rather than claim there is none.
#[test]
fn an_rdfxml_qualified_name_refusal_names_the_base_in_scope() {
    let text = concat!(
        "<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\" ",
        "xmlns:ex=\"foo/\">",
        "<rdf:Description rdf:about=\"http://example.org/s\"><ex:p>v</ex:p>",
        "</rdf:Description></rdf:RDF>",
    );
    let error = parse_dataset(
        text.as_bytes(),
        "application/rdf+xml",
        Some("http://example.org/dir/"),
    )
    .expect_err("a relative xmlns namespace yields a relative element IRI");
    assert_eq!(error.code, "iri-not-absolute-by-grammar");
    assert!(
        error.message.contains("<http://example.org/dir/>")
            && !error.message.contains("no base IRI is in scope"),
        "the refusal must name the base in scope: {}",
        error.message,
    );
}

/// The STREAMING lane carried the same defect twice over: `parse_dataset_from_reader`
/// took a base and never built a scope from it at all, so every line-oriented refusal
/// reported "no base IRI is in scope" no matter what the caller passed. The streamed
/// message must be the buffered message, byte for byte — that is the whole contract
/// between the two lanes.
#[test]
fn the_streaming_lane_reports_the_same_base_as_the_buffered_lane() {
    for (media_type, text) in RELATIVE_SUBJECT_BY_FORMAT {
        if !matches!(
            *media_type,
            "application/n-triples" | "application/n-quads" | "application/x-hextuples"
        ) {
            continue; // only the line-oriented grammars stream
        }
        for base in [None, Some("http://example.org/dir/")] {
            let streamed = parse_dataset_from_reader(text.as_bytes(), media_type, base)
                .expect_err("the streamed parse refuses it too");
            let buffered = parse_dataset(text.as_bytes(), media_type, base)
                .expect_err("the buffered parse refuses it");
            assert_eq!(
                streamed.code, buffered.code,
                "{media_type} with base = {base:?}"
            );
            assert_eq!(
                streamed.message, buffered.message,
                "{media_type} with base = {base:?}: the two lanes must say the same thing"
            );
        }
    }
}
