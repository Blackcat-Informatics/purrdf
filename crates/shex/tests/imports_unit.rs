// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Unit tests for `IMPORT` resolution (`resolve_imports`): transitive merge,
//! cycle/self-import tolerance, root-wins `start`, conflict/unresolved errors,
//! and end-to-end validation across an import boundary. The resolver is a
//! pure in-memory map — no filesystem, mirroring the wasm-clean injection
//! contract.

use std::collections::HashMap;
use std::sync::Arc;

use purrdf_core::{RdfDataset, RdfDatasetBuilder, RdfLiteral, TermValue};
use purrdf_shex::{
    parse_shexc, resolve_imports, validate, ConformanceStatus, ShapeSelector, ShexError,
};

/// Parse `src` as ShExC with a fixed base.
fn schema(src: &str) -> purrdf_shex::Schema {
    parse_shexc(src, Some("http://a.example/")).expect("schema parses")
}

/// A resolver over an in-memory `IRI -> ShExC source` map.
fn resolver_map(docs: &[(&str, &str)]) -> HashMap<String, String> {
    docs.iter()
        .map(|(iri, src)| ((*iri).to_owned(), (*src).to_owned()))
        .collect()
}

/// Adapt an in-memory lookup to the `ImportResolver` contract: a miss is a
/// concrete cause, not a silent `None`.
fn missing_or(
    found: Option<purrdf_shex::Schema>,
    iri: &str,
) -> Result<purrdf_shex::Schema, ShexError> {
    found.ok_or_else(|| ShexError::shexj(format!("no test document {iri}")))
}

fn dataset() -> Arc<RdfDataset> {
    // n1 <p1> n2 ; n2 <p2> "X"
    let mut b = RdfDatasetBuilder::new();
    let n1 = b.intern_iri("http://a.example/n1");
    let n2 = b.intern_iri("http://a.example/n2");
    let p1 = b.intern_iri("http://a.example/p1");
    let p2 = b.intern_iri("http://a.example/p2");
    let x = b.intern_literal(RdfLiteral::simple("X"));
    b.push_quad(n1, p1, n2, None);
    b.push_quad(n2, p2, x, None);
    b.freeze().expect("freeze")
}

#[test]
fn transitive_import_merges_and_validates() {
    // Root S1 references S2, which lives in the imported document.
    let root = schema("IMPORT <http://a.example/imported>\n<S1> { <p1> @<http://a.example/S2> }");
    let docs = resolver_map(&[("http://a.example/imported", "<S2> { <p2> . }")]);
    let merged = resolve_imports(root, &|iri| {
        missing_or(docs.get(iri).map(|s| schema(s)), iri)
    })
    .expect("imports resolve");
    assert!(merged.imports.is_empty(), "imports flattened away");
    assert_eq!(merged.shapes.len(), 2, "S1 and imported S2 present");

    let data = dataset();
    let result = validate(
        &merged,
        &data,
        &[(
            TermValue::iri("http://a.example/n1"),
            ShapeSelector::Label("http://a.example/S1".to_owned()),
        )],
    );
    assert_eq!(result.entries[0].status, ConformanceStatus::Conformant);
}

#[test]
fn cyclic_imports_terminate() {
    // a imports b imports a (chain cycle); each contributes one shape.
    let root = schema("IMPORT <http://a.example/b>\n<S1> { <p1> @<http://a.example/S2> }");
    let docs = resolver_map(&[(
        "http://a.example/b",
        "IMPORT <http://a.example/a>\n<S2> { <p2> . }",
    )]);
    // `a` re-imports `b` and redeclares the root's S1 identically.
    let mut docs = docs;
    docs.insert(
        "http://a.example/a".to_owned(),
        "IMPORT <http://a.example/b>\n<S1> { <p1> @<http://a.example/S2> }".to_owned(),
    );
    let merged = resolve_imports(root, &|iri| {
        missing_or(docs.get(iri).map(|s| schema(s)), iri)
    })
    .expect("cycle terminates");
    assert_eq!(merged.shapes.len(), 2, "identical S1 deduped, S2 merged");
}

#[test]
fn self_import_dedups() {
    let root = schema("IMPORT <http://a.example/self>\n<S1> { <p1> . }");
    let docs = resolver_map(&[("http://a.example/self", "<S1> { <p1> . }")]);
    let merged = resolve_imports(root, &|iri| {
        missing_or(docs.get(iri).map(|s| schema(s)), iri)
    })
    .expect("self-import ok");
    assert_eq!(merged.shapes.len(), 1, "self re-declaration deduped");
}

#[test]
fn root_start_wins_over_import() {
    let root = schema("IMPORT <http://a.example/i>\nstart=@<http://a.example/S1>\n<S1> { <p1> . }");
    let docs = resolver_map(&[(
        "http://a.example/i",
        "start=@<http://a.example/S2>\n<S2> { <p2> . }",
    )]);
    let merged = resolve_imports(root, &|iri| {
        missing_or(docs.get(iri).map(|s| schema(s)), iri)
    })
    .expect("resolves");
    // Root's start is kept; the imported start is dropped.
    assert!(merged.start.is_some(), "root start preserved");
    // The start must reference S1 (root), not S2 (import).
    let start = format!("{:?}", merged.start);
    assert!(start.contains("S1"), "root start=@S1 retained: {start}");
}

#[test]
fn conflicting_redefinition_errors() {
    let root = schema("IMPORT <http://a.example/i>\n<S1> { <p1> . }");
    let docs = resolver_map(&[("http://a.example/i", "<S1> { <p2> . }")]);
    let err = resolve_imports(root, &|iri| {
        missing_or(docs.get(iri).map(|s| schema(s)), iri)
    })
    .expect_err("conflicting S1 rejected");
    assert!(
        matches!(err, ShexError::ImportConflict(_)),
        "typed conflict variant: {err:?}"
    );
    assert_eq!(
        format!("{err}"),
        "conflicting redefinition of shape http://a.example/S1",
        "accurate conflict message: {err}"
    );
}

#[test]
fn unresolved_import_errors() {
    let root = schema("IMPORT <http://a.example/missing>\n<S1> { <p1> . }");
    let err =
        resolve_imports(root, &|iri| missing_or(None, iri)).expect_err("missing import rejected");
    assert!(
        matches!(err, ShexError::Import { .. }),
        "typed unresolved variant: {err:?}"
    );
    // The concrete resolver cause is preserved, not flattened away.
    let rendered = format!("{err}");
    assert!(
        rendered.contains("unresolved IMPORT")
            && rendered.contains("no test document http://a.example/missing"),
        "unresolved error surfaces its concrete cause: {err}"
    );
}
