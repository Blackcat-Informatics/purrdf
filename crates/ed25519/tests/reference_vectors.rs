// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The frozen reference vectors in `tests/vectors/`, replayed: public keys,
//! signatures and strict-verify verdicts recorded from `ed25519-dalek` 3 over
//! seeded keys and messages and their corruptions, the malleability, R
//! encoding and small-order edge cases, and point decoding over every
//! encoding near the edges of the field. A disagreement is a defect here,
//! never a reason to edit a vector; each file's header says what it covers.

use purrdf_ed25519::{Signature, SignatureError, SigningKey, VerifyingKey};
use purrdf_testkit::rng::SplitMix64;
use purrdf_testkit::vectors::{VectorFile, decode_str, encode_str};

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

/// A byte string as a vector field: lowercase hex.
fn hex(bytes: &[u8]) -> String {
    encode_str(&purrdf_hash::hex::encode(bytes))
}

/// A lowercase-hex vector field as bytes.
fn unhex(field: &str) -> Vec<u8> {
    purrdf_hash::hex::decode(&decode_str(field).expect("an encoded field")).expect("a hex field")
}

fn unhex_array<const N: usize>(field: &str) -> [u8; N] {
    unhex(field).try_into().expect("a fixed-length hex field")
}

fn verdict(accepted: bool) -> String {
    if accepted { "accept" } else { "refuse" }.to_owned()
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
fn two_thousand_seeded_keys_and_messages_match_the_frozen_reference_exactly() {
    let file =
        VectorFile::parse(include_str!("vectors/signing_vectors.txt")).expect("signing vectors");
    assert_eq!(file.records().len(), 2000);
    let mut rng = SplitMix64::new(0x6564_3235_3531_3921);
    let mut case = 0usize;
    let replayed = file
        .replay(4, |inputs| {
            // The inputs are the seeded stream's; regenerate the message the
            // record names only by length.
            let mut seed = [0u8; 32];
            fill(&mut rng, &mut seed);
            let mut message = vec![0u8; (rng.next_u64() % 300) as usize];
            fill(&mut rng, &mut message);
            let target = rng.next_u64();
            let regenerated = [
                case.to_string(),
                hex(&seed),
                message.len().to_string(),
                target.to_string(),
            ];
            assert_eq!(
                inputs,
                regenerated.iter().map(String::as_str).collect::<Vec<_>>(),
                "the seeded stream drifted from the frozen inputs"
            );
            case += 1;

            let key = SigningKey::from_bytes(&seed);
            assert_eq!(key.to_bytes(), seed);
            let public = key.verifying_key().to_bytes();
            let signature = key.sign(&message).to_bytes();
            let honest = ours_strict(&public, &message, &signature);

            // One corrupted bit, in the signature, the key or the message.
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
            vec![
                hex(&public),
                hex(&signature),
                verdict(honest),
                verdict(ours_strict(&key, &msg, &sig)),
            ]
        })
        .unwrap_or_else(|mismatch| panic!("{mismatch}"));
    assert_eq!(replayed, 2000);
    // Every honest triple verifies and every corruption is refused.
    for record in file.records() {
        assert_eq!(
            record.fields[6..],
            ["accept", "refuse"],
            "line {}",
            record.line
        );
    }
}

#[test]
fn verification_edge_cases_match_the_frozen_reference() {
    let file =
        VectorFile::parse(include_str!("vectors/verify_vectors.txt")).expect("verify vectors");
    let replayed = file
        .replay(4, |inputs| {
            let accepted = ours_strict(
                &unhex_array(inputs[1]),
                &unhex(inputs[2]),
                &unhex_array(inputs[3]),
            );
            vec![verdict(accepted)]
        })
        .unwrap_or_else(|mismatch| panic!("{mismatch}"));
    assert_eq!(replayed, file.records().len());
    // The honest signatures are the only acceptances: S + L, every forged R
    // and every signature under a small-order key are refusals.
    for record in file.records() {
        let expected = if matches!(record.fields[0], "s" | "r-honest") {
            "accept"
        } else {
            "refuse"
        };
        assert_eq!(record.fields[4], expected, "{}", record.fields[0]);
    }
}

#[test]
fn s_plus_l_is_refused_as_non_canonical_while_s_verifies() {
    let key = SigningKey::from_bytes(&[0x11; 32]);
    let message = b"malleability";
    let signature = key.sign(message);
    assert_eq!(
        key.verifying_key().verify_strict(message, &signature),
        Ok(())
    );
    let mut raw = signature.to_bytes();
    let s: [u8; 32] = raw[32..].try_into().expect("32 bytes");
    raw[32..].copy_from_slice(&add_le(&s, &L));
    assert_eq!(
        key.verifying_key()
            .verify_strict(message, &Signature::from_bytes(&raw)),
        Err(SignatureError::NonCanonicalS)
    );
}

#[test]
fn key_decoding_matches_the_frozen_reference_except_where_rfc_8032_refuses() {
    let file = VectorFile::parse(include_str!("vectors/key_decoding_vectors.txt"))
        .expect("key decoding vectors");
    let mut reference_only = Vec::new();
    for record in file.records() {
        let [encoding, decoded, class] = record.fields[..] else {
            panic!("line {}: three fields expected", record.line);
        };
        let bytes: [u8; 32] = unhex_array(encoding);
        let ours = VerifyingKey::from_bytes(&bytes);
        let non_canonical_y = {
            let mut y = bytes;
            y[31] &= 0x7f;
            // y >= p: compare big-endian.
            y.iter().rev().cmp(P.iter().rev()).is_ge()
        };
        let negative_zero = bytes[31] & 0x80 != 0 && {
            let mut unsigned = bytes;
            unsigned[31] &= 0x7f;
            unsigned == small(1) || {
                let mut minus_one = P;
                minus_one[0] -= 1;
                unsigned == minus_one
            }
        };
        if non_canonical_y || negative_zero {
            assert_eq!(ours, Err(SignatureError::InvalidPublicKey), "{encoding}");
            if decoded != "refuse" {
                reference_only.push((bytes, class == "weak"));
            }
        } else {
            let answer = ours.map_or_else(
                |_| ["refuse".to_owned(), "-".to_owned()],
                |key| {
                    [
                        hex(&key.to_bytes()),
                        if key.is_weak() { "weak" } else { "strong" }.to_owned(),
                    ]
                },
            );
            assert_eq!(answer, [decoded, class], "line {}: {encoding}", record.line);
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
    // The small-order encodings repeat some edge encodings.
    reference_only.sort_unstable();
    reference_only.dedup();
    expected.sort_unstable();
    assert_eq!(reference_only, expected);
}
