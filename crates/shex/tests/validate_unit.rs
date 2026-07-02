// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Engine-level unit tests for the ShEx 2.1 validator: node-constraint
//! families, `EXTRA`/`CLOSED` corners, recursion (a linked list),
//! `OneOf`/`EachOf` partitioning with repeated predicates, inverse triple
//! constraints, stems/exclusions, and the `EXTERNAL` resolver hook.

use std::sync::Arc;

use purrdf_core::{RdfDataset, RdfDatasetBuilder, RdfLiteral, TermValue};
use purrdf_shex::{
    parse_shexc, validate, validate_with, ConformanceStatus, ShapeSelector, ValidationOptions,
};

/// A term spec for the tiny triple builder below.
enum T<'a> {
    I(&'a str),
    B(&'a str),
    L(RdfLiteral),
}

fn dataset(triples: &[(T<'_>, &str, T<'_>)]) -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let term = |b: &mut RdfDatasetBuilder, t: &T<'_>| match t {
        T::I(iri) => b.intern_iri(iri),
        T::B(label) => b.intern_blank(label, purrdf_core::BlankScope::DEFAULT),
        T::L(lit) => b.intern_literal(lit.clone()),
    };
    for (s, p, o) in triples {
        let s = term(&mut b, s);
        let p = b.intern_iri(p);
        let o = term(&mut b, o);
        b.push_quad(s, p, o, None);
    }
    b.freeze().expect("freeze")
}

fn check(
    schema: &str,
    data: &Arc<RdfDataset>,
    focus: TermValue,
    shape: &str,
) -> Result<(), String> {
    let schema = parse_shexc(schema, Some("http://a.example/")).expect("schema parses");
    let selector = if shape == "START" {
        ShapeSelector::Start
    } else {
        ShapeSelector::Label(shape.to_owned())
    };
    let result = validate(&schema, data, &[(focus, selector)]);
    let entry = &result.entries[0];
    match entry.status {
        ConformanceStatus::Conformant => Ok(()),
        ConformanceStatus::Nonconformant => Err(entry
            .reason
            .clone()
            .unwrap_or_else(|| "no reason".to_owned())),
    }
}

const S1: &str = "http://a.example/S1";

fn iri(v: &str) -> TermValue {
    TermValue::iri(v)
}

// ── node-constraint families ────────────────────────────────────────────────

#[test]
fn node_kind_families() {
    let data = dataset(&[
        (
            T::I("http://a.example/s1"),
            "http://a.example/p1",
            T::I("http://a.example/o1"),
        ),
        (
            T::I("http://a.example/s2"),
            "http://a.example/p1",
            T::B("b1"),
        ),
        (
            T::I("http://a.example/s3"),
            "http://a.example/p1",
            T::L(RdfLiteral::simple("x")),
        ),
    ]);
    let schema = "<S1> { <p1> IRI }";
    assert!(check(schema, &data, iri("http://a.example/s1"), S1).is_ok());
    assert!(check(schema, &data, iri("http://a.example/s2"), S1).is_err());
    assert!(check(schema, &data, iri("http://a.example/s3"), S1).is_err());
    let schema = "<S1> { <p1> BNODE }";
    assert!(check(schema, &data, iri("http://a.example/s2"), S1).is_ok());
    assert!(check(schema, &data, iri("http://a.example/s1"), S1).is_err());
    let schema = "<S1> { <p1> LITERAL }";
    assert!(check(schema, &data, iri("http://a.example/s3"), S1).is_ok());
    assert!(check(schema, &data, iri("http://a.example/s2"), S1).is_err());
    let schema = "<S1> { <p1> NONLITERAL }";
    assert!(check(schema, &data, iri("http://a.example/s1"), S1).is_ok());
    assert!(check(schema, &data, iri("http://a.example/s2"), S1).is_ok());
    assert!(check(schema, &data, iri("http://a.example/s3"), S1).is_err());
}

#[test]
fn datatype_requires_lexical_validity() {
    let xsd_int = "http://www.w3.org/2001/XMLSchema#integer";
    let good = dataset(&[(
        T::I("http://a.example/s1"),
        "http://a.example/p1",
        T::L(RdfLiteral::typed("42", xsd_int)),
    )]);
    let bad = dataset(&[(
        T::I("http://a.example/s1"),
        "http://a.example/p1",
        T::L(RdfLiteral::typed("4.2", xsd_int)),
    )]);
    let schema = "<S1> { <p1> <http://www.w3.org/2001/XMLSchema#integer> }";
    assert!(check(schema, &good, iri("http://a.example/s1"), S1).is_ok());
    let err = check(schema, &bad, iri("http://a.example/s1"), S1).expect_err("ill-formed");
    assert!(err.contains("ill-formed"), "reason: {err}");
}

#[test]
fn string_facets_count_scalar_values() {
    // "né" is 2 scalar values, 3 bytes.
    let data = dataset(&[(
        T::I("http://a.example/s1"),
        "http://a.example/p1",
        T::L(RdfLiteral::simple("né")),
    )]);
    assert!(check(
        "<S1> { <p1> LITERAL LENGTH 2 }",
        &data,
        iri("http://a.example/s1"),
        S1
    )
    .is_ok());
    assert!(check(
        "<S1> { <p1> LITERAL LENGTH 3 }",
        &data,
        iri("http://a.example/s1"),
        S1
    )
    .is_err());
    assert!(check(
        "<S1> { <p1> LITERAL MINLENGTH 2 MAXLENGTH 2 }",
        &data,
        iri("http://a.example/s1"),
        S1
    )
    .is_ok());
}

#[test]
fn pattern_facet_is_partial_match() {
    let data = dataset(&[(
        T::I("http://a.example/s1"),
        "http://a.example/p1",
        T::L(RdfLiteral::simple("abcd")),
    )]);
    assert!(check(
        "<S1> { <p1> LITERAL /bc/ }",
        &data,
        iri("http://a.example/s1"),
        S1
    )
    .is_ok());
    assert!(check(
        "<S1> { <p1> LITERAL /^bc$/ }",
        &data,
        iri("http://a.example/s1"),
        S1
    )
    .is_err());
    assert!(check(
        "<S1> { <p1> LITERAL /^ABCD$/i }",
        &data,
        iri("http://a.example/s1"),
        S1
    )
    .is_ok());
}

#[test]
fn numeric_facets_promote_across_types() {
    let xsd_dec = "http://www.w3.org/2001/XMLSchema#decimal";
    let data = dataset(&[(
        T::I("http://a.example/s1"),
        "http://a.example/p1",
        T::L(RdfLiteral::typed("4.5", xsd_dec)),
    )]);
    assert!(check(
        "<S1> { <p1> MININCLUSIVE 4 }",
        &data,
        iri("http://a.example/s1"),
        S1
    )
    .is_ok());
    assert!(check(
        "<S1> { <p1> MINEXCLUSIVE 4.5 }",
        &data,
        iri("http://a.example/s1"),
        S1
    )
    .is_err());
    assert!(check(
        "<S1> { <p1> MAXINCLUSIVE 4.5 }",
        &data,
        iri("http://a.example/s1"),
        S1
    )
    .is_ok());
    assert!(check(
        "<S1> { <p1> TOTALDIGITS 2 }",
        &data,
        iri("http://a.example/s1"),
        S1
    )
    .is_ok());
    assert!(check(
        "<S1> { <p1> FRACTIONDIGITS 0 }",
        &data,
        iri("http://a.example/s1"),
        S1
    )
    .is_err());
}

#[test]
fn facets_apply_to_iri_and_bnode_lexical_forms() {
    let data = dataset(&[
        (
            T::I("http://a.example/s1"),
            "http://a.example/p1",
            T::I("http://a.example/o1"),
        ),
        (
            T::I("http://a.example/s2"),
            "http://a.example/p1",
            T::B("abcde"),
        ),
    ]);
    // The IRI string is 19 scalar values.
    assert!(check(
        "<S1> { <p1> LENGTH 19 }",
        &data,
        iri("http://a.example/s1"),
        S1
    )
    .is_ok());
    // The blank-node label is 5.
    assert!(check(
        "<S1> { <p1> LENGTH 5 }",
        &data,
        iri("http://a.example/s2"),
        S1
    )
    .is_ok());
    assert!(check(
        "<S1> { <p1> LENGTH 4 }",
        &data,
        iri("http://a.example/s2"),
        S1
    )
    .is_err());
}

// ── value sets, stems and exclusions ────────────────────────────────────────

#[test]
fn value_set_stems_and_exclusions() {
    let data = |o: &str| dataset(&[(T::I("http://a.example/s1"), "http://a.example/p1", T::I(o))]);
    let schema = "<S1> { <p1> [ <http://a.example/v>~ ] }";
    assert!(check(
        schema,
        &data("http://a.example/v1"),
        iri("http://a.example/s1"),
        S1
    )
    .is_ok());
    assert!(check(
        schema,
        &data("http://b.example/v1"),
        iri("http://a.example/s1"),
        S1
    )
    .is_err());
    // Wildcard with exclusions: matching an exclusion fails even under `.`.
    let schema = "<S1> { <p1> [ . - <http://a.example/v1> - <http://a.example/w>~ ] }";
    assert!(check(
        schema,
        &data("http://a.example/v2"),
        iri("http://a.example/s1"),
        S1
    )
    .is_ok());
    assert!(check(
        schema,
        &data("http://a.example/v1"),
        iri("http://a.example/s1"),
        S1
    )
    .is_err());
    assert!(check(
        schema,
        &data("http://a.example/w9"),
        iri("http://a.example/s1"),
        S1
    )
    .is_err());
}

#[test]
fn language_stems_use_rfc4647_basic_filtering() {
    let data = |tag: &str| {
        dataset(&[(
            T::I("http://a.example/s1"),
            "http://a.example/p1",
            T::L(RdfLiteral::language_tagged("septante", tag)),
        )])
    };
    let schema = "<S1> { <p1> [ @fr~ ] }";
    assert!(check(schema, &data("fr"), iri("http://a.example/s1"), S1).is_ok());
    assert!(check(schema, &data("fr-BE"), iri("http://a.example/s1"), S1).is_ok());
    assert!(check(schema, &data("frc"), iri("http://a.example/s1"), S1).is_err());
    let schema = "<S1> { <p1> [ @~ - @fr-be ] }";
    assert!(check(schema, &data("fr"), iri("http://a.example/s1"), S1).is_ok());
    assert!(check(schema, &data("fr-BE"), iri("http://a.example/s1"), S1).is_err());
}

// ── EXTRA / CLOSED corners ──────────────────────────────────────────────────

#[test]
fn extra_tolerates_failing_and_vapid_values() {
    let data = dataset(&[
        (
            T::I("http://a.example/s1"),
            "http://a.example/p1",
            T::I("http://a.example/v1"),
        ),
        (
            T::I("http://a.example/s1"),
            "http://a.example/p1",
            T::I("http://a.example/other"),
        ),
    ]);
    // Without EXTRA the non-matching value fails …
    let schema = "<S1> { <p1> [ <http://a.example/v1> ] }";
    assert!(check(schema, &data, iri("http://a.example/s1"), S1).is_err());
    // … with EXTRA it is tolerated.
    let schema = "<S1> EXTRA <p1> { <p1> [ <http://a.example/v1> ] }";
    assert!(check(schema, &data, iri("http://a.example/s1"), S1).is_ok());
    // Vapid EXTRA: EXTRA never diverts an arc that MATCHES the constraint
    // (spec §5.2), so two `.`-matching values still break cardinality {1,1}.
    let schema = "<S1> EXTRA <p1> CLOSED { <p1> . }";
    assert!(check(schema, &data, iri("http://a.example/s1"), S1).is_err());
}

#[test]
fn closed_forbids_unmentioned_predicates() {
    let data = dataset(&[
        (
            T::I("http://a.example/s1"),
            "http://a.example/p1",
            T::I("http://a.example/o1"),
        ),
        (
            T::I("http://a.example/s1"),
            "http://a.example/p2",
            T::I("http://a.example/o2"),
        ),
    ]);
    assert!(check("<S1> { <p1> . }", &data, iri("http://a.example/s1"), S1).is_ok());
    let err = check(
        "<S1> CLOSED { <p1> . }",
        &data,
        iri("http://a.example/s1"),
        S1,
    )
    .expect_err("closed");
    assert!(err.contains("CLOSED"), "reason: {err}");
}

// ── recursion ───────────────────────────────────────────────────────────────

#[test]
fn recursion_over_a_linked_list() {
    // <List> { <first> LITERAL; <rest> [<nil>] OR @<List> } — but written in
    // ShExC as a value-expression OR.
    let schema = "<List> { <first> LITERAL ; <rest> @<List> OR [ <http://a.example/nil> ] }";
    fn cell(n: &str) -> T<'_> {
        T::I(n)
    }
    let data = dataset(&[
        (
            cell("http://a.example/l1"),
            "http://a.example/first",
            T::L(RdfLiteral::simple("a")),
        ),
        (
            cell("http://a.example/l1"),
            "http://a.example/rest",
            cell("http://a.example/l2"),
        ),
        (
            cell("http://a.example/l2"),
            "http://a.example/first",
            T::L(RdfLiteral::simple("b")),
        ),
        (
            cell("http://a.example/l2"),
            "http://a.example/rest",
            cell("http://a.example/nil"),
        ),
    ]);
    assert!(check(
        schema,
        &data,
        iri("http://a.example/l1"),
        "http://a.example/List"
    )
    .is_ok());
    // A broken tail (missing <first>) refutes the whole list.
    let broken = dataset(&[
        (
            cell("http://a.example/l1"),
            "http://a.example/first",
            T::L(RdfLiteral::simple("a")),
        ),
        (
            cell("http://a.example/l1"),
            "http://a.example/rest",
            cell("http://a.example/l2"),
        ),
        (
            cell("http://a.example/l2"),
            "http://a.example/rest",
            cell("http://a.example/nil"),
        ),
    ]);
    assert!(check(
        schema,
        &broken,
        iri("http://a.example/l1"),
        "http://a.example/List"
    )
    .is_err());
}

#[test]
fn cyclic_data_conforms_coinductively() {
    // <S> { <p1> @<S> } over a data cycle s1 → s2 → s1.
    let schema = "<S1> { <p1> @<S1> }";
    let data = dataset(&[
        (
            T::I("http://a.example/s1"),
            "http://a.example/p1",
            T::I("http://a.example/s2"),
        ),
        (
            T::I("http://a.example/s2"),
            "http://a.example/p1",
            T::I("http://a.example/s1"),
        ),
    ]);
    assert!(check(schema, &data, iri("http://a.example/s1"), S1).is_ok());
}

// ── OneOf / EachOf with repeated predicates ─────────────────────────────────

#[test]
fn eachof_partition_with_repeated_predicates() {
    // <p1> [<v1>]; <p1> [<v2>] — the same predicate twice: the two arcs
    // must be routed to the right constraints.
    let schema = "<S1> { <p1> [ <http://a.example/v1> ] ; <p1> [ <http://a.example/v2> ] }";
    let data = dataset(&[
        (
            T::I("http://a.example/s1"),
            "http://a.example/p1",
            T::I("http://a.example/v1"),
        ),
        (
            T::I("http://a.example/s1"),
            "http://a.example/p1",
            T::I("http://a.example/v2"),
        ),
    ]);
    assert!(check(schema, &data, iri("http://a.example/s1"), S1).is_ok());
    let bad = dataset(&[
        (
            T::I("http://a.example/s1"),
            "http://a.example/p1",
            T::I("http://a.example/v1"),
        ),
        (
            T::I("http://a.example/s1"),
            "http://a.example/p1",
            T::I("http://a.example/v3"),
        ),
    ]);
    assert!(check(schema, &bad, iri("http://a.example/s1"), S1).is_err());
}

#[test]
fn oneof_choice_with_group_repetition() {
    // (<p1> [<v1>] | <p1> [<v2>]){2} — two picks across the branches.
    let schema = "<S1> { ( <p1> [ <http://a.example/v1> ] | <p1> [ <http://a.example/v2> ] ){2} }";
    let data = dataset(&[
        (
            T::I("http://a.example/s1"),
            "http://a.example/p1",
            T::I("http://a.example/v1"),
        ),
        (
            T::I("http://a.example/s1"),
            "http://a.example/p1",
            T::I("http://a.example/v2"),
        ),
    ]);
    assert!(check(schema, &data, iri("http://a.example/s1"), S1).is_ok());
    let short = dataset(&[(
        T::I("http://a.example/s1"),
        "http://a.example/p1",
        T::I("http://a.example/v1"),
    )]);
    assert!(check(schema, &short, iri("http://a.example/s1"), S1).is_err());
}

// ── inverse triple constraints ──────────────────────────────────────────────

#[test]
fn inverse_triple_constraint_matches_arcs_in() {
    let schema = "<S1> { ^<p1> IRI }";
    let data = dataset(&[(
        T::I("http://a.example/parent"),
        "http://a.example/p1",
        T::I("http://a.example/s1"),
    )]);
    assert!(check(schema, &data, iri("http://a.example/s1"), S1).is_ok());
    // No incoming arc → cardinality failure.
    assert!(check(schema, &data, iri("http://a.example/parent"), S1).is_err());
    // Arcs-in never violate CLOSED (it constrains arcs-out only).
    let closed = "<S1> CLOSED { ^<p1> IRI }";
    assert!(check(closed, &data, iri("http://a.example/s1"), S1).is_ok());
}

// ── boolean algebra, START, detached focus, EXTERNAL ────────────────────────

#[test]
fn boolean_algebra_and_start() {
    let schema = "start = @<S1> AND NOT @<S2>\n<S1> { <p1> . }\n<S2> { <p2> . }";
    let data = dataset(&[(
        T::I("http://a.example/s1"),
        "http://a.example/p1",
        T::I("http://a.example/o1"),
    )]);
    assert!(check(schema, &data, iri("http://a.example/s1"), "START").is_ok());
    let both = dataset(&[
        (
            T::I("http://a.example/s1"),
            "http://a.example/p1",
            T::I("http://a.example/o1"),
        ),
        (
            T::I("http://a.example/s1"),
            "http://a.example/p2",
            T::I("http://a.example/o2"),
        ),
    ]);
    assert!(check(schema, &both, iri("http://a.example/s1"), "START").is_err());
}

#[test]
fn detached_focus_validates_against_empty_neighbourhood() {
    let data = dataset(&[(
        T::I("http://a.example/x"),
        "http://a.example/p9",
        T::I("http://a.example/y"),
    )]);
    // Empty shape: any node (even one absent from the data) conforms.
    assert!(check("<S1> { }", &data, iri("http://a.example/dummy"), S1).is_ok());
    // A required triple constraint refutes a detached node.
    assert!(check("<S1> { <p1> . }", &data, iri("http://a.example/dummy"), S1).is_err());
}

#[test]
fn external_shapes_use_the_resolver_hook() {
    let schema = parse_shexc("<Sext> EXTERNAL", Some("http://a.example/")).expect("schema");
    let data = dataset(&[(
        T::I("http://a.example/s1"),
        "http://a.example/p2",
        T::I("http://a.example/o1"),
    )]);
    let map = [(
        iri("http://a.example/s1"),
        ShapeSelector::Label("http://a.example/Sext".to_owned()),
    )];
    // Without a resolver the EXTERNAL shape fails.
    let result = validate(&schema, &data, &map);
    assert_eq!(result.entries[0].status, ConformanceStatus::Nonconformant);
    // With one, the resolved definition is enforced.
    let external = parse_shexc("<Sext> { <p2> . }", Some("http://a.example/")).expect("external");
    let resolver = move |label: &str| {
        external
            .shapes
            .iter()
            .find(|d| d.id == label)
            .map(|d| d.expr.clone())
    };
    let options = ValidationOptions {
        external_resolver: Some(&resolver),
    };
    let result = validate_with(&schema, &data, &map, &options);
    assert_eq!(result.entries[0].status, ConformanceStatus::Conformant);
}

#[test]
fn bnode_focus_by_label() {
    let data = dataset(&[(
        T::B("abcd"),
        "http://a.example/p1",
        T::I("http://a.example/o1"),
    )]);
    assert!(check("<S1> { <p1> . }", &data, TermValue::blank("abcd"), S1).is_ok());
}
