// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Pure external FIPS 204 APIs against NIST's independent complete answers.
use purrdf_gts::mldsa65::{
    Error, PUBLIC_KEY_LENGTH, SECRET_KEY_LENGTH, SIGNATURE_LENGTH, Signature, SigningKey,
    VerifyingKey,
};
use purrdf_hash::hex;
use purrdf_testkit::vectors::VectorFile;

fn bytes(field: &str) -> Vec<u8> {
    let text = purrdf_testkit::vectors::decode_str(field).unwrap();
    hex::decode(&text).unwrap()
}
fn equal_bytes(actual: &[u8], expected: &[u8], identity: &str) {
    assert_eq!(actual.len(), expected.len(), "{identity}: byte length");
    let difference = actual.iter().zip(expected).position(|(a, b)| a != b);
    assert_eq!(difference, None, "{identity}: first differing byte");
}

fn nist_key_generation_complete_answers() {
    let vectors = VectorFile::load(include_str!("mldsa65/keyGen.txt"));
    assert_eq!(vectors.records().len(), 25);
    for record in vectors.records() {
        let fields = &record.fields;
        assert_eq!(fields.len(), 5);
        assert_eq!(fields[0], "2");
        let seed = bytes(fields[2]);
        let public = bytes(fields[3]);
        let secret = bytes(fields[4]);
        assert_eq!(public.len(), PUBLIC_KEY_LENGTH);
        assert_eq!(secret.len(), SECRET_KEY_LENGTH);
        let key = SigningKey::from_seed(seed.as_slice().try_into().unwrap()).unwrap();
        equal_bytes(key.verifying_key().as_bytes(), &public, fields[1]);
        let mut expanded = [0; SECRET_KEY_LENGTH];
        key.export_expanded(&mut expanded);
        equal_bytes(&expanded, &secret, fields[1]);
        let imported = SigningKey::from_expanded_bytes(&secret).unwrap();
        equal_bytes(imported.verifying_key().as_bytes(), &public, fields[1]);
        purrdf_ed25519::wipe_secret(&mut expanded);
    }
}

fn nist_deterministic_and_hedged_signing_complete_answers() {
    let vectors = VectorFile::load(include_str!("mldsa65/sigGen.txt"));
    assert_eq!(vectors.records().len(), 30);
    let mut deterministic = 0;
    let mut hedged = 0;
    for record in vectors.records() {
        let fields = &record.fields;
        assert_eq!(fields.len(), 7);
        let secret = bytes(fields[2]);
        let message = bytes(fields[3]);
        let context = bytes(fields[4]);
        let randomizer = bytes(fields[5]);
        let expected = bytes(fields[6]);
        assert_eq!(secret.len(), SECRET_KEY_LENGTH);
        assert_eq!(expected.len(), SIGNATURE_LENGTH);
        let key = SigningKey::from_expanded_bytes(&secret).unwrap();
        let actual = if fields[0] == "3" {
            deterministic += 1;
            assert_eq!(randomizer, [0; 32]);
            key.sign_deterministic(&message, &context).unwrap()
        } else {
            assert_eq!(fields[0], "15");
            hedged += 1;
            key.sign_hedged(
                &message,
                &context,
                randomizer.as_slice().try_into().unwrap(),
            )
            .unwrap()
        };
        equal_bytes(actual.as_bytes(), &expected, fields[1]);
        let decoded = Signature::from_bytes(&expected).unwrap();
        key.verifying_key()
            .verify(&message, &context, &decoded)
            .unwrap();
    }
    assert_eq!((deterministic, hedged), (15, 15));
}

fn nist_valid_and_invalid_verification_answers() {
    let vectors = VectorFile::load(include_str!("mldsa65/sigVer.txt"));
    assert_eq!(vectors.records().len(), 15);
    let mut valid = 0;
    let mut invalid = 0;
    for record in vectors.records() {
        let fields = &record.fields;
        assert_eq!(fields.len(), 7);
        assert_eq!(fields[0], "3");
        let public = VerifyingKey::from_bytes(&bytes(fields[2])).unwrap();
        let signature = Signature::from_bytes(&bytes(fields[5]));
        let observed = signature
            .and_then(|signature| public.verify(&bytes(fields[3]), &bytes(fields[4]), &signature))
            .is_ok();
        let expected: bool = fields[6].parse().unwrap();
        assert_eq!(observed, expected, "NIST verification case {}", fields[1]);
        if expected {
            valid += 1;
        } else {
            invalid += 1;
        }
    }
    assert_eq!((valid, invalid), (3, 12));
}

fn canonical_inputs_and_context_boundaries() {
    let key = SigningKey::from_seed(&[9; 32]).unwrap();
    let public = key.verifying_key();
    for length in [0, PUBLIC_KEY_LENGTH - 1, PUBLIC_KEY_LENGTH + 1] {
        assert_eq!(
            VerifyingKey::from_bytes(&vec![0; length]).unwrap_err(),
            Error::InvalidPublicKey
        );
    }
    for length in [0, SIGNATURE_LENGTH - 1, SIGNATURE_LENGTH + 1] {
        assert_eq!(
            Signature::from_bytes(&vec![0; length]).unwrap_err(),
            Error::InvalidSignature
        );
    }
    for length in [0, SECRET_KEY_LENGTH - 1, SECRET_KEY_LENGTH + 1] {
        assert_eq!(
            SigningKey::from_expanded_bytes(&vec![0; length]).unwrap_err(),
            Error::InvalidSecretKey
        );
    }
    let context = [3; 255];
    let signature = key.sign_deterministic(b"message", &context).unwrap();
    public.verify(b"message", &context, &signature).unwrap();
    assert_eq!(
        key.sign_deterministic(b"message", &[0; 256]).unwrap_err(),
        Error::ContextTooLong
    );
    assert_eq!(
        key.sign_hedged(b"message", &[0; 256], &[7; 32])
            .unwrap_err(),
        Error::ContextTooLong
    );
    assert_eq!(
        public.verify(b"message", &[0; 256], &signature),
        Err(Error::ContextTooLong)
    );
    assert!(public.verify(b"changed", &context, &signature).is_err());
    assert!(
        public
            .verify(b"message", b"other context", &signature)
            .is_err()
    );
    let first = key.sign_hedged(b"message", b"", &[1; 32]).unwrap();
    let second = key.sign_hedged(b"message", b"", &[2; 32]).unwrap();
    assert_ne!(first.as_bytes(), second.as_bytes());
    public.verify(b"message", b"", &first).unwrap();
    public.verify(b"message", b"", &second).unwrap();
    let empty = key.sign_deterministic(b"", b"").unwrap();
    public.verify(b"", b"", &empty).unwrap();
    let mut secret = [0; SECRET_KEY_LENGTH];
    key.export_expanded(&mut secret);
    // rho, public-key hash, both bounded-secret vectors and the t0 low bits.
    for position in [
        0,
        64,
        127,
        128,
        128 + 5 * 128,
        128 + 11 * 128,
        SECRET_KEY_LENGTH - 1,
    ] {
        let mut changed = secret;
        changed[position] ^= 1;
        assert_eq!(
            SigningKey::from_expanded_bytes(&changed).unwrap_err(),
            Error::InvalidSecretKey
        );
        purrdf_ed25519::wipe_secret(&mut changed);
    }
    // All forbidden eta=4 packed values, on both vector sections.
    for position in [128, 128 + 5 * 128] {
        for nibble in 9..16 {
            let mut changed = secret;
            changed[position] = (changed[position] & 0xf0) | nibble;
            assert!(SigningKey::from_expanded_bytes(&changed).is_err());
            purrdf_ed25519::wipe_secret(&mut changed);
        }
    }
    let clone = key.clone();
    assert_eq!(clone.verifying_key(), public);
    assert!(!format!("{key:?}").contains("encoded"));
    purrdf_ed25519::wipe_secret(&mut secret);
}

fn strict_signature_norm_hints_and_tampering() {
    let key = SigningKey::from_seed(&[7; 32]).unwrap();
    let signature = key.sign_deterministic(b"canonical", b"test").unwrap();
    let public = key.verifying_key();
    // Each challenge/response/hint section is authenticated.
    for position in [0, 47, 48, 687, 688, 3247, 3248, SIGNATURE_LENGTH - 1] {
        let mut changed = *signature.as_bytes();
        changed[position] ^= 1;
        assert!(
            Signature::from_bytes(&changed)
                .and_then(|s| public.verify(b"canonical", b"test", &s))
                .is_err()
        );
    }
    let mut changed_public = *public.as_bytes();
    for position in [0, 31, 32, PUBLIC_KEY_LENGTH - 1] {
        changed_public[position] ^= 1;
        let changed = VerifyingKey::from_bytes(&changed_public).unwrap();
        assert!(changed.verify(b"canonical", b"test", &signature).is_err());
        changed_public[position] ^= 1;
    }
    // Invalidate ordering, uniqueness, cumulative offsets and unused storage.
    const HINT: usize = 48 + 5 * 640;
    let base = *signature.as_bytes();
    let mut changed = base;
    changed[HINT..].fill(0);
    assert!(Signature::from_bytes(&changed).is_ok());
    for case in 0..5 {
        let mut malformed = changed;
        match case {
            0 => malformed[HINT + 55] = 56,
            1 => {
                malformed[HINT + 55] = 1;
                malformed[HINT + 56] = 0;
            }
            2 => {
                malformed[HINT..HINT + 2].copy_from_slice(&[5, 5]);
                malformed[HINT + 55..].fill(2);
            }
            3 => {
                malformed[HINT..HINT + 2].copy_from_slice(&[6, 5]);
                malformed[HINT + 55..].fill(2);
            }
            4 => malformed[HINT] = 1,
            _ => unreachable!(),
        }
        assert_eq!(
            Signature::from_bytes(&malformed).unwrap_err(),
            Error::InvalidSignature
        );
    }
    const BOUND: i32 = (1 << 19) - 196;
    for value in [-BOUND - 1, -BOUND, -BOUND + 1, BOUND - 1, BOUND, BOUND + 1] {
        let mut bytes = base;
        let packed = ((1 << 19) - value) as u32;
        bytes[48] = packed as u8;
        bytes[49] = (packed >> 8) as u8;
        bytes[50] = (bytes[50] & 0xf0) | ((packed >> 16) as u8 & 0x0f);
        assert_eq!(
            Signature::from_bytes(&bytes).is_ok(),
            value.abs() < BOUND,
            "z={value}"
        );
    }
}

fn portable_without_host_entropy_or_clock() {
    purrdf_testkit::harness::without_host_clock_or_entropy(|| {
        let key = SigningKey::from_seed(&[3; 32]).unwrap();
        let signature = key.sign_hedged(b"sealed", b"portable", &[4; 32]).unwrap();
        key.verifying_key()
            .verify(b"sealed", b"portable", &signature)
            .unwrap();
    });
}

purrdf_testkit::harness_main!(
    nist_key_generation_complete_answers,
    nist_deterministic_and_hedged_signing_complete_answers,
    nist_valid_and_invalid_verification_answers,
    canonical_inputs_and_context_boundaries,
    strict_signature_norm_hints_and_tampering,
    portable_without_host_entropy_or_clock,
);
