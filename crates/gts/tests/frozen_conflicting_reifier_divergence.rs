// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! A MEASURED cross-engine divergence on `vectors/12-conflicting-reifier`.
//!
//! # What the vector encodes
//!
//! `vectors/12-conflicting-reifier.gts` is a single generic-profile segment
//! holding five terms and no quads. Its `reifies` frame binds ONE reifier id,
//! `https://example.org/r1`, to TWO different triples:
//!
//! * `<https://example.org/Cat> rdfs:label "Cat"@en`
//! * `<https://example.org/Cat> rdfs:label "Chat"@fr`
//!
//! Critically, the file contains no quoted-triple term at all — no `k:3` term,
//! and therefore no `"tt"`-less term whose meaning could depend on which of the
//! two bindings wins.
//!
//! # Why this reader's answer changed
//!
//! `rdf:reifies` is NOT a functional property: one reifier id may legitimately
//! bind several distinct triples, and the `reifies` frame is a multi-valued
//! statement layer. An earlier wire model routed a quoted triple's components
//! indirectly through its reifier id, so the wire could express only one triple
//! per reifier; the reader enforced that limit by keeping the first binding and
//! discarding the rest in silence. A terms row now carries its own triple
//! (`"tt"`), which is authoritative, so the reifier indirection no longer has
//! to be single-valued and no binding is dropped.
//!
//! `ConflictingReifier` survives, but only for the shape that is still
//! genuinely incoherent — a `"tt"`-less `k:3` term whose reifier binds several
//! triples, which would give that one term two meanings. That shape is covered
//! by `tests/multi_binding_reifier.rs`. This vector does not contain it, so
//! this reader folds it to two reifier rows and raises nothing.
//!
//! # Why the vector is not "fixed" here
//!
//! The GTS wire format and its shared vector corpus are governed upstream in
//! `gmeow-gts`; `vectors/` is byte-frozen across engines and is never
//! regenerated, edited, or deleted in this repository. The committed
//! `vectors/12-conflicting-reifier.expected.json` still states the superseded
//! single-binding reading: one N-Quad and a `ConflictingReifier` diagnostic.
//! That expectation encodes the wrong premise — that `rdf:reifies` is
//! functional — but correcting it is an upstream act, not ours.
//!
//! # What this test is for
//!
//! A known divergence that no check observes is a swallowed error. Nothing else
//! in this repository reads this vector, so without this test the disagreement
//! would be invisible. The test therefore does two things:
//!
//! 1. Pins EXACTLY what this reader produces today, so our side cannot drift
//!    unnoticed.
//! 2. Asserts that the committed expectation says something DIFFERENT, so the
//!    divergence is asserted rather than merely described.
//!
//! # What must land upstream for the two to agree
//!
//! The governing corpus must republish `12-conflicting-reifier.expected.json`
//! with both `rdf:reifies` statements and an empty `diagnostics` list, keeping
//! the `.gts` bytes as they are — the file is valid, only the expectation was
//! written against the functional-`rdf:reifies` premise. A conforming
//! `ConflictingReifier` vector needs a `"tt"`-less `k:3` term added, which
//! would be new bytes and hence a new vector id.
//!
//! When that lands, [`the_frozen_expectation_still_states_the_superseded_reading`]
//! starts FAILING. That failure is the signal that the divergence closed: at
//! that point this file should collapse into a plain agreement check — the
//! reader fold compared for equality against the expectation, with no
//! divergence assertions left.

use std::path::PathBuf;

use purrdf_gts::model::{Graph, TermKind};
use purrdf_gts::reader::read;
use serde_json::Value as Json;

/// The frozen vector under test, without extension.
const VECTOR: &str = "12-conflicting-reifier";

fn vectors_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vectors")
}

/// Fold the frozen vector through the real reader, exactly as a consumer would.
fn fold_frozen_vector() -> Graph {
    let path = vectors_dir().join(format!("{VECTOR}.gts"));
    let bytes = std::fs::read(&path)
        .unwrap_or_else(|error| panic!("read frozen vector {}: {error}", path.display()));
    read(&bytes, true, None)
}

/// Parse the frozen cross-engine expectation that ships beside the vector.
fn frozen_expectation() -> Json {
    let path = vectors_dir().join(format!("{VECTOR}.expected.json"));
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read frozen expectation {}: {error}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|error| panic!("parse {}: {error}", path.display()))
}

/// Render one reifier row as `(reifier, subject, predicate, object, graph?)`
/// using each term's human-readable spelling, so an assertion failure names the
/// data rather than a term id.
fn describe_reifier_rows(graph: &Graph) -> Vec<(String, String, String, String, Option<String>)> {
    let spell = |id: usize| -> String {
        let term = &graph.terms[id];
        let value = term.value.clone().unwrap_or_default();
        match (term.kind, term.lang.as_deref()) {
            (TermKind::Literal, Some(lang)) => format!("\"{value}\"@{lang}"),
            (TermKind::Literal, None) => format!("\"{value}\""),
            (TermKind::Bnode, _) => format!("_:{value}"),
            _ => value,
        }
    };
    graph
        .reifiers
        .iter()
        .map(|(reifier, (subject, predicate, object), by)| {
            (
                spell(*reifier),
                spell(*subject),
                spell(*predicate),
                spell(*object),
                by.map(&spell),
            )
        })
        .collect()
}

const CAT: &str = "https://example.org/Cat";
const LABEL: &str = "http://www.w3.org/2000/01/rdf-schema#label";
const R1: &str = "https://example.org/r1";

/// Our side of the divergence, pinned in full: both bindings survive, with
/// their content and their (default) graph slot, and nothing is reported.
#[test]
fn purrdf_folds_the_frozen_vector_to_two_bindings_and_no_diagnostic() {
    let graph = fold_frozen_vector();

    assert_eq!(
        describe_reifier_rows(&graph),
        vec![
            (
                R1.to_string(),
                CAT.to_string(),
                LABEL.to_string(),
                "\"Cat\"@en".to_string(),
                None,
            ),
            (
                R1.to_string(),
                CAT.to_string(),
                LABEL.to_string(),
                "\"Chat\"@fr".to_string(),
                None,
            ),
        ],
        "one reifier id bound to two distinct triples must keep BOTH rows",
    );

    assert!(
        graph.diagnostics.is_empty(),
        "a multi-valued statement layer is legitimate RDF 1.2, not a defect: {:?}",
        graph.diagnostics,
    );

    // The premise behind the surviving `ConflictingReifier` guard: the vector
    // has no quoted-triple term whose meaning the two bindings could split.
    assert!(
        graph.terms.iter().all(|term| term.kind != TermKind::Triple),
        "the vector encodes no quoted-triple term, so no term is made ambiguous",
    );

    // The rest of the fold, so a drift anywhere in this vector is caught here.
    assert_eq!(graph.terms.len(), 5, "term count");
    assert!(graph.quads.is_empty(), "the vector carries no base quads");
    assert!(
        graph.annotations.is_empty(),
        "the vector carries no annotations"
    );
    assert!(
        graph.suppressions.is_empty(),
        "the vector suppresses nothing"
    );
    assert_eq!(
        graph.segment_profiles,
        vec!["generic".to_string()],
        "profile"
    );
    assert_eq!(graph.segment_heads.len(), 1, "segment count");
}

/// The other side of the divergence, pinned from the committed expectation.
///
/// This must FAIL the day the governing corpus republishes the expectation to
/// match a non-functional `rdf:reifies`. See this file's module documentation
/// for what to do then.
#[test]
fn the_frozen_expectation_still_states_the_superseded_reading() {
    let expectation = frozen_expectation();
    let graph = fold_frozen_vector();

    let expected_diagnostics: Vec<&str> = expectation["diagnostics"]
        .as_array()
        .expect("expectation carries a diagnostics array")
        .iter()
        .map(|code| code.as_str().expect("diagnostic codes are strings"))
        .collect();
    let expected_statements: Vec<&str> = expectation["nquads"]
        .as_array()
        .expect("expectation carries an nquads array")
        .iter()
        .map(|line| line.as_str().expect("nquads entries are strings"))
        .collect();

    // Pin the expectation verbatim, so an upstream republication is observed
    // here rather than passing unnoticed.
    assert_eq!(
        expected_diagnostics,
        vec!["ConflictingReifier"],
        "the frozen expectation is no longer the pre-fix one — the divergence \
         may have closed; re-read this file's module documentation",
    );
    assert_eq!(
        expected_statements.len(),
        1,
        "the frozen expectation is no longer the pre-fix one — the divergence \
         may have closed; re-read this file's module documentation",
    );

    // And assert the two sides actually disagree, in both respects.
    let our_diagnostics: Vec<&str> = graph
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code.as_str())
        .collect();
    assert_ne!(
        our_diagnostics, expected_diagnostics,
        "DIVERGENCE: the expectation demands a diagnostic this reader no longer \
         raises, because `rdf:reifies` is not functional",
    );
    assert_ne!(
        graph.reifiers.len(),
        expected_statements.len(),
        "DIVERGENCE: the expectation keeps one binding where this reader keeps \
         every binding the statement layer carries",
    );

    // Name the shape of the disagreement: the expectation retains the first
    // binding in file order and drops the second.
    assert!(
        expected_statements[0].contains("\"Cat\"@en"),
        "the retained binding is the first in file order: {expected_statements:?}",
    );
    assert!(
        !expected_statements
            .iter()
            .any(|line| line.contains("\"Chat\"@fr")),
        "the dropped binding is the one this reader additionally keeps: \
         {expected_statements:?}",
    );
}
