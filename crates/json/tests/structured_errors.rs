// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native failures from the public ordered JSON reader.

use purrdf_hash::error::ErrorValue;
use purrdf_json::{Bounds, Profile, SourceDocument, Vocabulary, analyze};
use purrdf_testkit::error_capture::Capture;

fn source_refusals_keep_offsets_and_declared_limits() {
    let mut bounds = Bounds::standard();
    bounds.max_source_bytes = 4;
    let profile = Profile::new(
        "inspection",
        1,
        Vocabulary::under("https://example.org/json#").unwrap(),
        bounds,
    )
    .unwrap();
    for (bytes, code, field, expected) in [
        (&b"\xff"[..], "json-cover-invalid-utf8", "valid_up_to", 0),
        (&b"{]"[..], "json-cover-syntax", "at", 1),
    ] {
        let error = analyze(
            SourceDocument {
                id: "urn:example:json",
                bytes,
            },
            &profile,
        )
        .unwrap_err();
        let captured = Capture::read(&error);
        assert_eq!(captured.code, code);
        assert!(matches!(captured.field(field), ErrorValue::ByteCount(value) if value == expected));
    }
    let error = analyze(
        SourceDocument {
            id: "urn:example:json",
            bytes: b"false",
        },
        &profile,
    )
    .unwrap_err();
    let captured = Capture::read(&error);
    assert_eq!(captured.code, "json-cover-limit");
    assert!(matches!(
        captured.field("resource"),
        ErrorValue::Code("source bytes")
    ));
    assert!(matches!(captured.field("limit"), ErrorValue::Unsigned(4)));
    assert!(
        analyze(
            SourceDocument {
                id: "urn:example:json",
                bytes: b"null"
            },
            &profile
        )
        .is_ok()
    );
}

purrdf_testkit::harness_main!(source_refusals_keep_offsets_and_declared_limits);
