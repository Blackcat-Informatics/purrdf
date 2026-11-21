// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The protected header of a detached COSE_Sign1.
//!
//! Specification section 4 requires every hashed or signed byte string to use
//! deterministic encoding, so a protected header that does not is refused with
//! its own cause. RFC 9052 section 3.1 types `alg` as an integer or a text
//! string, so a text algorithm is a sound envelope with an algorithm this
//! engine does not implement, and the `kid` of any sound envelope is reported.
//! Every envelope below carries a signature that is valid over its protected
//! bytes, so only the header decides the outcome.

use purrdf_ed25519::SigningKey;
use purrdf_gts::compact::CompactRefusedError;
use purrdf_gts::cose::{self, SigStatus, Sign1Error};
use purrdf_gts::{model, wire};
use purrdf_lex::cbor::Value;

const FRAME: &[u8] = b"frame-id";
const KID: &[u8] = b"test-kid";

fn key() -> SigningKey {
    SigningKey::from_bytes(&[7; 32])
}

/// A tagged envelope over `protected` exactly as given, signed with a valid
/// Ed25519 signature over the Sig_structure of those bytes.
fn envelope(protected: &[u8]) -> Vec<u8> {
    let signature = key()
        .sign(&cose::sig_structure(protected, FRAME))
        .to_bytes()
        .to_vec();
    wire::encode(&Value::Tag(
        18,
        Box::new(Value::Array(vec![
            Value::Bytes(protected.to_vec()),
            Value::Map(vec![(4.into(), Value::Bytes(KID.to_vec()))]),
            Value::Null,
            Value::Bytes(signature),
        ])),
    ))
}

fn status_of(envelope: &[u8]) -> (String, Option<String>) {
    let mut rows = [model::Signature {
        frame_id: FRAME.to_vec(),
        kid: None,
        status: String::new(),
        cose: Some(envelope.to_vec()),
        packaging: false,
    }];
    let public = key().verifying_key();
    cose::verify_signatures(&mut rows, |kid| (kid == "test-kid").then_some(public));
    let [row] = rows;
    (row.status, row.kid)
}

/// The deterministic header a conforming writer emits: `{1: -8}`.
const CANONICAL: &[u8] = &[0xa1, 0x01, 0x27];

#[test]
fn the_deterministic_header_verifies_valid() {
    let envelope = envelope(CANONICAL);
    assert_eq!(
        status_of(&envelope),
        ("valid".into(), Some("test-kid".into()))
    );
    assert_eq!(cose::signature_refusal(&envelope), None);
}

/// Each header below decodes to the same map `{1: -8}` or an equivalent one,
/// yet is not the deterministic encoding of it. A valid signature over any of
/// them is `invalid`, with a cause a caller can read, and the cause is not the
/// one a damaged envelope reports.
#[test]
fn a_header_that_is_not_deterministically_encoded_is_invalid_with_its_own_cause() {
    let cases: [(&str, &[u8]); 5] = [
        (
            "alg as an eight-byte integer",
            &[0xa1, 0x01, 0x3b, 0, 0, 0, 0, 0, 0, 0, 0x07],
        ),
        ("indefinite-length map", &[0xbf, 0x01, 0x27, 0xff]),
        ("map count in a following byte", &[0xb8, 0x01, 0x01, 0x27]),
        (
            "label as a one-byte integer",
            &[0xa2, 0x18, 0x01, 0x27, 0x03, 0x00],
        ),
        // `{3: 0, 1: -8}`: the key 3 sorts after the key 1 bytewise.
        (
            "map keys out of bytewise order",
            &[0xa2, 0x03, 0x00, 0x01, 0x27],
        ),
    ];
    for (name, protected) in cases {
        let envelope = envelope(protected);
        assert_eq!(
            cose::parse_sign1(&envelope).err(),
            Some(Sign1Error::NonCanonicalProtectedHeader),
            "{name}"
        );
        assert_eq!(
            cose::signature_refusal(&envelope),
            Some(Sign1Error::NonCanonicalProtectedHeader),
            "{name}"
        );
        let public = key().verifying_key();
        assert_eq!(
            cose::verify_sig(&envelope, FRAME, &public),
            SigStatus::Invalid,
            "{name}"
        );
        assert_eq!(status_of(&envelope).0, "invalid", "{name}");
    }
}

/// A damaged envelope and a non-canonical one are both `invalid`, and only the
/// cause tells them apart.
#[test]
fn a_damaged_envelope_does_not_report_the_non_canonical_cause() {
    let attached = wire::encode(&Value::Tag(
        18,
        Box::new(Value::Array(vec![
            Value::Bytes(CANONICAL.to_vec()),
            Value::Map(vec![]),
            Value::Bytes(b"attached".to_vec()),
            Value::Bytes(vec![0; 64]),
        ])),
    ));
    assert_eq!(
        cose::signature_refusal(&attached),
        Some(Sign1Error::Malformed)
    );
    assert_eq!(status_of(&attached).0, "invalid");
}

/// The header grammar admits integers from -2^63 to 2^63-1, as a label, a value
/// or anywhere inside a nested value. One past either end is malformed; the
/// integers at the ends are accepted.
#[test]
fn a_header_integer_outside_the_signed_64_bit_range_is_malformed() {
    let over_max: &[u8] = &[0xa2, 0x01, 0x27, 0x1b, 0x80, 0, 0, 0, 0, 0, 0, 0, 0x00];
    let under_min: &[u8] = &[0xa2, 0x01, 0x27, 0x3b, 0x80, 0, 0, 0, 0, 0, 0, 0, 0x00];
    for (name, protected) in [("2^63", over_max), ("-2^63-1", under_min)] {
        assert_eq!(
            cose::signature_refusal(&envelope(protected)),
            Some(Sign1Error::Malformed),
            "{name}"
        );
    }
    // `{1: -8, 99: [2^64-1]}` and `{1: -8, 99: {1: -2^64}}`: deterministic, with
    // an out-of-range integer one and two levels below the top-level map.
    let nested_array: &[u8] = &[
        0xa2, 0x01, 0x27, 0x18, 0x63, 0x81, 0x1b, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    ];
    let nested_map: &[u8] = &[
        0xa2, 0x01, 0x27, 0x18, 0x63, 0xa1, 0x01, 0x3b, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff,
    ];
    for (name, protected) in [("nested array", nested_array), ("nested map", nested_map)] {
        assert_eq!(
            cose::signature_refusal(&envelope(protected)),
            Some(Sign1Error::Malformed),
            "{name}"
        );
    }
    let nested_in_range: &[u8] = &[
        0xa2, 0x01, 0x27, 0x18, 0x63, 0x81, 0x1b, 0x7f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    ];
    assert_eq!(cose::signature_refusal(&envelope(nested_in_range)), None);
    let max: &[u8] = &[
        0xa2, 0x01, 0x27, 0x1b, 0x7f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x00,
    ];
    let min: &[u8] = &[
        0xa2, 0x01, 0x27, 0x3b, 0x7f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x00,
    ];
    for (name, protected) in [("2^63-1", max), ("-2^63", min)] {
        let envelope = envelope(protected);
        assert_eq!(cose::signature_refusal(&envelope), None, "{name}");
        assert_eq!(status_of(&envelope).0, "valid", "{name}");
    }
}

/// An algorithm this engine does not implement, named by an integer or by a
/// text string, is `unverified` and keeps its `kid`.
#[test]
fn an_unsupported_algorithm_is_unverified_and_keeps_its_kid() {
    let integer: &[u8] = &[0xa1, 0x01, 0x38, 0x62]; // {1: -99}
    let text: &[u8] = &[0xa1, 0x01, 0x63, b'f', b'o', b'o']; // {1: "foo"}
    for (name, protected) in [("integer", integer), ("text", text)] {
        let envelope = envelope(protected);
        assert_eq!(
            cose::parse_sign1(&envelope).err(),
            Some(Sign1Error::UnsupportedAlgorithm),
            "{name}"
        );
        assert_eq!(
            status_of(&envelope),
            ("unverified".into(), Some("test-kid".into())),
            "{name}"
        );
    }
}

/// A header whose `alg` is neither an integer nor a text string is malformed.
#[test]
fn an_algorithm_that_is_neither_integer_nor_text_is_malformed() {
    let bytes_alg: &[u8] = &[0xa1, 0x01, 0x41, 0x00]; // {1: h'00'}
    assert_eq!(
        cose::signature_refusal(&envelope(bytes_alg)),
        Some(Sign1Error::Malformed)
    );
}

/// A file whose signature has a protected header that is not deterministically
/// encoded is reported by the file verifier with its own cause, and compaction
/// refuses it; the same file with a deterministic header and a signature that
/// does not verify is `invalid` without that cause and compacts.
#[test]
fn a_non_canonical_header_is_named_by_the_file_verifier_and_refused_by_compaction() {
    use purrdf_gts::compact::detached_signature_pairs;
    use purrdf_gts::reader::read;
    use purrdf_gts::verify::{Keyring, VerificationError, verify_file_with_keyring};
    use purrdf_gts::writer::{FrameOptions, Writer};

    // The writer declares its transport key, so the verifier reaches the
    // signatures; the explicit frame signature is the one under test.
    let file = |protected: &[u8]| {
        let mut writer = Writer::new("generic");
        writer.sign_with(key(), "test-kid");
        writer
            .add_frame_with_options(
                "meta",
                FrameOptions {
                    payload: Some(Value::Map(vec![("k".into(), "v".into())])),
                    signature: Some(envelope(protected)),
                    ..FrameOptions::default()
                },
            )
            .expect("a frame with an explicit signature writes");
        writer.into_bytes()
    };
    let mut keyring = Keyring::default();
    keyring.insert("test-kid", key().verifying_key());

    let non_canonical = file(&[0xa1, 0x01, 0x3b, 0, 0, 0, 0, 0, 0, 0, 0x07]);
    let result = verify_file_with_keyring(&non_canonical, &keyring);
    assert!(
        result.errors.iter().any(|error| matches!(
            error,
            VerificationError::NonCanonicalProtectedHeader { count: 1 }
        )),
        "{:?}",
        result.errors
    );
    assert_eq!(result.invalid, 1);
    let graph = read(&non_canonical, true, None);
    assert!(matches!(
        detached_signature_pairs(&graph),
        Err(CompactRefusedError::FrameCose(
            Sign1Error::NonCanonicalProtectedHeader
        ))
    ));

    // A deterministic header is not named by the cause. This envelope is signed
    // over a fixed test frame id, not the file's frame id, so it is `invalid` for
    // that reason alone.
    let canonical = file(CANONICAL);
    let result = verify_file_with_keyring(&canonical, &keyring);
    assert_eq!(result.valid, 0, "{:?}", result.errors);
    assert_eq!(result.invalid, 1, "{:?}", result.errors);
    assert!(
        !result
            .errors
            .iter()
            .any(|error| matches!(error, VerificationError::NonCanonicalProtectedHeader { .. })),
        "{:?}",
        result.errors
    );
    let graph = read(&canonical, true, None);
    assert_eq!(detached_signature_pairs(&graph).expect("compacts").len(), 1);
}
