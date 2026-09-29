// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Torsion and mixed-order cases, which need the group internals to build,
//! checked against the frozen `ed25519-dalek` reference and paired with the
//! valid neighbour each refusal borders.

use ed25519_dalek::Verifier as _;
use purrdf_testkit::rng::SplitMix64;
use sha2::{Digest as _, Sha512};

use crate::point::Point;
use crate::scalar::Scalar;
use crate::{Signature, SignatureError, SigningKey, VerifyingKey};

/// L - 1 in little-endian bytes.
const L_MINUS_1: [u8; 32] = [
    0xec, 0xd3, 0xf5, 0x5c, 0x1a, 0x63, 0x12, 0x58, 0xd6, 0x9c, 0xf7, 0xa2, 0xde, 0xf9, 0xde, 0x14,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x10,
];

fn random_bytes<const N: usize>(rng: &mut SplitMix64) -> [u8; N] {
    let mut out = [0u8; N];
    for chunk in out.chunks_mut(8) {
        let word = rng.next_u64().to_le_bytes();
        chunk.copy_from_slice(&word[..chunk.len()]);
    }
    out
}

/// The torsion component L·P of a point: (L-1)·P + P.
fn torsion_of(p: &Point) -> Point {
    let l_minus_1 = Scalar::from_canonical_bytes(&L_MINUS_1).expect("L-1 is canonical");
    Point::double_scalar_mul_vartime(&l_minus_1, p, &Scalar::ZERO).sum(p)
}

/// The eight points of order dividing 8, found as the torsion components of
/// decoded points and deduplicated by encoding.
fn torsion_points() -> Vec<[u8; 32]> {
    let mut found = std::collections::BTreeSet::new();
    let mut y = 3u8;
    while found.len() < 8 {
        let mut bytes = [0u8; 32];
        bytes[0] = y;
        y += 1;
        let Some(p) = Point::decode(&bytes) else {
            continue;
        };
        let t = torsion_of(&p);
        let mut multiple = t;
        for _ in 0..8 {
            found.insert(multiple.encode());
            multiple = multiple.sum(&t);
        }
    }
    found.into_iter().collect()
}

fn dalek_verdict(key: &[u8; 32], message: &[u8], signature: &Signature) -> (bool, bool) {
    let Ok(key) = ed25519_dalek::VerifyingKey::from_bytes(key) else {
        return (false, false);
    };
    let signature = ed25519_dalek::Signature::from_bytes(&signature.to_bytes());
    (
        key.verify_strict(message, &signature).is_ok(),
        key.verify(message, &signature).is_ok(),
    )
}

fn ours(key: &[u8; 32], message: &[u8], signature: &Signature) -> Result<(), SignatureError> {
    VerifyingKey::from_bytes(key)?.verify_strict(message, signature)
}

fn challenge(r: &[u8; 32], a: &[u8; 32], message: &[u8]) -> Scalar {
    let mut hasher = Sha512::new();
    hasher.update(r);
    hasher.update(a);
    hasher.update(message);
    Scalar::from_bytes_wide(&hasher.finalize().into())
}

#[test]
fn the_eight_torsion_points_are_weak_keys_for_both_implementations() {
    let torsion = torsion_points();
    assert_eq!(torsion.len(), 8);
    for encoding in &torsion {
        let key = VerifyingKey::from_bytes(encoding).expect("torsion points decode");
        assert!(key.is_weak(), "{encoding:02x?}");
        let dalek = ed25519_dalek::VerifyingKey::from_bytes(encoding).expect("dalek decodes");
        assert!(dalek.is_weak());
    }
    // Neighbour: a prime-order key is not weak.
    assert!(!SigningKey::from_bytes(&[1; 32]).verifying_key().is_weak());
}

#[test]
fn a_small_order_key_is_refused_even_when_the_equation_holds() {
    // A = identity: [S]B = R + [k]·identity holds for R = [S]B and any message.
    let identity = Point::IDENTITY.encode();
    let s = Scalar::from_bytes_mod_order(&[9; 32]);
    let r = Point::mul_base(&s).encode();
    let forged = Signature::from_components(r, s.to_bytes());
    for message in [&b""[..], b"any message at all", b"another"] {
        assert_eq!(
            ours(&identity, message, &forged),
            Err(SignatureError::SmallOrder)
        );
        // The loose rule accepts it; the strict one refuses it.
        assert_eq!(dalek_verdict(&identity, message, &forged), (false, true));
    }
    // Neighbour: the same construction under a prime-order key it does not
    // fit is a plain mismatch, and a real signature from that key verifies.
    let key = SigningKey::from_bytes(&[2; 32]);
    let public = key.verifying_key().to_bytes();
    assert_eq!(ours(&public, b"m", &forged), Err(SignatureError::Mismatch));
    assert_eq!(ours(&public, b"m", &key.sign(b"m")), Ok(()));
}

#[test]
fn a_small_order_r_is_refused_even_when_the_equation_holds() {
    let mut rng = SplitMix64::new(0x7072_5f73_6d61_6c6c);
    for encoding in torsion_points() {
        let seed: [u8; 32] = random_bytes(&mut rng);
        let key = SigningKey::from_bytes(&seed);
        let public = key.verifying_key().to_bytes();
        let message = b"small-order R";
        // With R = T (torsion) and S = k·s, [S]B = [k]A, so the cofactorless
        // equation holds exactly when T is the identity.
        let k = challenge(&encoding, &public, message);
        let s = k.mul(key.secret_scalar());
        let signature = Signature::from_components(encoding, s.to_bytes());
        assert_eq!(
            ours(&public, message, &signature),
            Err(SignatureError::SmallOrder)
        );
        let (strict, loose) = dalek_verdict(&public, message, &signature);
        assert!(!strict);
        assert_eq!(loose, encoding == Point::IDENTITY.encode());
        // Neighbour: an honest signature under the same key verifies.
        assert_eq!(ours(&public, message, &key.sign(message)), Ok(()));
    }
}

#[test]
fn mixed_order_keys_follow_the_cofactorless_equation_like_the_reference() {
    // A' = A + T2 with T2 = (0, -1) of order 2. A signer who knows s signs
    // over A' with S = r + k·s; then [S]B - [k]A' = R - [k]T2, which equals R
    // exactly when k is even. Both parities must occur and both verdicts
    // must agree with the reference.
    let key = SigningKey::from_bytes(&[0x42; 32]);
    let a = Point::decode(&key.verifying_key().to_bytes()).expect("public key decodes");
    let t2 = Point::decode(&{
        let mut minus_one = [0xffu8; 32];
        minus_one[0] = 0xec;
        minus_one[31] = 0x7f;
        minus_one
    })
    .expect("(0, -1) decodes");
    let mixed = a.sum(&t2).encode();
    let mixed_key = VerifyingKey::from_bytes(&mixed).expect("mixed-order points decode");
    assert!(!mixed_key.is_weak());

    let mut accepted = 0;
    let mut refused = 0;
    for i in 0u32..64 {
        let message = i.to_le_bytes();
        let nonce = Scalar::from_bytes_mod_order(&[i as u8 ^ 0x5a; 32]);
        let r = Point::mul_base(&nonce).encode();
        let k = challenge(&r, &mixed, &message);
        let s = k.mul_add(key.secret_scalar(), nonce);
        let signature = Signature::from_components(r, s.to_bytes());
        let verdict = ours(&mixed, &message, &signature);
        let k_even = k.to_bytes()[0] & 1 == 0;
        assert_eq!(verdict.is_ok(), k_even, "message {i}");
        if k_even {
            accepted += 1;
        } else {
            assert_eq!(verdict, Err(SignatureError::Mismatch));
            refused += 1;
        }
        assert_eq!(
            dalek_verdict(&mixed, &message, &signature).0,
            verdict.is_ok()
        );
    }
    assert!(accepted > 0 && refused > 0);
}

#[test]
fn mixed_order_r_never_satisfies_the_cofactorless_equation() {
    let key = SigningKey::from_bytes(&[0x24; 32]);
    let public = key.verifying_key().to_bytes();
    for torsion in torsion_points() {
        let t = Point::decode(&torsion).expect("torsion decodes");
        let nonce = Scalar::from_bytes_mod_order(&[0x33; 32]);
        let r = Point::mul_base(&nonce).sum(&t).encode();
        let message = b"mixed R";
        let k = challenge(&r, &public, message);
        let s = k.mul_add(key.secret_scalar(), nonce);
        let signature = Signature::from_components(r, s.to_bytes());
        let expected = if torsion == Point::IDENTITY.encode() {
            Ok(())
        } else {
            Err(SignatureError::Mismatch)
        };
        assert_eq!(ours(&public, message, &signature), expected);
        assert_eq!(
            dalek_verdict(&public, message, &signature).0,
            expected.is_ok()
        );
    }
}
