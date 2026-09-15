// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Differential coverage against the complete CBOR decoder and its error order.

use std::cell::RefCell;

use super::*;

fn fixture() -> (Vec<u8>, [u8; 32]) {
    let value: serde_json::Value =
        serde_json::from_str(include_str!("../../../../vectors/encrypt0/basic.json")).unwrap();
    let bytes = |field: &str| {
        value[field]
            .as_str()
            .unwrap()
            .as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect::<Vec<_>>()
    };
    (bytes("cose"), bytes("key").try_into().unwrap())
}

fn parts(blob: &[u8]) -> [Value; 3] {
    let Value::Tag(_, value) = ciborium::de::from_reader(blob).unwrap() else {
        panic!("fixture must have an outer tag");
    };
    let Value::Array(value) = *value else {
        panic!("fixture must contain an array");
    };
    value.try_into().unwrap()
}

fn envelope(parts: [Value; 3]) -> Vec<u8> {
    wire::encode(&Value::Tag(16, Box::new(Value::Array(parts.into()))))
}

fn differential(blob: &[u8], key: [u8; 32]) {
    let mut wrong = key;
    wrong[0] ^= 1;
    for key in [None, Some(key), Some(wrong)] {
        for limit in [0, 1, 15, 16, 17, usize::MAX] {
            let reference_calls = RefCell::new(Vec::new());
            let reference = parse_encrypt0_owned(blob)
                .ok_or(BoundedDecrypt0Error::Crypto(Encrypt0Error::Malformed))
                .and_then(|parts| {
                    decrypt_parts(
                        parts,
                        |kid| {
                            reference_calls.borrow_mut().push(kid.to_owned());
                            key
                        },
                        limit,
                    )
                });
            let actual_calls = RefCell::new(Vec::new());
            let actual = decrypt0_bounded(
                blob,
                |kid| {
                    actual_calls.borrow_mut().push(kid.to_owned());
                    key
                },
                limit,
            );
            assert_eq!(actual, reference, "limit {limit}, input {blob:02x?}");
            assert_eq!(actual_calls, reference_calls, "input {blob:02x?}");
        }
    }
    assert_eq!(
        recipient_kid(blob),
        parse_encrypt0_owned(blob).map(|parts| parts.kid.into_owned())
    );
}

#[test]
fn borrowed_parser_matches_complete_decoder_on_truncation_and_byte_mutations() {
    let (blob, key) = fixture();
    for length in 0..=blob.len() {
        differential(&blob[..length], key);
    }
    for index in 0..blob.len() {
        for byte in [
            0, 1, 4, 5, 16, 23, 24, 27, 28, 31, 0x40, 0x5f, 0x60, 0x7f, 0x80, 0x83, 0x9f, 0xa0,
            0xa2, 0xbf, 0xc0, 0xd0, 0xdf, 0xe0, 0xf8, 0xf9, 0xfa, 0xfb, 0xff,
        ] {
            let mut changed = blob.clone();
            changed[index] = byte;
            differential(&changed, key);
        }
    }
}

fn indefinite_bytes(bytes: &[u8]) -> Vec<u8> {
    let mut encoded = vec![0x5f];
    for chunk in bytes.chunks(2) {
        encoded.extend(wire::encode(&Value::Bytes(chunk.to_vec())));
    }
    encoded.push(0xff);
    encoded
}

#[test]
fn complete_decoder_preserves_extended_envelope_forms() {
    let (blob, key) = fixture();
    let parts = parts(&blob);
    let mut cases = vec![blob.clone()];
    for tag in [0, 1, 2, 3, 16, 17, 23, 24, 255, 65536, u64::MAX] {
        cases.push(wire::encode(&Value::Tag(
            tag,
            Box::new(Value::Array(parts.to_vec())),
        )));
    }
    cases.push(wire::encode(&Value::Tag(
        16,
        Box::new(Value::Tag(16, Box::new(Value::Array(parts.to_vec())))),
    )));
    for tail in [vec![0xff], vec![0x5b, 0xff], vec![0, 1, 2]] {
        cases.push([blob.clone(), tail].concat());
    }
    let Value::Map(headers) = &parts[1] else {
        panic!("fixture headers must be a map");
    };
    for indefinite_array in [false, true] {
        for indefinite_map in [false, true] {
            for indefinite_strings in [false, true] {
                let encode = |value: &Value| match value {
                    Value::Bytes(bytes) if indefinite_strings => indefinite_bytes(bytes),
                    _ => wire::encode(value),
                };
                let mut raw = vec![0xd0, if indefinite_array { 0x9f } else { 0x83 }];
                raw.extend(encode(&parts[0]));
                raw.push(if indefinite_map { 0xbf } else { 0xa2 });
                for (label, value) in headers {
                    raw.extend(encode(label));
                    raw.extend(encode(value));
                }
                if indefinite_map {
                    raw.push(0xff);
                }
                raw.extend(encode(&parts[2]));
                if indefinite_array {
                    raw.push(0xff);
                }
                cases.push(raw);
            }
        }
    }
    // Non-shortest definite argument widths are valid CBOR, including a
    // 64-bit ciphertext length that is checked against the actual input span.
    let mut wide = vec![0xd8, 16, 0x98, 3];
    wide.extend(wire::encode(&parts[0]));
    wide.extend(wire::encode(&parts[1]));
    let ciphertext = parts[2].as_bytes().unwrap();
    wide.push(0x5b);
    wide.extend(u64::try_from(ciphertext.len()).unwrap().to_be_bytes());
    wide.extend(ciphertext);
    cases.push(wide);
    for case in cases {
        differential(&case, key);
    }
}

#[test]
fn header_selection_matches_duplicate_and_unknown_field_behavior() {
    let (blob, key) = fixture();
    let original = parts(&blob);
    let headers = original[1].as_map().unwrap();
    let kid = headers[0].1.clone();
    let iv = headers[1].1.clone();
    let variants = [
        vec![
            (4.into(), Value::Bytes(vec![0xff])),
            (4.into(), kid.clone()),
        ],
        vec![(4.into(), Value::Text("ignored".into())), (4.into(), kid)],
        vec![
            (5.into(), Value::Bytes(vec![0; 11])),
            (5.into(), iv.clone()),
        ],
        vec![(5.into(), Value::Text("ignored".into())), (5.into(), iv)],
        vec![(4.into(), Value::Bytes(b"first recipient".to_vec()))],
        vec![(99.into(), Value::Bytes(vec![0xff]))],
        vec![(
            Integer::from(-1),
            Value::Array(vec![Value::Null, Value::Bool(true)]),
        )],
        vec![(
            99.into(),
            Value::Tag(42, Box::new(Value::Text("extension".into()))),
        )],
    ];
    for prefix in variants {
        let mut changed = original.clone();
        changed[1] = Value::Map(
            prefix
                .into_iter()
                .map(|(label, value)| (Value::Integer(label), value))
                .chain(headers.iter().cloned())
                .collect(),
        );
        differential(&envelope(changed), key);
    }
}

#[test]
fn unknown_headers_are_fully_validated_before_key_resolution() {
    let (blob, key) = fixture();
    let parts = parts(&blob);
    let mut prefix = vec![0xd0, 0x83];
    prefix.extend(wire::encode(&parts[0]));
    prefix.push(0xa3);
    for (label, value) in parts[1].as_map().unwrap() {
        prefix.extend(wire::encode(label));
        prefix.extend(wire::encode(value));
    }
    prefix.extend([0x18, 99]);
    let mut nested = vec![0x81; 257];
    nested.push(0);
    for invalid in [
        vec![0x61, 0xff],             // Invalid UTF-8 in an ignored text value.
        vec![0xff],                   // Break outside an indefinite container.
        vec![0x5f, 0x61, b'x', 0xff], // A text chunk inside a byte string.
        vec![0x5b, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
        nested,
    ] {
        let mut malformed = prefix.clone();
        malformed.extend(invalid);
        malformed.extend(wire::encode(&parts[2]));
        assert!(parse_encrypt0_owned(&malformed).is_none());
        assert!(matches!(
            decrypt0_bounded(&malformed, |_| panic!("invalid extension key lookup"), 0),
            Err(BoundedDecrypt0Error::Crypto(Encrypt0Error::Malformed))
        ));
        differential(&malformed, key);
    }
}

#[test]
fn borrowed_aad_serialization_matches_the_original_byte_structure() {
    for length in [0, 1, 23, 24, 255, 256, 65535, 65536] {
        let protected: Vec<u8> = (0..=255).cycle().take(length).collect();
        let reference = wire::encode(&Value::Array(vec![
            Value::Text("Encrypt0".into()),
            Value::Bytes(protected.clone()),
            Value::Bytes(Vec::new()),
        ]));
        assert_eq!(enc_structure(&protected), reference);
    }
}

#[test]
fn definite_decryption_allocates_only_the_plaintext_capacity() {
    let (blob, key) = fixture();
    let parsed = borrowed::parse(&blob).expect("writer envelope must borrow");
    assert!(matches!(parsed.kid, Cow::Borrowed(_)));
    assert!(matches!(parsed.protected, Cow::Borrowed(_)));
    assert!(matches!(parsed.iv, Cow::Borrowed(_)));
    assert!(matches!(parsed.ciphertext, Cow::Borrowed(_)));
    let plaintext_len = parsed.ciphertext.len() - 16;
    for limit in [plaintext_len, plaintext_len + 1] {
        let plaintext = decrypt0_bounded(&blob, |_| Some(key), limit).unwrap();
        assert_eq!(plaintext.len(), plaintext_len);
        assert_eq!(plaintext.capacity(), plaintext_len);
    }
    assert!(matches!(
        decrypt0_bounded(&blob, |_| Some(key), plaintext_len - 1),
        Err(BoundedDecrypt0Error::Limit)
    ));
}

#[test]
fn decryption_failure_order_is_parse_key_nonce_tag_limit_then_authentication() {
    use BoundedDecrypt0Error::{Crypto, Limit};

    let (blob, key) = fixture();
    let original = parts(&blob);
    let mut short_tag = original.clone();
    short_tag[2] = Value::Bytes(vec![0; 15]);
    let mut short_iv = short_tag.clone();
    let Value::Map(headers) = &mut short_iv[1] else {
        panic!("fixture headers must be a map");
    };
    let (_, iv) = headers
        .iter_mut()
        .find(|(label, _)| *label == Value::Integer(IV.into()))
        .unwrap();
    *iv = Value::Bytes(vec![0; 11]);
    let short_iv = envelope(short_iv);
    assert_eq!(
        decrypt0_bounded(&short_iv, |_| None, 0),
        Err(Crypto(Encrypt0Error::MissingKey))
    );
    assert_eq!(
        decrypt0_bounded(&short_iv, |_| Some(key), 0),
        Err(Crypto(Encrypt0Error::Malformed))
    );
    assert_eq!(
        decrypt0_bounded(&envelope(short_tag), |_| Some(key), 0),
        Err(Crypto(Encrypt0Error::AuthFailed))
    );
    let plaintext_len = original[2].as_bytes().unwrap().len() - 16;
    let mut wrong = key;
    wrong[0] ^= 1;
    assert_eq!(
        decrypt0_bounded(&blob, |_| Some(wrong), plaintext_len - 1),
        Err(Limit)
    );
    assert_eq!(
        decrypt0_bounded(&blob, |_| Some(wrong), plaintext_len),
        Err(Crypto(Encrypt0Error::AuthFailed))
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn protected_header_bytes_are_opaque_even_when_they_are_not_cbor() {
    let mut key = [0; 32];
    getrandom::fill(&mut key).expect("test encryption randomness");
    let cipher = Aes256Gcm::new((&key).into());
    for (index, protected) in [
        Vec::new(),
        vec![0xff],
        vec![0x7f, 0x61],
        vec![0xa1, 0x18, 1, 3],
    ]
    .into_iter()
    .enumerate()
    {
        let iv = [u8::try_from(index).unwrap(); 12];
        let ciphertext = cipher
            .encrypt(
                Nonce::from_slice(&iv),
                Payload {
                    msg: b"opaque protected bytes",
                    aad: &enc_structure(&protected),
                },
            )
            .unwrap();
        let encoded = envelope([
            Value::Bytes(protected),
            Value::Map(vec![
                (
                    Value::Integer(4.into()),
                    Value::Bytes(b"recipient".to_vec()),
                ),
                (Value::Integer(5.into()), Value::Bytes(iv.to_vec())),
            ]),
            Value::Bytes(ciphertext),
        ]);
        assert_eq!(
            decrypt0(&encoded, |_| Some(key)).unwrap(),
            b"opaque protected bytes"
        );
        differential(&encoded, key);
    }
}
