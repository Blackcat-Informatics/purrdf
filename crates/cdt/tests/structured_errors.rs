// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Composite parser and comparison failures through the generic typed consumer.

use purrdf_hash::error::ErrorValue;
use purrdf_testkit::error_capture::Capture;

#[test]
fn duplicate_key_retains_original_offset_and_canonical_lexical_key() {
    let error = purrdf_cdt::parse_map("{1: true, 1: false}")
        .expect_err("repeated identical map keys must be refused");
    let record = Capture::read(&error);
    assert_eq!(record.code, "cdt-duplicate-map-key");
    assert_eq!(record.events.len(), 2);
    assert!(matches!(record.field("offset"), ErrorValue::ByteCount(10)));
    let expected = format!("\"1\"^^<{}>", purrdf_xsd::datatype::XSD_INTEGER);
    assert!(matches!(record.field("key"), ErrorValue::Text(key) if key == expected));
}

#[test]
fn comparison_refusal_retains_classification_and_original_reason() {
    let left = purrdf_cdt::parse_list("[\"x\"^^<http://example.org/unmodelled>]")
        .expect("a well-formed list");
    let right = purrdf_cdt::parse_list("[\"y\"^^<http://example.org/unmodelled>]")
        .expect("a well-formed list");
    let error = purrdf_cdt::value_equal(&left, &right)
        .expect_err("the custom value space has no equality implementation");
    let record = Capture::read(&error);
    assert_eq!(record.code, "cdt-type");
    assert_eq!(record.events.len(), 2);
    assert_eq!(record.child("kind").code, "cdt-type-unmodelled");
    assert!(matches!(record.field("reason"), ErrorValue::Text(reason) if reason == error.reason));
}
