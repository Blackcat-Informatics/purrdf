// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Unit tests for semantic-action dispatch: the Test extension's `print`/`fail`
//! verdict at the triple-constraint, shape, group and start positions; the
//! inert-by-default and unregistered-extension behaviours; and a custom
//! registered extension.

use std::sync::Arc;

use purrdf_core::{RdfDataset, RdfDatasetBuilder, TermValue};
use purrdf_shex::{
    parse_shexc, validate, validate_with, ConformanceStatus, SemActRegistry, ShapeSelector,
    ValidationOptions,
};

fn data() -> Arc<RdfDataset> {
    // s1 <p1> o1
    let mut b = RdfDatasetBuilder::new();
    let s1 = b.intern_iri("http://a.example/s1");
    let p1 = b.intern_iri("http://a.example/p1");
    let o1 = b.intern_iri("http://a.example/o1");
    b.push_quad(s1, p1, o1, None);
    b.freeze().expect("freeze")
}

/// Validate focus `s1` against shape `S1` with the Test extension registered.
fn conformant_with_test(schema_src: &str) -> bool {
    let schema = parse_shexc(schema_src, Some("http://a.example/")).expect("schema parses");
    let data = data();
    let options = ValidationOptions {
        sem_acts: SemActRegistry::with_test(),
        ..ValidationOptions::default()
    };
    let map = [(
        TermValue::iri("http://a.example/s1"),
        ShapeSelector::Label("http://a.example/S1".to_owned()),
    )];
    validate_with(&schema, &data, &map, &options).entries[0].status == ConformanceStatus::Conformant
}

const TEST: &str = "http://shex.io/extensions/Test/";

#[test]
fn triple_constraint_print_passes_fail_fails() {
    assert!(conformant_with_test(&format!(
        "<S1> {{ <p1> . %<{TEST}>{{ print(o) %}} }}"
    )));
    assert!(!conformant_with_test(&format!(
        "<S1> {{ <p1> . %<{TEST}>{{ fail(s) %}} }}"
    )));
    // A `fail` amongst prints on the same constraint still fails.
    assert!(!conformant_with_test(&format!(
        "<S1> {{ <p1> . %<{TEST}>{{ print(s) %}}%<{TEST}>{{ fail(s) %}}%<{TEST}>{{ print(o) %}} }}"
    )));
}

#[test]
fn shape_action_fail_fails() {
    assert!(conformant_with_test(&format!(
        "<S1> {{ <p1> . }} %<{TEST}>{{ print(\"ok\") %}}"
    )));
    assert!(!conformant_with_test(&format!(
        "<S1> {{ <p1> . }} %<{TEST}>{{ fail(\"no\") %}}"
    )));
}

#[test]
fn start_action_fail_fails() {
    let schema = parse_shexc(
        &format!("%<{TEST}>{{ fail(\"startAct\") %}}\n<S1> {{ <p1> . }}"),
        Some("http://a.example/"),
    )
    .expect("schema parses");
    let data = data();
    let options = ValidationOptions {
        sem_acts: SemActRegistry::with_test(),
        ..ValidationOptions::default()
    };
    let map = [(
        TermValue::iri("http://a.example/s1"),
        ShapeSelector::Label("http://a.example/S1".to_owned()),
    )];
    let out = validate_with(&schema, &data, &map, &options);
    assert_eq!(out.entries[0].status, ConformanceStatus::Nonconformant);
    assert_eq!(
        out.entries[0].reason.as_deref(),
        Some("start semantic action failed")
    );
}

#[test]
fn unregistered_extension_is_inert() {
    // A `fail` on an extension the registry does not know still passes.
    assert!(conformant_with_test(
        "<S1> { <p1> . %<http://example.org/Other>{ fail(s) %} }"
    ));
}

#[test]
fn default_registry_ignores_actions() {
    // Plain `validate` has an empty registry: even a Test `fail` is inert.
    let schema = parse_shexc(
        &format!("<S1> {{ <p1> . %<{TEST}>{{ fail(s) %}} }}"),
        Some("http://a.example/"),
    )
    .expect("schema parses");
    let data = data();
    let map = [(
        TermValue::iri("http://a.example/s1"),
        ShapeSelector::Label("http://a.example/S1".to_owned()),
    )];
    assert_eq!(
        validate(&schema, &data, &map).entries[0].status,
        ConformanceStatus::Conformant
    );
}

#[test]
fn custom_extension_can_veto() {
    // Register an extension that fails whenever its code mentions "veto".
    let schema = parse_shexc(
        "<S1> { <p1> . %<http://example.org/Veto>{ veto %} }",
        Some("http://a.example/"),
    )
    .expect("schema parses");
    let data = data();
    let mut registry = SemActRegistry::new();
    registry.register(
        "http://example.org/Veto",
        Box::new(
            |act: &purrdf_shex::SemAct, _ctx: &purrdf_shex::SemActContext| {
                !act.code.as_deref().unwrap_or_default().contains("veto")
            },
        ),
    );
    let options = ValidationOptions {
        sem_acts: registry,
        ..ValidationOptions::default()
    };
    let map = [(
        TermValue::iri("http://a.example/s1"),
        ShapeSelector::Label("http://a.example/S1".to_owned()),
    )];
    assert_eq!(
        validate_with(&schema, &data, &map, &options).entries[0].status,
        ConformanceStatus::Nonconformant
    );
}
