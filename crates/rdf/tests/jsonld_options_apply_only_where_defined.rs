// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! JSON-LD serialization options reach only the two formats that define them.
//!
//! `serialize_dataset_with` refuses the options up front for any other format, and
//! its format dispatch now names `JsonLd` and `YamlLd` explicitly instead of
//! sending every remaining format down the YAML-LD writer through a catch-all. The
//! catch-all was unreachable behind the guard, but it meant the failure mode of
//! loosening that guard was a Turtle request silently returning a YAML document
//! rather than an error. Both spellings route through one diagnostic constructor,
//! so the condition has one wording.
//!
//! This file is the executable statement of that contract, in both directions: the
//! formats that do not define the options are refused BY NAME, and the two that do
//! still serialize, each in its own syntax. A refusal that cannot be shown to
//! spare the inputs it is supposed to admit is not strictness.

use purrdf_core::{RdfDatasetBuilder, RdfTerm};
use purrdf_rdf::{
    JsonLdSerializeOptions, NativeRdfFormat, serialize_dataset_to_format_with_jsonld_options,
};

/// One quad, so every format has something to emit.
fn dataset() -> std::sync::Arc<purrdf_core::RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let s = builder.intern_owned_term(&RdfTerm::iri("https://example.org/s"));
    let p = builder.intern_iri("https://example.org/p");
    let o = builder.intern_owned_term(&RdfTerm::iri("https://example.org/o"));
    builder.push_quad(s, p, o, None);
    builder.freeze().expect("dataset freezes")
}

/// Every format that does NOT define JSON-LD options refuses them, by name, rather
/// than quietly emitting some other format's document.
#[test]
fn a_format_without_jsonld_options_refuses_them() {
    let dataset = dataset();
    let options = JsonLdSerializeOptions::expanded();

    for format in [
        NativeRdfFormat::Turtle,
        NativeRdfFormat::TriG,
        NativeRdfFormat::NTriples,
        NativeRdfFormat::NQuads,
        NativeRdfFormat::RdfXml,
        NativeRdfFormat::TriX,
        NativeRdfFormat::HexTuples,
    ] {
        let diagnostic = serialize_dataset_to_format_with_jsonld_options(
            &*dataset, format, None, &options,
        )
        .expect_err("JSON-LD options do not apply to this format and must be refused");
        let rendered = diagnostic.to_string();
        assert!(
            rendered.contains("cannot be used with"),
            "unexpected refusal wording for {format:?}: {rendered}"
        );
        assert!(
            rendered.contains(format.media_type()),
            "the refusal must name the format that was asked for; got: {rendered}"
        );
    }
}

/// The valid neighbours: the two formats that DO define the options still serialize,
/// and each still produces its own syntax rather than the other's.
#[test]
fn jsonld_and_yamlld_still_accept_their_own_options() {
    let dataset = dataset();
    let options = JsonLdSerializeOptions::expanded();

    let jsonld = serialize_dataset_to_format_with_jsonld_options(
        &*dataset,
        NativeRdfFormat::JsonLd,
        None,
        &options,
    )
    .expect("JSON-LD defines these options");
    let jsonld = String::from_utf8(jsonld.bytes).expect("utf-8");
    assert!(
        jsonld.trim_start().starts_with('{') || jsonld.trim_start().starts_with('['),
        "JSON-LD must emit JSON, got: {}",
        &jsonld[..jsonld.len().min(80)]
    );

    let yamlld = serialize_dataset_to_format_with_jsonld_options(
        &*dataset,
        NativeRdfFormat::YamlLd,
        None,
        &options,
    )
    .expect("YAML-LD defines these options");
    let yamlld = String::from_utf8(yamlld.bytes).expect("utf-8");
    assert!(
        yamlld.contains("yaml-language-server"),
        "YAML-LD must emit YAML, got: {}",
        &yamlld[..yamlld.len().min(80)]
    );

    assert_ne!(
        jsonld, yamlld,
        "the two formats must not collapse onto one writer"
    );
}

/// Without options, every format still serializes — the refusal is conditioned on
/// the options being present, not on the format.
#[test]
fn every_format_still_serializes_without_options() {
    let dataset = dataset();
    for format in [
        NativeRdfFormat::Turtle,
        NativeRdfFormat::TriG,
        NativeRdfFormat::NTriples,
        NativeRdfFormat::NQuads,
        NativeRdfFormat::RdfXml,
        NativeRdfFormat::TriX,
        NativeRdfFormat::HexTuples,
        NativeRdfFormat::JsonLd,
        NativeRdfFormat::YamlLd,
    ] {
        purrdf_rdf::serialize_dataset_to_format(&*dataset, format, None)
            .unwrap_or_else(|e| panic!("{format:?} serializes without options: {e}"));
    }
}
