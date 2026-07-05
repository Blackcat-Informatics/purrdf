// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Unit tests for semantic-action dispatch: the Test extension's `print`/`fail`
//! verdict at the triple-constraint, shape, group and start positions; the
//! inert-by-default and unregistered-extension behaviours; and a custom
//! registered extension.

use std::cell::RefCell;
use std::sync::Arc;

use purrdf_core::{RdfDataset, RdfDatasetBuilder, TermValue};
use purrdf_shex::{
    ConformanceStatus, SemActRegistry, ShapeSelector, ValidationOptions, parse_shexc, validate,
    validate_with,
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
fn group_act_in_unselected_oneof_branch_does_not_fire() {
    // OneOf of two singleton groups, each with its own group-level action
    // (forced by giving each inner triple constraint its own explicit `{1}`
    // cardinality, so the enclosing parens become a real EachOf group
    // rather than folding the action onto the bare constraint). Data has
    // p1 but not p2, so the p1 branch is selected and its `print` fires;
    // the p2 branch's `fail` must NOT fire even though it's part of a
    // matching `OneOf`.
    assert!(conformant_with_test(&format!(
        "<S1> {{ ( ( <p2> . {{1}} ) %<{TEST}>{{ fail(s) %}} | ( <p1> . {{1}} ) %<{TEST}>{{ print(s) %}} ) }}"
    )));
}

#[test]
fn group_act_on_zero_rep_optional_group_does_not_fire() {
    // An optional group (`?`) matching zero repetitions still carries a
    // `fail` action; since the group did not participate (no p2 triple
    // present), the action must not fire.
    assert!(conformant_with_test(&format!(
        "<S1> {{ ( <p2> . {{1}} )? %<{TEST}>{{ fail(s) %}} }}"
    )));
}

#[test]
fn group_act_on_matched_group_fires() {
    // The group actually matches (p1 is present), so its `fail` action
    // DOES fire, and the shape is nonconformant.
    assert!(!conformant_with_test(&format!(
        "<S1> {{ ( <p1> . {{1}} ) %<{TEST}>{{ fail(s) %}} }}"
    )));
}

#[test]
fn group_act_on_matched_group_print_still_conforms() {
    // Regression: a matched group with a non-failing action still
    // conforms.
    assert!(conformant_with_test(&format!(
        "<S1> {{ ( <p1> . {{1}} ) %<{TEST}>{{ print(s) %}} }}"
    )));
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

#[test]
fn custom_extension_fires_once_per_matched_arc_with_value_and_predicate() {
    // s1 has two <p1> arcs (o1, o2) and one <q1> arc (o3); shape
    // `{ <p1> {1,2} ; <q1> . }`. The Recorder extension must be invoked once
    // per matched TRIPLE (not once per slot), and `ctx.value` must be the
    // actual matched object of that triple with `ctx.predicate` set to the
    // constraint's own predicate.
    const EXT: &str = "http://example.org/Recorder";
    let mut b = RdfDatasetBuilder::new();
    let s1 = b.intern_iri("http://a.example/s1");
    let p1 = b.intern_iri("http://a.example/p1");
    let q1 = b.intern_iri("http://a.example/q1");
    let o1 = b.intern_iri("http://a.example/o1");
    let o2 = b.intern_iri("http://a.example/o2");
    let o3 = b.intern_iri("http://a.example/o3");
    b.push_quad(s1, p1, o1, None);
    b.push_quad(s1, p1, o2, None);
    b.push_quad(s1, q1, o3, None);
    let data = b.freeze().expect("freeze");

    let schema = parse_shexc(
        &format!("<S1> {{ <p1> . {{1,2}} %<{EXT}>{{ rec %}} ; <q1> . %<{EXT}>{{ rec %}} }}"),
        Some("http://a.example/"),
    )
    .expect("schema parses");

    let calls: RefCell<Vec<(Option<String>, Option<TermValue>)>> = RefCell::new(Vec::new());
    let mut registry = SemActRegistry::new();
    registry.register(
        EXT,
        Box::new(
            |_act: &purrdf_shex::SemAct, ctx: &purrdf_shex::SemActContext| {
                calls
                    .borrow_mut()
                    .push((ctx.predicate.clone(), ctx.value.clone()));
                true
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
    let out = validate_with(&schema, &data, &map, &options);
    assert_eq!(out.entries[0].status, ConformanceStatus::Conformant);
    drop(options);

    let recorded = calls.into_inner();
    assert_eq!(recorded.len(), 3, "one dispatch per matched arc, not slot");

    let p1_iri = "http://a.example/p1".to_owned();
    let q1_iri = "http://a.example/q1".to_owned();
    let mut p1_values: Vec<TermValue> = recorded
        .iter()
        .filter(|(pred, _)| pred.as_ref() == Some(&p1_iri))
        .map(|(_, value)| value.clone().expect("value populated"))
        .collect();
    p1_values.sort_by_key(|v| match v {
        TermValue::Iri(iri) => iri.clone(),
        _ => String::new(),
    });
    assert_eq!(
        p1_values,
        vec![
            TermValue::iri("http://a.example/o1"),
            TermValue::iri("http://a.example/o2"),
        ]
    );

    let q1_values: Vec<TermValue> = recorded
        .iter()
        .filter(|(pred, _)| pred.as_ref() == Some(&q1_iri))
        .map(|(_, value)| value.clone().expect("value populated"))
        .collect();
    assert_eq!(q1_values, vec![TermValue::iri("http://a.example/o3")]);
}
