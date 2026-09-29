// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A frozen differential against `ed25519-dalek` (pinned by Cargo.lock):
//! identical public keys, identical signatures and identical strict-verify
//! verdicts over seeded keys and messages and their corruptions, and point
//! decoding compared over every encoding near the edges of the field.

use purrdf_ed25519::{Signature, SignatureError, SigningKey, VerifyingKey};
use purrdf_testkit::rng::SplitMix64;

/// p = 2^255 - 19 in little-endian bytes.
const P: [u8; 32] = {
    let mut p = [0xff; 32];
    p[0] = 0xed;
    p[31] = 0x7f;
    p
};

/// The group order L in little-endian bytes.
const L: [u8; 32] = [
    0xed, 0xd3, 0xf5, 0x5c, 0x1a, 0x63, 0x12, 0x58, 0xd6, 0x9c, 0xf7, 0xa2, 0xde, 0xf9, 0xde, 0x14,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x10,
];

fn fill(rng: &mut SplitMix64, out: &mut [u8]) {
    for chunk in out.chunks_mut(8) {
        let word = rng.next_u64().to_le_bytes();
        chunk.copy_from_slice(&word[..chunk.len()]);
    }
}

fn dalek_strict(key: &[u8; 32], message: &[u8], signature: &[u8; 64]) -> bool {
    ed25519_dalek::VerifyingKey::from_bytes(key).is_ok_and(|key| {
        key.verify_strict(message, &ed25519_dalek::Signature::from_bytes(signature))
            .is_ok()
    })
}

fn ours_strict(key: &[u8; 32], message: &[u8], signature: &[u8; 64]) -> bool {
    VerifyingKey::from_bytes(key).is_ok_and(|key| {
        key.verify_strict(message, &Signature::from_bytes(signature))
            .is_ok()
    })
}

/// Little-endian 256-bit addition, wrapping.
fn add_le(a: &[u8; 32], b: &[u8; 32]) -> [u8; 32] {
    let mut out = [0u8; 32];
    let mut carry = 0u16;
    for i in 0..32 {
        let t = u16::from(a[i]) + u16::from(b[i]) + carry;
        out[i] = t as u8;
        carry = t >> 8;
    }
    out
}

fn small(n: u8) -> [u8; 32] {
    let mut out = [0u8; 32];
    out[0] = n;
    out
}

#[test]
fn two_thousand_seeded_keys_and_messages_match_the_reference_exactly() {
    use ed25519_dalek::Signer as _;
    let mut rng = SplitMix64::new(0x6564_3235_3531_3921);
    for case in 0..2000 {
        let mut seed = [0u8; 32];
        fill(&mut rng, &mut seed);
        let mut message = vec![0u8; (rng.next_u64() % 300) as usize];
        fill(&mut rng, &mut message);

        let ours = SigningKey::from_bytes(&seed);
        let reference = ed25519_dalek::SigningKey::from_bytes(&seed);
        let public = ours.verifying_key().to_bytes();
        assert_eq!(
            public,
            reference.verifying_key().to_bytes(),
            "case {case}: public key"
        );
        assert_eq!(ours.to_bytes(), seed);

        let signature = ours.sign(&message).to_bytes();
        assert_eq!(
            signature,
            reference.sign(&message).to_bytes(),
            "case {case}: signature"
        );
        assert!(ours_strict(&public, &message, &signature), "case {case}");
        assert!(dalek_strict(&public, &message, &signature), "case {case}");

        // One corrupted bit, in the signature, the key or the message: the
        // verdicts must still agree (and are refusals).
        let target = rng.next_u64();
        let bit = (target >> 8) as usize;
        let (mut key, mut msg, mut sig) = (public, message.clone(), signature);
        match target % 3 {
            0 => sig[(bit / 8) % 64] ^= 1 << (bit % 8),
            1 => key[(bit / 8) % 32] ^= 1 << (bit % 8),
            _ if !msg.is_empty() => {
                let at = (bit / 8) % msg.len();
                msg[at] ^= 1 << (bit % 8);
            }
            _ => msg.push(0),
        }
        let verdict = ours_strict(&key, &msg, &sig);
        assert_eq!(
            verdict,
            dalek_strict(&key, &msg, &sig),
            "case {case}: corrupted verdict"
        );
        assert!(!verdict, "case {case}: a corruption verified");
    }
}

#[test]
fn s_plus_l_is_refused_by_both_while_s_verifies() {
    let key = SigningKey::from_bytes(&[0x11; 32]);
    let public = key.verifying_key().to_bytes();
    let message = b"malleability";
    let signature = key.sign(message);
    let mut raw = signature.to_bytes();
    assert!(ours_strict(&public, message, &raw));
    assert!(dalek_strict(&public, message, &raw));

    let s: [u8; 32] = raw[32..].try_into().expect("32 bytes");
    raw[32..].copy_from_slice(&add_le(&s, &L));
    assert!(!ours_strict(&public, message, &raw));
    assert!(!dalek_strict(&public, message, &raw));
    assert_eq!(
        key.verifying_key()
            .verify_strict(message, &Signature::from_bytes(&raw)),
        Err(SignatureError::NonCanonicalS)
    );
}

#[test]
fn non_canonical_r_encodings_are_refused_by_both() {
    let key = SigningKey::from_bytes(&[0x12; 32]);
    let public = key.verifying_key().to_bytes();
    let message = b"R encodings";
    let honest = key.sign(message).to_bytes();
    assert!(ours_strict(&public, message, &honest));
    for offset in 0u8..19 {
        for sign in [0u8, 0x80] {
            // y = p + offset, which reduces to the small y = offset.
            let mut r = add_le(&P, &small(offset));
            r[31] |= sign;
            let mut forged = honest;
            forged[..32].copy_from_slice(&r);
            assert!(
                !ours_strict(&public, message, &forged),
                "p+{offset} sign {sign}"
            );
            assert!(
                !dalek_strict(&public, message, &forged),
                "p+{offset} sign {sign}"
            );
        }
    }
    // Flipping R's sign bit names a different, canonical point: refused too.
    let mut flipped = honest;
    flipped[31] ^= 0x80;
    assert!(!ours_strict(&public, message, &flipped));
    assert!(!dalek_strict(&public, message, &flipped));
}

#[test]
fn small_order_keys_are_refused_by_both_strict_verifiers() {
    const TORSION: [&str; 8] = [
        "0000000000000000000000000000000000000000000000000000000000000000",
        "0000000000000000000000000000000000000000000000000000000000000080",
        "0100000000000000000000000000000000000000000000000000000000000000",
        "26e8958fc2b227b045c3f489f2ef98f0d5dfac05d3c63339b13802886d53fc05",
        "26e8958fc2b227b045c3f489f2ef98f0d5dfac05d3c63339b13802886d53fc85",
        "c7176a703d4dd84fba3c0b760d10670f2a2053fa2c39ccc64ec7fd7792ac037a",
        "c7176a703d4dd84fba3c0b760d10670f2a2053fa2c39ccc64ec7fd7792ac03fa",
        "ecffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f",
    ];
    let mut rng = SplitMix64::new(0x746f_7273_696f_6e21);
    for encoding in TORSION {
        let key: [u8; 32] = purrdf_hash::hex::decode(encoding)
            .expect("hex")
            .try_into()
            .expect("32 bytes");
        let parsed = VerifyingKey::from_bytes(&key).expect("torsion points decode");
        assert!(parsed.is_weak());
        assert!(
            ed25519_dalek::VerifyingKey::from_bytes(&key)
                .expect("reference decodes")
                .is_weak()
        );
        for _ in 0..16 {
            let mut signature = [0u8; 64];
            fill(&mut rng, &mut signature);
            signature[63] &= 0x0f; // canonical S
            let message = b"weak key";
            assert!(!ours_strict(&key, message, &signature));
            assert!(!dalek_strict(&key, message, &signature));
        }
    }
}

#[test]
fn key_decoding_matches_the_reference_except_where_rfc_8032_refuses() {
    // Every y in [0, 64), [p - 64, p) and [p, 2^255), each with both sign
    // bits, plus random encodings.
    let mut candidates = Vec::new();
    for n in 0u8..64 {
        candidates.push(small(n));
        let mut below_p = P;
        below_p[0] -= n + 1;
        candidates.push(below_p);
    }
    for n in 0u8..19 {
        candidates.push(add_le(&P, &small(n)));
    }
    let mut rng = SplitMix64::new(0x6465_636f_6465_2121);
    for _ in 0..2000 {
        let mut bytes = [0u8; 32];
        fill(&mut rng, &mut bytes);
        bytes[31] &= 0x7f;
        candidates.push(bytes);
    }

    let mut reference_only = Vec::new();
    for unsigned in candidates {
        for sign in [0u8, 0x80] {
            let mut bytes = unsigned;
            bytes[31] |= sign;
            let reference = ed25519_dalek::VerifyingKey::from_bytes(&bytes);
            let ours = VerifyingKey::from_bytes(&bytes);
            let non_canonical_y = {
                let mut y = bytes;
                y[31] &= 0x7f;
                // y >= p: compare big-endian.
                y.iter().rev().cmp(P.iter().rev()).is_ge()
            };
            let negative_zero = sign != 0
                && (unsigned == small(1)
                    || unsigned == {
                        let mut minus_one = P;
                        minus_one[0] -= 1;
                        minus_one
                    });
            if non_canonical_y || negative_zero {
                assert_eq!(ours, Err(SignatureError::InvalidPublicKey), "{bytes:02x?}");
                if let Ok(key) = reference {
                    reference_only.push((bytes, key.is_weak()));
                }
            } else {
                assert_eq!(ours.is_ok(), reference.is_ok(), "{bytes:02x?}");
                if let (Ok(ours), Ok(reference)) = (ours, reference) {
                    assert_eq!(ours.to_bytes(), reference.to_bytes());
                    assert_eq!(ours.is_weak(), reference.is_weak());
                }
            }
        }
    }

    // The documented difference, exactly: the reference also accepts y = p + t
    // for t in {0, 1, 3, 4, 5, 6, 9, 10, 14, 15, 16, 18} with either sign, and
    // x = 0 with the sign bit set (y = 1, y = p - 1); of those, the ones naming
    // small-order points are the reduced y of 0 or 1 and the x = 0 encodings.
    let mut expected = Vec::new();
    for t in [0u8, 1, 3, 4, 5, 6, 9, 10, 14, 15, 16, 18] {
        for sign in [0u8, 0x80] {
            let mut bytes = add_le(&P, &small(t));
            bytes[31] |= sign;
            // p + 1 with the sign bit is x = 0 with sign set: the reference
            // accepts it too (as the identity).
            expected.push((bytes, t <= 1));
        }
    }
    let mut identity_signed = small(1);
    identity_signed[31] |= 0x80;
    expected.push((identity_signed, true));
    let mut minus_one_signed = P;
    minus_one_signed[0] -= 1;
    minus_one_signed[31] |= 0x80;
    expected.push((minus_one_signed, true));
    reference_only.sort_unstable();
    expected.sort_unstable();
    assert_eq!(reference_only, expected);
}
