// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Standalone COSE consumers must be able to open existing encrypted objects.

use purrdf_gts::cose::{Encrypt0Error, decrypt0, encrypt0};

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
    let key: [u8; 32] = bytes("key").try_into().unwrap();
    let kid = vector["kid"].as_str().unwrap();
    let plaintext = decrypt0(&bytes("cose"), |found| (found == kid).then_some(key)).unwrap();
    assert_eq!(plaintext, bytes("plaintext"));
}
