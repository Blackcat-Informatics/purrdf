// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Byte-level (canonicalized-key-level) spec-spelling pins.
//!
//! `results_roundtrip.rs` proves this crate's writer and reader NEVER
//! disagree with EACH OTHER (decode → re-encode → re-decode → model-compare).
//! That is deliberately blind to a shared-but-wrong spelling: if the writer
//! renamed `"its:dir"` to `"its:direction"` and the reader learned the same
//! new name on the same day, every model-level comparison anywhere in this
//! repo — including `results_roundtrip.rs` — would stay green, because both
//! sides of every comparison are DECODED MODELS, and model equality has
//! already thrown the spelling away.
//!
//! This harness closes that gap the only way that is possible: by comparing
//! actual SERIALIZED BYTES against the spelling a real spec-derived fixture
//! carries, so a future writer change that diverges from the spec's spelling
//! FAILS here even though every model-level check stays green.
//!
//! # What is and is not "byte-level" here
//!
//! A whole-DOCUMENT byte-for-byte comparison against the vendored W3C corpus
//! is not meaningful: the vendored fixtures use their own pretty-printing
//! (extra spaces after `:`/`,` in JSON, multi-line indented XML) that this
//! writer's compact/canonical style never reproduces, and JSON key ORDER
//! differs too (the vendored fixtures put `"type"` after `"value"`; this
//! writer puts it first) — none of that is a spelling divergence, since JSON
//! key order is not significant and XML inter-element whitespace is not
//! significant. So each pin below:
//!
//! 1. reads a REAL fixture from disk (vendored W3C corpus, or — for XML base
//!    direction, where no vendored `.srx` carries a directional literal at
//!    all — the spec-derived `suite/purrdf-extend/basedir.srx`),
//! 2. decodes it with this crate's OWN reader (so the term data under test is
//!    never hand-typed by this file — it comes from the fixture's real
//!    bytes),
//! 3. re-encodes it with this crate's OWN writer, and
//! 4. asserts the exact byte SPELLING of the encoding under test — the
//!    specific key/attribute name, its presence or deliberate absence, and
//!    its value — appears in the writer's own output, cross-anchored against
//!    the fixture's raw bytes carrying the same fact under the spec's own
//!    (differently-whitespaced) spelling.
//!
//! A future writer regression in any of these spellings fails step 4 directly
//! — it cannot hide behind model equality, because nothing here decodes the
//! writer's own output before asserting on it.

use std::path::{Path, PathBuf};

use purrdf_core::{RdfDatasetBuilder, SparqlResult};
use purrdf_sparql_results::{ResultProvenance, from_json, from_xml, to_json, to_xml};

/// The `suite/` directory of this crate.
fn suite_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("suite")
}

/// Read a fixture's raw bytes from `suite/<rel>`.
fn read_fixture(rel: &str) -> Vec<u8> {
    let path = suite_root().join(rel);
    std::fs::read(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// Wrap decoded `SELECT` solutions as the model-level [`SparqlResult`] the
/// writer functions take.
fn as_solutions(parsed: purrdf_sparql_results::ParsedSolutions) -> SparqlResult {
    SparqlResult::Solutions {
        variables: parsed.variables,
        rows: parsed.rows,
        aux: RdfDatasetBuilder::new()
            .freeze()
            .expect("an empty dataset always freezes"),
    }
}

/// JSON base direction: re-serializing the vendored W3C
/// `lang-basedir/langdir-literal.srj` fixture must still spell the attribute
/// `"its:dir"` (compact) — cross-anchored against the fixture's own (spaced)
/// `"its:dir": "…"` spelling on disk.
///
/// This same fixture also pins the simple-literal encoding decision: its
/// `"langdir"` column carries a plain empty-string
/// literal written BARE (`{ "type": "literal" , "value": "" }`, no
/// `"datatype"` member) — the spec's own encoding table says a simple literal
/// serializes bare (see `crates/sparql-results/src/json.rs`'s module docs),
/// so this writer must reproduce that, not silently add back an explicit
/// `"datatype":"…#string"`.
#[test]
fn json_base_direction_and_bare_simple_literal_pin() {
    let raw = read_fixture("w3c-sparql12/lang-basedir/langdir-literal.srj");
    let raw_text = String::from_utf8(raw.clone()).expect("UTF-8 fixture");

    // Anchor: the vendored fixture itself really does carry these spellings
    // (guards against reading the wrong file, or the fixture drifting).
    assert!(
        raw_text.contains("\"its:dir\": \"ltr\""),
        "vendored fixture must carry its:dir=ltr: {raw_text}"
    );
    assert!(
        raw_text.contains("\"its:dir\": \"rtl\""),
        "vendored fixture must carry its:dir=rtl: {raw_text}"
    );
    assert!(
        raw_text.contains(r#"{ "type": "literal" , "value": "" }"#),
        "vendored fixture must carry a bare simple literal for `langdir`'s \
         no-direction row: {raw_text}"
    );

    let parsed = from_json(&raw).expect("vendored fixture decodes");
    let result = as_solutions(parsed);
    let written = to_json(&result, &ResultProvenance::default(), None).expect("re-encodes");
    let out = String::from_utf8(written.bytes).expect("UTF-8 output");

    // The writer's own (compact) its:dir spelling, driven by data decoded
    // from the real fixture.
    assert!(
        out.contains("\"its:dir\":\"ltr\""),
        "writer output must carry compact its:dir=ltr: {out}"
    );
    assert!(
        out.contains("\"its:dir\":\"rtl\""),
        "writer output must carry compact its:dir=rtl: {out}"
    );

    // The `langdir` column's bare-string rows must stay bare in the writer's
    // own output too — no resurrected `"datatype":"…#string"`.
    assert!(
        out.contains("\"langdir\":{\"type\":\"literal\",\"value\":\"\"}"),
        "simple literal must serialize bare (no datatype member): {out}"
    );
    assert!(
        !out.contains("\"langdir\":{\"type\":\"literal\",\"value\":\"\",\"datatype\""),
        "simple literal must NOT carry an explicit xsd:string datatype: {out}"
    );
}

/// JSON triple terms: re-serializing the vendored W3C
/// `eval-triple-terms/basic-2.srj` fixture must reproduce the exact
/// `{"type":"triple","value":{"subject":…,"predicate":…,"object":…}}` nested
/// shape, keyed off the fixture's own IRIs (never hand-typed here).
#[test]
fn json_triple_term_pin_against_vendored_fixture() {
    let raw = read_fixture("w3c-sparql12/eval-triple-terms/basic-2.srj");
    let parsed = from_json(&raw).expect("vendored fixture decodes");
    let result = as_solutions(parsed);
    let written = to_json(&result, &ResultProvenance::default(), None).expect("re-encodes");
    let out = String::from_utf8(written.bytes).expect("UTF-8 output");

    let expected_triple = concat!(
        "\"o\":{\"type\":\"triple\",\"value\":{",
        "\"subject\":{\"type\":\"uri\",\"value\":\"http://example/a\"},",
        "\"predicate\":{\"type\":\"uri\",\"value\":\"http://example/b\"},",
        "\"object\":{\"type\":\"uri\",\"value\":\"http://example/c\"}}}",
    );
    assert!(
        out.contains(expected_triple),
        "triple-term encoding drifted from the vendored fixture's own data: {out}"
    );
}

/// JSON triple terms carrying a directional literal component: re-serializing
/// the vendored W3C `expression/triple-on-str-literals.srj` fixture must
/// nest `"its:dir"` correctly inside the triple's subject.
#[test]
fn json_triple_term_with_directional_literal_pin_against_vendored_fixture() {
    let raw = read_fixture("w3c-sparql12/expression/triple-on-str-literals.srj");
    let parsed = from_json(&raw).expect("vendored fixture decodes");
    let result = as_solutions(parsed);
    let written = to_json(&result, &ResultProvenance::default(), None).expect("re-encodes");
    let out = String::from_utf8(written.bytes).expect("UTF-8 output");

    // The vendored row whose `subject` is `"a"` with `xml:lang: "nl"` and
    // `"its:dir": "ltr"`, nested inside a triple-term binding.
    let expected_subject = concat!(
        "\"subject\":{\"type\":\"literal\",\"value\":\"a\",",
        "\"xml:lang\":\"nl\",\"its:dir\":\"ltr\"}",
    );
    assert!(
        out.contains(expected_subject),
        "nested directional literal inside a triple term drifted: {out}"
    );
}

/// XML triple terms: re-serializing the vendored W3C
/// `eval-triple-terms/results-reifiedtriples-1.srx` fixture must reproduce
/// the exact `<triple><subject>…<predicate>…<object>…</triple>` nested shape,
/// keyed off the fixture's own IRIs.
#[test]
fn xml_triple_term_pin_against_vendored_fixture() {
    let raw = read_fixture("w3c-sparql12/eval-triple-terms/results-reifiedtriples-1.srx");
    let parsed = from_xml(&raw).expect("vendored fixture decodes");
    let result = as_solutions(parsed);
    let written = to_xml(&result, &ResultProvenance::default(), None).expect("re-encodes");
    let out = String::from_utf8(written.bytes).expect("UTF-8 output");

    let expected_triple = concat!(
        "<binding name=\"o\"><triple>",
        "<subject><uri>http://example/a</uri></subject>",
        "<predicate><uri>http://example/b</uri></predicate>",
        "<object><uri>http://example/c</uri></object>",
        "</triple></binding>",
    );
    assert!(
        out.contains(expected_triple),
        "triple-term encoding drifted from the vendored fixture's own data: {out}"
    );
}

/// XML base direction: no vendored W3C `.srx` fixture carries a directional
/// literal at all (upstream `w3c/rdf-tests` only exercises base direction via
/// `.srj` — see `crates/sparql-results/src/xml.rs`'s module docs), so this
/// pins against the spec-derived `suite/purrdf-extend/basedir.srx`
/// (constructed directly from the SPARQL 1.2 Query Results XML Format spec's
/// own §2.3.1 worked example content and its default root-declared
/// `xmlns:its`/`its:version` style).
///
/// Because this fixture's own bytes ARE this writer's canonical output for
/// the query `suite/purrdf-extend/basedir.rq` (also exercised end to end,
/// model-level, by the main manifest-driven conformance suite), decoding it
/// and re-encoding it must reproduce the fixture BYTE FOR BYTE — the
/// strongest pin available.
#[test]
fn xml_base_direction_pin_against_spec_derived_fixture() {
    let raw = read_fixture("purrdf-extend/basedir.srx");
    let raw_text = String::from_utf8(raw.clone()).expect("UTF-8 fixture");

    // Anchor: the fixture really carries the spec's root-declared namespace
    // style plus a bare per-literal its:dir (no inline xmlns:its) — the
    // decision recorded in `crates/sparql-results/src/xml.rs`'s module docs.
    assert!(
        raw_text.contains(
            "<sparql xmlns=\"http://www.w3.org/2005/sparql-results#\" xmlns:its=\"http://www.w3.org/2005/11/its\" its:version=\"2.0\">"
        ),
        "fixture must declare the ITS namespace on the root: {raw_text}"
    );
    assert!(
        raw_text.contains("<literal xml:lang=\"ar\" its:dir=\"rtl\">قطة</literal>"),
        "fixture must carry a bare (no inline xmlns:its) directional literal: {raw_text}"
    );

    let parsed = from_xml(&raw).expect("spec-derived fixture decodes");
    let result = as_solutions(parsed);
    let written = to_xml(&result, &ResultProvenance::default(), None).expect("re-encodes");

    assert_eq!(
        written.bytes, raw,
        "re-encoding the spec-derived base-direction fixture must reproduce it byte for byte"
    );
}
