// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original authority failures and base provenance, consumed without enum mirrors.

use purrdf_hash::error::ErrorValue;
use purrdf_testkit::error_capture::Capture;

#[test]
fn all_authority_producers_retain_typed_reasons_and_previous_messages() {
    for (input, code, message) in [
        (
            "http://[::1]tail",
            "iri-authority-trailing-ip-literal",
            "trailing characters after IP-literal",
        ),
        (
            "http://[::1",
            "iri-authority-unterminated-ip-literal",
            "unterminated IP-literal '['",
        ),
        (
            "http://[v1]",
            "iri-authority-ipvfuture-version",
            "IPvFuture at byte 8 requires a version and '.'",
        ),
        (
            "http://[v1.]",
            "iri-authority-ipvfuture-empty",
            "IPvFuture at byte 8 needs a nonempty version and address",
        ),
        (
            "http://[badé]",
            "iri-authority-invalid-ipv6",
            "invalid IPv6 address \"badé\" at byte 8",
        ),
    ] {
        let error =
            purrdf_iri::parse(input).expect_err("the authority production refuses this input");
        let record = Capture::read(&error);
        assert_eq!(record.code, "iri-bad-authority");
        assert_eq!(record.events.len(), 1);
        let cause = record.child("cause");
        assert_eq!(cause.code, code);
        if code == "iri-authority-invalid-ipv6" {
            assert_eq!(cause.events.len(), 2);
            assert!(matches!(cause.field("address"), ErrorValue::Text("badé")));
            assert!(matches!(cause.field("offset"), ErrorValue::ByteCount(8)));
        }
        // This separately checks compatibility of human presentation, after
        // the typed consumer has already inspected the original native fields.
        assert_eq!(error.to_string(), format!("malformed authority: {message}"));
        assert_eq!(error.presentation().english(), error.to_string());
    }
}

#[test]
fn absolute_only_refusal_keeps_original_reference_base_and_directive_position() {
    use purrdf_iri::{BaseIri, BaseOrigin, BaseScope};
    let scope = BaseScope::rooted(
        BaseIri::parse("http://example.org/base/").expect("an absolute base"),
        BaseOrigin::Directive {
            line: u64::MAX,
            column: u32::MAX,
        },
    );
    let error = scope
        .resolve_absolute_only("relative")
        .expect_err("the grammar requires an absolute IRI");
    let record = Capture::read(&error);
    assert_eq!(record.code, "iri-not-absolute-by-grammar");
    assert!(matches!(
        record.field("reference"),
        ErrorValue::Iri("relative")
    ));
    let base = record.child("base");
    assert_eq!(base.code, "iri-base-in-force");
    assert!(matches!(
        base.field("iri"),
        ErrorValue::Iri("http://example.org/base/")
    ));
    let origin = base.child("origin");
    assert_eq!(origin.code, "iri-base-directive");
    assert!(matches!(origin.field("line"), ErrorValue::Unsigned(n) if n == u128::from(u64::MAX)));
    assert!(matches!(origin.field("column"), ErrorValue::Unsigned(n) if n == u128::from(u32::MAX)));
}
