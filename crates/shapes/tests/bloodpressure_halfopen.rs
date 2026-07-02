// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The sharp ADL→FOL fidelity test: the half-open magnitude interval, read
//! from the vendored OPT itself rather than transcribed by hand.
//!
//! This test drives [`purrdf_shapes::openehr_opt`] end to end against the
//! actual `validations/openehr-bloodpressure/Blutdruck.opt` file: it parses
//! the systolic (`at0004`) and diastolic (`at0005`) `C_DV_QUANTITY` magnitude
//! intervals directly out of the OPT's XML, lowers the systolic interval to a
//! SHACL shape, and validates data against the *generated* shape — not a
//! hand-copied constant. If the OPT ever changes its boundary inclusivity or
//! bounds, this test reads the new values and (dis)proves the invariant
//! against them, rather than silently continuing to check a stale constant.
//!
//! The Blutdruck OPT constrains both the systolic and diastolic magnitude
//! with `lower_included=true`, **`upper_included=false`** — a half-open
//! `[lo, hi)`. The concrete bounds are `lower=0`, `upper=1000`, `units=mm[Hg]`.
//!
//! Lowering that ADL constraint to SHACL must regenerate the boundary
//! inclusivity EXACTLY: `lower_included=true → sh:minInclusive`,
//! `upper_included=false → sh:maxExclusive` (NEVER `sh:maxInclusive`). This
//! is the sharp test that `u ∘ d = id` holds on the *constraint* and not
//! merely the data — an off-by-one on the open boundary would silently admit
//! `value == hi`. The checks below cover both the structural lowering (the
//! parsed shape carries MaxExclusive, never MaxInclusive) and the enforced
//! semantics (`value == hi` violates; `value == lo` and `value < hi` conform).

use std::path::PathBuf;

use purrdf_shapes::engine::{parse_shapes, validate_graphs};
use purrdf_shapes::model::sh::MAX_EXCLUSIVE_CONSTRAINT_COMPONENT;
use purrdf_shapes::openehr_opt::{lower_magnitude_to_shacl_ttl, read_magnitude_interval};
use purrdf_shapes::shapes::Constraint;

/// Caller-supplied prefix bindings for the CURIEs passed to
/// `lower_magnitude_to_shacl_ttl` (PurRDF mints no vocabulary of its own).
const TEST_PREFIXES: &[(&str, &str)] = &[
    ("meta", "https://example.org/meta/"),
    ("ex", "https://purrdf.example/openehr/bp/"),
];

/// Reads the vendored Blutdruck OPT from disk, relative to this crate's
/// manifest directory (`crates/shacl` → `../../validations/openehr-bloodpressure`).
fn read_blutdruck_opt() -> String {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("../../validations/openehr-bloodpressure/Blutdruck.opt");
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

fn data_node(local: &str, value: &str) -> String {
    format!(
        "<https://purrdf.example/openehr/bp/{local}> \
         <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> \
         <https://example.org/meta/SystolicMeasurement> .\n\
         <https://purrdf.example/openehr/bp/{local}> \
         <https://example.org/meta/quantityValue> \
         \"{value}\"^^<http://www.w3.org/2001/XMLSchema#decimal> .\n"
    )
}

#[test]
fn systolic_and_diastolic_intervals_read_from_opt_match() {
    let opt = read_blutdruck_opt();

    let systolic = read_magnitude_interval(&opt, "at0004").expect("read systolic magnitude");
    assert!(
        systolic.lower_included,
        "systolic lower_included must be true, read {systolic:?}"
    );
    assert!(
        !systolic.upper_included,
        "systolic upper_included must be false (half-open), read {systolic:?}"
    );
    assert_eq!(
        systolic.lower, 0.0,
        "systolic lower bound, read {systolic:?}"
    );
    assert_eq!(
        systolic.upper, 1000.0,
        "systolic upper bound, read {systolic:?}"
    );
    assert_eq!(
        systolic.units, "mm[Hg]",
        "systolic units, read {systolic:?}"
    );

    let diastolic = read_magnitude_interval(&opt, "at0005").expect("read diastolic magnitude");
    assert!(
        diastolic.lower_included,
        "diastolic lower_included must be true, read {diastolic:?}"
    );
    assert!(
        !diastolic.upper_included,
        "diastolic upper_included must be false (half-open), read {diastolic:?}"
    );
    assert_eq!(
        diastolic, systolic,
        "diastolic must match the systolic [0, 1000) mm[Hg] interval"
    );
}

#[test]
fn half_open_lowers_to_max_exclusive_never_max_inclusive() {
    let opt = read_blutdruck_opt();
    let systolic = read_magnitude_interval(&opt, "at0004").expect("read systolic magnitude");
    let shapes_ttl = lower_magnitude_to_shacl_ttl(
        &systolic,
        "meta:SystolicMeasurement",
        "meta:quantityValue",
        "ex:SystolicMeasurementShape",
        TEST_PREFIXES,
    );

    let shapes = parse_shapes(&shapes_ttl).expect("parse systolic shapes");
    let constraints: Vec<&Constraint> = shapes
        .node_shapes
        .iter()
        .flat_map(|n| n.property_shapes.iter())
        .flat_map(|p| p.constraints.iter())
        .collect();

    let has_min_inclusive = constraints
        .iter()
        .any(|c| matches!(c, Constraint::MinInclusive(_)));
    let has_max_exclusive = constraints
        .iter()
        .any(|c| matches!(c, Constraint::MaxExclusive(_)));
    let has_max_inclusive = constraints
        .iter()
        .any(|c| matches!(c, Constraint::MaxInclusive(_)));

    assert!(
        has_min_inclusive,
        "lower_included=true must lower to sh:minInclusive"
    );
    assert!(
        has_max_exclusive,
        "upper_included=false must lower to sh:maxExclusive"
    );
    assert!(
        !has_max_inclusive,
        "upper_included=false must NOT regenerate sh:maxInclusive (the off-by-one ADL leak)"
    );
}

#[test]
fn value_equal_to_open_upper_bound_is_rejected() {
    // value == hi (1000) must VIOLATE under maxExclusive — the boundary that distinguishes
    // the half-open [lo, hi) from a closed [lo, hi]. value == lo (0) and value < hi conform.
    let opt = read_blutdruck_opt();
    let systolic = read_magnitude_interval(&opt, "at0004").expect("read systolic magnitude");
    let shapes_ttl = lower_magnitude_to_shacl_ttl(
        &systolic,
        "meta:SystolicMeasurement",
        "meta:quantityValue",
        "ex:SystolicMeasurementShape",
        TEST_PREFIXES,
    );

    let data = format!(
        "{}{}{}",
        data_node("atLowerBound", "0"), // == lo: minInclusive admits it
        data_node("belowUpper", "999"), // < hi: inside the interval
        data_node("atUpperBound", "1000"), // == hi: maxExclusive rejects it
    );
    let report = validate_graphs(&data, &shapes_ttl).expect("validate");

    assert!(
        !report.conforms,
        "value == hi must make the graph non-conformant"
    );

    assert_eq!(
        report.results.len(),
        1,
        "exactly one violation (value == hi); lo and below-hi must pass"
    );
    let v = &report.results[0];
    assert_eq!(
        v.source_constraint_component.as_str(),
        MAX_EXCLUSIVE_CONSTRAINT_COMPONENT,
        "the violation must be a MaxExclusiveConstraintComponent"
    );
    assert!(
        v.focus_node.to_string().contains("atUpperBound"),
        "the violating focus must be the value-at-hi node, got {:?}",
        v.focus_node
    );
}
