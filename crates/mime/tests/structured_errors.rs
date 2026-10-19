// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Exact transfer refusal coordinates from the original covered message.

use purrdf_hash::error::ErrorValue;
use purrdf_mime::{Limits, Profile, SourceDocument, Vocabulary, analyze};
use purrdf_testkit::error_capture::Capture;

fn attachment_refusal_keeps_part_and_absolute_offset() {
    let profile = Profile::new(
        "inspection",
        Vocabulary::under("https://example.org/mime#").unwrap(),
        Limits::unbounded(),
    )
    .unwrap();
    let source = b"Content-Transfer-Encoding: base64\r\n\r\nY===";
    let document = analyze(
        SourceDocument {
            id: "urn:example:mail",
            bytes: source,
        },
        &profile,
    )
    .unwrap();
    let error = document.decoded_attachment(0).unwrap_err();
    drop(document);
    let captured = Capture::read(&error);
    assert_eq!(captured.code, "mime-transfer");
    assert!(matches!(captured.field("part"), ErrorValue::Unsigned(0)));
    assert!(matches!(captured.field("at"), ErrorValue::ByteCount(37)));
    assert!(matches!(
        captured.field("reason"),
        ErrorValue::Text("malformed base64 groups or padding")
    ));
    let valid = analyze(
        SourceDocument {
            id: "urn:example:mail",
            bytes: b"Content-Transfer-Encoding: base64\r\n\r\nYQ==",
        },
        &profile,
    )
    .unwrap();
    assert_eq!(valid.decoded_attachment(0).unwrap().bytes, b"a");
}

purrdf_testkit::harness_main!(attachment_refusal_keeps_part_and_absolute_offset);
