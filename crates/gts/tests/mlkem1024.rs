// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native FIPS 203 complete NIST answers and private-key admission boundary.

use purrdf_gts::mlkem1024::{
    CIPHERTEXT_LENGTH, Ciphertext, DECAPSULATION_KEY_LENGTH, DecapsulationKey,
    ENCAPSULATION_KEY_LENGTH, EncapsulationKey, SEED_LENGTH,
};
use purrdf_hash::SecretArray;
use purrdf_hash::sha3::{Sha3_256, Shake256};
use purrdf_testkit::vectors::VectorFile;

#[path = "support/crypto_vectors.rs"]
mod crypto_vectors;
use crypto_vectors::{bytes, equal_bytes};

fn nist_key_generation_complete_answers() {
    let vectors = VectorFile::load(include_str!("mlkem1024/keyGen.txt"));
    assert_eq!(vectors.records().len(), 25);
    for record in vectors.records() {
        let fields = &record.fields;
        assert_eq!(fields.len(), 6);
        let mut seed = SecretArray::new([0; SEED_LENGTH]);
        seed[..32].copy_from_slice(&bytes(fields[2]));
        seed[32..].copy_from_slice(&bytes(fields[3]));
        let key = DecapsulationKey::from_seed(&seed);
        let public = key.encapsulation_key().unwrap();
        equal_bytes(public.as_bytes(), &bytes(fields[4]), fields[1]);
        let mut expanded = SecretArray::new([0; DECAPSULATION_KEY_LENGTH]);
        key.export_expanded(&mut expanded);
        equal_bytes(&expanded[..], &bytes(fields[5]), fields[1]);
        let imported = DecapsulationKey::from_expanded_bytes(&expanded[..]).unwrap();
        equal_bytes(
            imported.encapsulation_key().unwrap().as_bytes(),
            public.as_bytes(),
            fields[1],
        );
    }
}

fn nist_encapsulation_complete_answers() {
    let vectors = VectorFile::load(include_str!("mlkem1024/encapsulation.txt"));
    assert_eq!(vectors.records().len(), 25);
    for record in vectors.records() {
        let fields = &record.fields;
        assert_eq!(fields.len(), 6);
        let key = EncapsulationKey::from_bytes(&bytes(fields[2])).unwrap();
        let randomizer = bytes(fields[3]);
        let (ciphertext, secret) = key.encapsulate(randomizer.as_slice().try_into().unwrap());
        equal_bytes(ciphertext.as_bytes(), &bytes(fields[4]), fields[1]);
        equal_bytes(secret.as_bytes(), &bytes(fields[5]), fields[1]);
    }
}

fn nist_decapsulation_complete_answers() {
    let vectors = VectorFile::load(include_str!("mlkem1024/decapsulation.txt"));
    assert_eq!(vectors.records().len(), 10);
    for record in vectors.records() {
        let fields = &record.fields;
        assert_eq!(fields.len(), 5);
        let key = DecapsulationKey::from_expanded_bytes(&bytes(fields[2])).unwrap();
        let ciphertext = Ciphertext::from_bytes(&bytes(fields[3])).unwrap();
        let secret = key.decapsulate(&ciphertext);
        equal_bytes(secret.as_bytes(), &bytes(fields[4]), fields[1]);
    }
}

fn nist_encapsulation_key_checks() {
    let vectors = VectorFile::load(include_str!("mlkem1024/encapsulationKeyCheck.txt"));
    assert_eq!(vectors.records().len(), 10);
    for record in vectors.records() {
        let fields = &record.fields;
        assert_eq!(fields.len(), 4);
        assert_eq!(
            EncapsulationKey::from_bytes(&bytes(fields[2])).is_ok(),
            fields[3].parse::<bool>().unwrap(),
            "{}: FIPS public key check",
            fields[1],
        );
    }
}

fn nist_decapsulation_key_checks() {
    let vectors = VectorFile::load(include_str!("mlkem1024/decapsulationKeyCheck.txt"));
    assert_eq!(vectors.records().len(), 10);
    for record in vectors.records() {
        let fields = &record.fields;
        assert_eq!(fields.len(), 4);
        assert_eq!(
            DecapsulationKey::from_expanded_bytes(&bytes(fields[2])).is_ok(),
            fields[3].parse::<bool>().unwrap(),
            "{}: FIPS private key check",
            fields[1],
        );
    }
}

fn hash_consistent_private_key_preserves_raw_embedded_encodings() {
    let vectors = VectorFile::load(include_str!("mlkem1024/keyGen.txt"));
    let fields = &vectors.records()[0].fields;
    let mut expanded = bytes(fields[5]);
    let public_start = DECAPSULATION_KEY_LENGTH - ENCAPSULATION_KEY_LENGTH - 64;
    let hash_start = public_start + ENCAPSULATION_KEY_LENGTH;
    // ByteDecode_12 reduces secret coefficients modulo q. Section 7.3 imposes
    // no modulus check on them or on the embedded public encoding.
    for position in [0, public_start] {
        expanded[position] = 1;
        expanded[position + 1] = (expanded[position + 1] & 0xf0) | 0x0d;
    }
    let public_hash = Sha3_256::digest(&expanded[public_start..hash_start]);
    expanded[hash_start..hash_start + 32].copy_from_slice(&public_hash);
    let key = DecapsulationKey::from_expanded_bytes(&expanded).unwrap();
    assert!(key.encapsulation_key().is_err());
    let ciphertext = Ciphertext::from_bytes(&[0; CIPHERTEXT_LENGTH]).unwrap();
    let actual = key.decapsulate(&ciphertext);
    let mut expected = [0; 32];
    let mut rejection = Shake256::new();
    rejection.update(&expanded[hash_start + 32..]);
    rejection.update(ciphertext.as_bytes());
    rejection.finalize().squeeze(&mut expected);
    equal_bytes(
        actual.as_bytes(),
        &expected,
        "hash-consistent raw private key",
    );
}

purrdf_testkit::harness_main!(
    nist_key_generation_complete_answers,
    nist_encapsulation_complete_answers,
    nist_decapsulation_complete_answers,
    nist_encapsulation_key_checks,
    nist_decapsulation_key_checks,
    hash_consistent_private_key_preserves_raw_embedded_encodings,
);
