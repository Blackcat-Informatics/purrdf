// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Wycheproof-style edge cases built from RFC 8032 itself. Every refusal is
//! paired with the valid neighbour on the other side of its boundary, and the
//! neighbour is executed and must succeed (or fail only for a later, named
//! reason, showing the boundary under test let it through).

use purrdf_ed25519::{
    PUBLIC_KEY_LENGTH, SIGNATURE_LENGTH, Signature, SignatureError, SigningKey, VerifyingKey,
};

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

fn small(n: u8) -> [u8; 32] {
    let mut out = [0u8; 32];
    out[0] = n;
    out
}

fn with_top(mut bytes: [u8; 32], bits: u8) -> [u8; 32] {
    bytes[31] |= bits;
    bytes
}

fn fixture() -> (SigningKey, VerifyingKey, &'static [u8], Signature) {
    let key = SigningKey::from_bytes(&[0x5a; 32]);
    let public = key.verifying_key();
    let message: &'static [u8] = b"edge cases";
    let signature = key.sign(message);
    (key, public, message, signature)
}

#[test]
fn s_equal_to_l_is_refused_and_l_minus_1_passes_the_scalar_gate() {
    let (_, public, message, signature) = fixture();
    let r = *signature.r_bytes();
    assert_eq!(
        public.verify_strict(message, &Signature::from_components(r, L)),
        Err(SignatureError::NonCanonicalS)
    );
    let mut l_minus_1 = L;
    l_minus_1[0] -= 1;
    // L - 1 is canonical, so the refusal moves on to the equation.
    assert_eq!(
        public.verify_strict(message, &Signature::from_components(r, l_minus_1)),
        Err(SignatureError::Mismatch)
    );
    assert_eq!(public.verify_strict(message, &signature), Ok(()));
}

#[test]
fn s_with_any_of_the_top_three_bits_set_is_refused() {
    let (_, public, message, signature) = fixture();
    let (r, s) = (*signature.r_bytes(), *signature.s_bytes());
    for bits in [0x20u8, 0x40, 0x80, 0xe0] {
        assert_eq!(
            public.verify_strict(message, &Signature::from_components(r, with_top(s, bits))),
            Err(SignatureError::NonCanonicalS),
            "{bits:#x}"
        );
    }
    assert_eq!(
        public.verify_strict(message, &Signature::from_components(r, s)),
        Ok(())
    );
}

#[test]
fn an_r_with_y_at_or_above_p_is_refused_and_its_reduced_form_decodes() {
    let (_, public, message, signature) = fixture();
    let s = *signature.s_bytes();
    // y = 3 lies on the curve; p + 3 is its non-canonical encoding.
    let mut p_plus_3 = P;
    p_plus_3[0] += 3;
    assert_eq!(
        public.verify_strict(message, &Signature::from_components(p_plus_3, s)),
        Err(SignatureError::InvalidR)
    );
    // The canonical y = 3 decodes; the signature then fails only the equation.
    assert_eq!(
        public.verify_strict(message, &Signature::from_components(small(3), s)),
        Err(SignatureError::Mismatch)
    );
}

#[test]
fn an_r_off_the_curve_is_refused_and_an_on_curve_neighbour_decodes() {
    let (_, public, message, signature) = fixture();
    let s = *signature.s_bytes();
    // No x satisfies the curve equation for y = 2; one does for y = 3.
    assert_eq!(
        public.verify_strict(message, &Signature::from_components(small(2), s)),
        Err(SignatureError::InvalidR)
    );
    assert_eq!(
        public.verify_strict(message, &Signature::from_components(small(3), s)),
        Err(SignatureError::Mismatch)
    );
}

#[test]
fn an_r_encoding_x_zero_with_the_sign_bit_is_refused_and_without_it_decodes() {
    let (_, public, message, signature) = fixture();
    let s = *signature.s_bytes();
    assert_eq!(
        public.verify_strict(
            message,
            &Signature::from_components(with_top(small(1), 0x80), s)
        ),
        Err(SignatureError::InvalidR)
    );
    // The identity itself decodes, and is refused only for its small order.
    assert_eq!(
        public.verify_strict(message, &Signature::from_components(small(1), s)),
        Err(SignatureError::SmallOrder)
    );
}

#[test]
fn keys_that_fail_rfc_8032_decoding_are_refused_beside_ones_that_pass() {
    // y >= p.
    let mut p_plus_4 = P;
    p_plus_4[0] += 4;
    assert_eq!(
        VerifyingKey::from_bytes(&p_plus_4),
        Err(SignatureError::InvalidPublicKey)
    );
    assert!(VerifyingKey::from_bytes(&small(4)).is_ok());
    // p - 1 (y = -1) is canonical.
    let mut p_minus_1 = P;
    p_minus_1[0] -= 1;
    assert!(VerifyingKey::from_bytes(&p_minus_1).is_ok());
    assert_eq!(
        VerifyingKey::from_bytes(&P),
        Err(SignatureError::InvalidPublicKey)
    );
    // Off the curve.
    assert_eq!(
        VerifyingKey::from_bytes(&small(2)),
        Err(SignatureError::InvalidPublicKey)
    );
    assert!(VerifyingKey::from_bytes(&small(3)).is_ok());
    // x = 0 with the sign bit.
    assert_eq!(
        VerifyingKey::from_bytes(&with_top(small(1), 0x80)),
        Err(SignatureError::InvalidPublicKey)
    );
    assert!(VerifyingKey::from_bytes(&small(1)).is_ok());
    assert_eq!(
        VerifyingKey::from_bytes(&with_top(p_minus_1, 0x80)),
        Err(SignatureError::InvalidPublicKey)
    );
}

#[test]
fn a_signature_under_one_key_is_refused_by_another() {
    let (_, public, message, signature) = fixture();
    let other = SigningKey::from_bytes(&[0xa5; 32]).verifying_key();
    assert_eq!(
        other.verify_strict(message, &signature),
        Err(SignatureError::Mismatch)
    );
    assert_eq!(public.verify_strict(message, &signature), Ok(()));
}

#[test]
fn every_single_bit_flip_of_a_valid_signature_is_refused() {
    let (_, public, message, signature) = fixture();
    let raw = signature.to_bytes();
    for bit in 0..SIGNATURE_LENGTH * 8 {
        let mut flipped = raw;
        flipped[bit / 8] ^= 1 << (bit % 8);
        assert!(
            public
                .verify_strict(message, &Signature::from_bytes(&flipped))
                .is_err(),
            "bit {bit}"
        );
    }
    assert_eq!(
        public.verify_strict(message, &Signature::from_bytes(&raw)),
        Ok(())
    );
}

#[test]
fn truncated_and_extended_messages_are_refused() {
    let (_, public, message, signature) = fixture();
    assert!(
        public
            .verify_strict(&message[..message.len() - 1], &signature)
            .is_err()
    );
    let mut longer = message.to_vec();
    longer.push(0);
    assert!(public.verify_strict(&longer, &signature).is_err());
    assert_eq!(public.verify_strict(message, &signature), Ok(()));
}

#[test]
fn empty_and_large_messages_sign_and_verify() {
    let key = SigningKey::from_bytes(&[0; 32]);
    let public = key.verifying_key();
    for message in [Vec::new(), vec![0xab; 1 << 16]] {
        let signature = key.sign(&message);
        assert_eq!(public.verify_strict(&message, &signature), Ok(()));
        // Signing is deterministic.
        assert_eq!(key.sign(&message), signature);
    }
}

#[test]
fn slices_of_the_wrong_length_are_refused_beside_the_right_length() {
    let (_, public, _, signature) = fixture();
    let raw_sig = signature.to_bytes();
    for len in [0, SIGNATURE_LENGTH - 1, SIGNATURE_LENGTH + 1] {
        let bytes: Vec<u8> = raw_sig.iter().copied().cycle().take(len).collect();
        assert_eq!(
            Signature::try_from(bytes.as_slice()),
            Err(SignatureError::InvalidLength {
                expected: SIGNATURE_LENGTH,
                actual: len
            })
        );
    }
    assert_eq!(Signature::try_from(&raw_sig[..]), Ok(signature));

    let raw_key = public.to_bytes();
    let mut long = raw_key.to_vec();
    long.push(0);
    assert_eq!(
        VerifyingKey::try_from(long.as_slice()),
        Err(SignatureError::InvalidLength {
            expected: PUBLIC_KEY_LENGTH,
            actual: 33
        })
    );
    assert_eq!(
        VerifyingKey::try_from(&raw_key[..PUBLIC_KEY_LENGTH - 1]),
        Err(SignatureError::InvalidLength {
            expected: PUBLIC_KEY_LENGTH,
            actual: 31
        })
    );
    assert_eq!(VerifyingKey::try_from(&raw_key[..]), Ok(public));
}

#[test]
fn verify_is_the_strict_rule() {
    let (_, public, message, signature) = fixture();
    let mut s_plus_l = [0u8; 32];
    let mut carry = 0u16;
    for (i, slot) in s_plus_l.iter_mut().enumerate() {
        let t = u16::from(signature.s_bytes()[i]) + u16::from(L[i]) + carry;
        *slot = t as u8;
        carry = t >> 8;
    }
    let malleated = Signature::from_components(*signature.r_bytes(), s_plus_l);
    assert_eq!(
        public.verify(message, &malleated),
        Err(SignatureError::NonCanonicalS)
    );
    assert_eq!(public.verify(message, &signature), Ok(()));
}

#[test]
fn debug_output_never_shows_the_secret() {
    let seed = [0x3c; 32];
    let key = SigningKey::from_bytes(&seed);
    let rendered = format!("{key:?}");
    assert!(!rendered.contains(&purrdf_hash::hex::encode(&seed)));
    assert!(rendered.contains(&purrdf_hash::hex::encode(&key.verifying_key().to_bytes())));
    // Clones sign identically and outlive the original.
    let clone = key.clone();
    drop(key);
    assert_eq!(clone.to_bytes(), seed);
    assert_eq!(
        clone.verifying_key().verify_strict(b"m", &clone.sign(b"m")),
        Ok(())
    );
}
