// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Standalone COSE consumers must be able to open existing encrypted objects.

use ciborium::Value;
use purrdf_gts::cose::{Encrypt0Error, decrypt0, encrypt0};
use purrdf_gts::wire;

#[test]
fn standalone_cose_decryption_authenticates_the_exact_plaintext() {
    let key = [37; 32];
    for plaintext in [b"".as_slice(), b"an existing encrypted checkpoint"] {
        // These two fixture inputs have distinct lengths and therefore IVs.
        let iv = [u8::try_from(plaintext.len()).unwrap(); 12];
        let blob = encrypt0(plaintext, "recipient", &key, &iv);
        let opened = decrypt0(&blob, |kid| (kid == "recipient").then_some(key)).unwrap();
        assert_eq!(opened, plaintext);
        assert_eq!(decrypt0(&blob, |_| None), Err(Encrypt0Error::MissingKey));
        assert_eq!(
            decrypt0(&blob, |_| Some([38; 32])),
            Err(Encrypt0Error::AuthFailed)
        );
        let mut tampered = blob;
        *tampered.last_mut().unwrap() ^= 1;
        assert_eq!(
            decrypt0(&tampered, |_| Some(key)),
            Err(Encrypt0Error::AuthFailed)
        );
    }
    assert_eq!(decrypt0(&[], |_| Some(key)), Err(Encrypt0Error::Malformed));
}

#[test]
fn standalone_decryption_opens_the_frozen_encrypt0_vector() {
    let fixture = frozen_fixture();
    let plaintext = decrypt0(&fixture.blob, |found| {
        (found == fixture.kid).then_some(fixture.key)
    })
    .unwrap();
    assert_eq!(plaintext, fixture.plaintext);
}

struct Fixture {
    blob: Vec<u8>,
    key: [u8; 32],
    kid: String,
    plaintext: Vec<u8>,
}

fn frozen_fixture() -> Fixture {
    let vector: serde_json::Value =
        serde_json::from_str(include_str!("../../../vectors/encrypt0/basic.json")).unwrap();
    let bytes = |field: &str| {
        let (pairs, tail) = vector[field].as_str().unwrap().as_bytes().as_chunks::<2>();
        assert_eq!(tail, [0_u8; 0]);
        pairs
            .iter()
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect::<Vec<_>>()
    };
    Fixture {
        blob: bytes("cose"),
        key: bytes("key").try_into().unwrap(),
        kid: vector["kid"].as_str().unwrap().into(),
        plaintext: bytes("plaintext"),
    }
}

fn envelope_parts(blob: &[u8]) -> [Value; 3] {
    let value: Value = ciborium::de::from_reader(blob).unwrap();
    let Value::Tag(16, body) = value else {
        panic!("fixture must be tagged COSE_Encrypt0");
    };
    let Value::Array(parts) = *body else {
        panic!("fixture must contain an array");
    };
    parts.try_into().unwrap()
}

fn encode_envelope(parts: [Value; 3]) -> Vec<u8> {
    wire::encode(&Value::Tag(16, Box::new(Value::Array(parts.into()))))
}

fn headers(parts: &mut [Value; 3]) -> &mut Vec<(Value, Value)> {
    let Value::Map(headers) = &mut parts[1] else {
        panic!("fixture must contain an unprotected header map");
    };
    headers
}

fn set_header(parts: &mut [Value; 3], label: i64, value: Value) {
    let (_, found) = headers(parts)
        .iter_mut()
        .find(|(key, _)| *key == Value::Integer(label.into()))
        .unwrap();
    *found = value;
}

#[test]
fn standalone_decryption_rejects_every_truncated_envelope_before_key_lookup() {
    let fixture = frozen_fixture();
    for length in 0..fixture.blob.len() {
        assert_eq!(
            decrypt0(&fixture.blob[..length], |_| panic!(
                "truncated envelope key lookup"
            )),
            Err(Encrypt0Error::Malformed),
            "encoded length {length}"
        );
    }
}

#[test]
fn standalone_decryption_rejects_malformed_fields_before_key_lookup() {
    let fixture = frozen_fixture();
    let parts = envelope_parts(&fixture.blob);
    let mut malformed = vec![
        Value::Null,
        Value::Bytes(Vec::new()),
        Value::Array(Vec::new()),
        Value::Array(parts[..2].to_vec()),
        Value::Array([parts.to_vec(), vec![Value::Null]].concat()),
    ];
    for index in 0..3 {
        let mut changed = parts.clone();
        changed[index] = Value::Text("wrong type".into());
        malformed.push(Value::Array(changed.into()));
    }
    for label in [4, 5] {
        let mut missing = parts.clone();
        headers(&mut missing).retain(|(key, _)| *key != Value::Integer(label.into()));
        malformed.push(Value::Array(missing.into()));
        let mut wrong_type = parts.clone();
        set_header(&mut wrong_type, label, Value::Text("wrong type".into()));
        malformed.push(Value::Array(wrong_type.into()));
    }
    let mut invalid_kid = parts;
    set_header(&mut invalid_kid, 4, Value::Bytes(vec![0xff]));
    malformed.push(Value::Array(invalid_kid.into()));
    for value in malformed {
        assert_eq!(
            decrypt0(&wire::encode(&value), |_| panic!(
                "malformed envelope key lookup"
            )),
            Err(Encrypt0Error::Malformed),
            "{value:?}"
        );
    }
}

#[test]
fn standalone_decryption_requires_a_twelve_byte_nonce() {
    let fixture = frozen_fixture();
    for length in [0, 1, 11, 13, 16] {
        let mut parts = envelope_parts(&fixture.blob);
        set_header(&mut parts, 5, Value::Bytes(vec![0; length]));
        assert_eq!(
            decrypt0(&encode_envelope(parts), |_| Some(fixture.key)),
            Err(Encrypt0Error::Malformed),
            "nonce length {length}"
        );
    }
}

#[test]
fn standalone_decryption_rejects_ciphertexts_without_a_complete_authentication_tag() {
    let fixture = frozen_fixture();
    for length in 0..16 {
        let mut parts = envelope_parts(&fixture.blob);
        parts[2] = Value::Bytes(vec![0; length]);
        assert_eq!(
            decrypt0(&encode_envelope(parts), |_| Some(fixture.key)),
            Err(Encrypt0Error::AuthFailed),
            "ciphertext length {length}"
        );
    }
}

#[test]
fn standalone_decryption_authenticates_the_original_protected_header_encoding() {
    let fixture = frozen_fixture();
    let mut parts = envelope_parts(&fixture.blob);
    // Both maps mean {1: 3}, but the replacement encodes label 1 over two bytes.
    // Authentication binds the original bytes, not a re-encoded equivalent map.
    assert_eq!(parts[0], Value::Bytes(vec![0xa1, 0x01, 0x03]));
    parts[0] = Value::Bytes(vec![0xa1, 0x18, 0x01, 0x03]);
    assert_eq!(
        decrypt0(&encode_envelope(parts), |_| Some(fixture.key)),
        Err(Encrypt0Error::AuthFailed)
    );
}

#[test]
fn standalone_decryption_authenticates_every_nonce_byte() {
    let fixture = frozen_fixture();
    for index in 0..12 {
        let mut parts = envelope_parts(&fixture.blob);
        let (_, Value::Bytes(nonce)) = headers(&mut parts)
            .iter_mut()
            .find(|(key, _)| *key == Value::Integer(5.into()))
            .unwrap()
        else {
            panic!("fixture nonce must be bytes");
        };
        nonce[index] ^= 1;
        assert_eq!(
            decrypt0(&encode_envelope(parts), |_| Some(fixture.key)),
            Err(Encrypt0Error::AuthFailed),
            "nonce byte {index}"
        );
    }
}

#[test]
fn standalone_decryption_authenticates_every_ciphertext_and_tag_byte() {
    let fixture = frozen_fixture();
    for index in 0..fixture.plaintext.len() + 16 {
        let mut parts = envelope_parts(&fixture.blob);
        let Value::Bytes(ciphertext) = &mut parts[2] else {
            panic!("fixture ciphertext must be bytes");
        };
        ciphertext[index] ^= 1;
        assert_eq!(
            decrypt0(&encode_envelope(parts), |_| Some(fixture.key)),
            Err(Encrypt0Error::AuthFailed),
            "ciphertext/tag byte {index}"
        );
    }
}

#[test]
fn standalone_decryption_accepts_untagged_and_indefinite_length_envelopes() {
    let fixture = frozen_fixture();
    let parts = envelope_parts(&fixture.blob);
    let untagged = wire::encode(&Value::Array(parts.clone().into()));
    let mut indefinite = vec![0x9f];
    for part in parts {
        indefinite.extend(wire::encode(&part));
    }
    indefinite.push(0xff);
    for blob in [untagged, indefinite] {
        assert_eq!(
            decrypt0(&blob, |_| Some(fixture.key)).unwrap(),
            fixture.plaintext
        );
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn fresh_key() -> [u8; 32] {
    let mut key = [0; 32];
    getrandom::fill(&mut key).expect("test encryption randomness");
    key
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn standalone_decryption_passes_recipient_identity_to_the_key_resolver_verbatim() {
    let key = fresh_key();
    let plaintext = b"binary\0plaintext\xff";
    for (index, kid) in ["", " recipient ", "recipient\0key", "鍵/🐈"]
        .into_iter()
        .enumerate()
    {
        // Distinct fixture nonces for each encryption under this key.
        let iv = [u8::try_from(index).unwrap(); 12];
        let blob = encrypt0(plaintext, kid, &key, &iv);
        let calls = std::cell::Cell::new(0);
        let opened = decrypt0(&blob, |found| {
            calls.set(calls.get() + 1);
            assert_eq!(found, kid);
            Some(key)
        });
        assert_eq!(opened.unwrap(), plaintext);
        assert_eq!(calls.get(), 1);
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn standalone_decryption_preserves_binary_plaintext_across_block_and_length_boundaries() {
    let key = fresh_key();
    for (index, length) in [1, 15, 16, 17, 255, 256, 257, 4095, 4096, 4097, 65536]
        .into_iter()
        .enumerate()
    {
        let plaintext: Vec<u8> = (0..=255).cycle().take(length).collect();
        let iv = [u8::try_from(index).unwrap(); 12];
        let blob = encrypt0(&plaintext, "binary", &key, &iv);
        assert_eq!(
            decrypt0(&blob, |_| Some(key)).unwrap(),
            plaintext,
            "plaintext length {length}"
        );
    }
}
