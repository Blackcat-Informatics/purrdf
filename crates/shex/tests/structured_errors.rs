// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original ShEx producer fields through the portable public interface.

use purrdf_hash::error::ErrorValue;
use purrdf_shex::{check_structure, parse_shape_map, parse_shexc};
use purrdf_testkit::error_capture::Capture;

fn actual_shape_map_keeps_original_iri_cause_after_input_disposal() {
    let input = String::from("<alice>@<https://example.org/Shape>");
    let error = parse_shape_map(&input, None).unwrap_err();
    drop(input);
    let copy = error.clone();
    drop(error);
    let fields = Capture::read(&copy);
    assert_eq!(fields.code, "shex-iri");
    assert!(matches!(fields.field("lexical"), ErrorValue::Text("alice")));
    assert_eq!(fields.child("cause").code, "iri-relative-no-base");
    assert!(
        std::error::Error::source(&copy)
            .unwrap()
            .is::<purrdf_iri::IriError>()
    );
}

fn structural_refusals_keep_the_original_unresolved_label() {
    let input = String::from("<https://example.org/Shape> @<https://example.org/missing>");
    let schema = parse_shexc(&input, None).unwrap();
    drop(input);
    let errors = check_structure(&schema).unwrap_err();
    drop(schema);
    assert_eq!(errors.len(), 1);
    let fields = Capture::read(&errors[0]);
    assert_eq!(fields.code, "shex-dangling-shape-reference");
    assert!(matches!(
        fields.field("label"),
        ErrorValue::Text("https://example.org/missing")
    ));
}

purrdf_testkit::harness_main!(
    actual_shape_map_keeps_original_iri_cause_after_input_disposal,
    structural_refusals_keep_the_original_unresolved_label,
);
