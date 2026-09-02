// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The umbrella facade's DOCUMENT BASE contract.
//!
//! `.goals` is rust-first, so the Rust surface must be at least as capable as the
//! Python, WebAssembly, and C ones. Those three all carry a document base on both
//! their parse and their serialize entry points; this test pins that the facade does
//! too — from the facade ALONE, with no second dependency on `purrdf-iri` and no
//! reach into a sub-crate.
//!
//! Four things are asserted, matching the four the other surfaces are held to:
//!
//! 1. the ingress base resolves a relative reference;
//! 2. the egress base is written and relativized against, for the syntaxes whose
//!    registry row says they can express one;
//! 3. a syntax that cannot express one still ACCEPTS the base and answers with
//!    absolute IRIs, rather than erroring or silently swallowing it;
//! 4. absent or malformed, the base hard-fails with the shared `purrdf-iri`
//!    diagnostic code — no base is ever fabricated.

use purrdf::iri::{BaseIri, BaseOrigin, BaseScope, IriError, ScopedBase};
use purrdf::{NativeRdfFormat, RdfDataset, parse_dataset, serialize_dataset_to_format};
use std::sync::Arc;

/// A Turtle document whose subject is a relative reference and which declares no
/// `@base` of its own, so the caller's base is the only one that can be in scope.
const RELATIVE_TURTLE: &str = "<rel> <https://example.org/p> <https://example.org/o> .\n";

const BASE: &str = "https://example.org/base/";

fn parsed_under_base() -> Arc<RdfDataset> {
    parse_dataset(RELATIVE_TURTLE.as_bytes(), "text/turtle", Some(BASE))
        .expect("a relative reference resolves against the caller's base")
}

fn text(dataset: &RdfDataset, format: NativeRdfFormat, base: Option<&str>) -> String {
    let outcome = serialize_dataset_to_format(dataset, format, base)
        .expect("serialization through the facade");
    String::from_utf8(outcome.bytes).expect("the native codecs emit UTF-8")
}

#[test]
fn ingress_base_resolves_a_relative_reference() {
    let dataset = parsed_under_base();
    assert_eq!(dataset.quad_count(), 1);
    // The resolved subject is the base joined with the reference — the facade did the
    // RFC-3986 §5 resolution, not a string concatenation the caller had to do.
    let nt = text(&dataset, NativeRdfFormat::NTriples, None);
    assert!(
        nt.contains("<https://example.org/base/rel>"),
        "relative <rel> must resolve against the base, got: {nt}"
    );
}

#[test]
fn egress_base_is_emitted_by_every_format_whose_registry_row_says_so() {
    let dataset = parsed_under_base();
    // Turtle and TriG write `@base`; RDF/XML writes `xml:base`; JSON-LD and YAML-LD
    // write `@base` into the context. Each is the base spelling ITS grammar owns.
    for (format, marker) in [
        (NativeRdfFormat::Turtle, "@base <https://example.org/base/>"),
        (NativeRdfFormat::TriG, "@base <https://example.org/base/>"),
        (
            NativeRdfFormat::RdfXml,
            "xml:base=\"https://example.org/base/\"",
        ),
        (NativeRdfFormat::JsonLd, "https://example.org/base/"),
        (NativeRdfFormat::YamlLd, "https://example.org/base/"),
    ] {
        assert!(
            format.emits_base(),
            "{format:?} is asserted here because its registry row claims it emits a base"
        );
        let out = text(&dataset, format, Some(BASE));
        assert!(
            out.contains(marker),
            "{format:?} must declare the document base, got: {out}"
        );
    }
}

#[test]
fn a_base_incapable_format_answers_with_absolute_iris_rather_than_erroring() {
    let dataset = parsed_under_base();
    // The parameter is NOT swallowed: it is still read and still validated, and the
    // answer is the only spelling these grammars admit.
    for format in [
        NativeRdfFormat::NTriples,
        NativeRdfFormat::NQuads,
        NativeRdfFormat::TriX,
        NativeRdfFormat::HexTuples,
    ] {
        assert!(!format.emits_base());
        let out = text(&dataset, format, Some(BASE));
        assert!(
            out.contains("https://example.org/base/rel"),
            "{format:?} must emit the absolute IRI under a base, got: {out}"
        );
    }
}

#[test]
fn no_base_in_scope_is_a_hard_failure_with_the_shared_code() {
    let error = parse_dataset(RELATIVE_TURTLE.as_bytes(), "text/turtle", None)
        .expect_err("a relative reference with no base in scope must hard-fail");
    // The code is `purrdf-iri`'s, reached through the facade — one identity for every
    // surface, never a per-binding respelling.
    assert_eq!(error.code, "iri-relative-no-base");
    assert_eq!(
        IriError::NoBase {
            reference: "rel".to_owned()
        }
        .diagnostic_code(),
        error.code,
        "the facade's diagnostic code is purrdf-iri's own"
    );
}

#[test]
fn a_non_absolute_base_hard_fails_on_both_legs() {
    let ingress = parse_dataset(
        RELATIVE_TURTLE.as_bytes(),
        "text/turtle",
        Some("not-absolute/"),
    )
    .expect_err("a relative base is unusable on ingress");
    let dataset = parsed_under_base();
    let egress =
        serialize_dataset_to_format(&dataset, NativeRdfFormat::Turtle, Some("not-absolute/"))
            .expect_err("a relative base is unusable on egress");
    assert_eq!(
        ingress.code, egress.code,
        "one base validation, so one diagnostic identity across the two legs"
    );
    assert_eq!(
        ingress.code,
        BaseIri::parse("not-absolute/")
            .expect_err("a relative reference is not a base")
            .diagnostic_code()
    );
}

#[test]
fn an_in_document_base_overrides_the_callers() {
    // RFC-3986 §5.1: the document's own base wins over the caller-supplied one.
    let doc = "@base <https://example.org/inner/> .\n<rel> <https://example.org/p> \
               <https://example.org/o> .\n";
    let dataset = parse_dataset(doc.as_bytes(), "text/turtle", Some(BASE)).expect("parses");
    let nt = text(&dataset, NativeRdfFormat::NTriples, None);
    assert!(
        nt.contains("<https://example.org/inner/rel>"),
        "the in-document @base must win over the caller's, got: {nt}"
    );
}

#[test]
fn the_base_types_are_reachable_from_the_facade_alone() {
    // A consumer that builds or inspects a base names these through `purrdf::iri`, so
    // carrying a base never costs a second dependency on `purrdf-iri`.
    let base = BaseIri::parse(BASE).expect("an absolute IRI is a base");
    let scope = BaseScope::rooted(base.clone(), BaseOrigin::Caller);
    assert!(matches!(
        scope.current().map(ScopedBase::origin),
        Some(BaseOrigin::Caller)
    ));
    let resolved = scope
        .resolve("rel")
        .expect("the scope resolves a relative reference");
    assert_eq!(resolved.as_str(), "https://example.org/base/rel");
    // The inverse a serializer uses: relativize a target back against the base.
    assert_eq!(base.relativize(&resolved).as_deref(), Some("rel"));
}
