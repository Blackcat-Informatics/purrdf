// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! `purrdf-ed25519` — Ed25519 signatures (RFC 8032, pure Ed25519: no context,
//! no pre-hash) for PurRDF.
//!
//! This crate is the workspace's one Ed25519 implementation: key expansion,
//! signing and verification are written once here, over the workspace's one
//! SHA-512 ([`sha2::Sha512`]), and every signer and verifier (the GTS
//! COSE_Sign1 envelope, OpenPGP key import, the RDF packaging signatures)
//! calls into it.
//!
//! ```
//! use purrdf_ed25519::{SigningKey, VerifyingKey};
//!
//! let key = SigningKey::from_bytes(&[7u8; 32]);
//! let signature = key.sign(b"message");
//! let public = VerifyingKey::from_bytes(&key.verifying_key().to_bytes()).unwrap();
//! assert!(public.verify_strict(b"message", &signature).is_ok());
//! assert!(public.verify_strict(b"messagf", &signature).is_err());
//! ```
//!
//! # Verification contract
//!
//! There is one verification rule, [`VerifyingKey::verify_strict`]
//! ([`VerifyingKey::verify`] is the same function). A (key, message,
//! signature) triple is refused exactly when one of these holds:
//!
//! * **the key does not decode** ([`VerifyingKey::from_bytes`],
//!   [`SignatureError::InvalidPublicKey`]) — the decoding of RFC 8032 5.1.3:
//!   the 255-bit y is not below p = 2^255 - 19, or (y² - 1)/(d·y² + 1) has no
//!   square root, or the root is x = 0 while the sign bit is set;
//! * **S is not canonical** ([`SignatureError::NonCanonicalS`]) — the last 32
//!   bytes, as a little-endian integer, are not below the group order L;
//! * **R does not decode** ([`SignatureError::InvalidR`]) — the first 32 bytes
//!   fail the same three decoding tests as a key;
//! * **the key or R has small order** ([`SignatureError::SmallOrder`]) — 8·A
//!   or 8·R is the identity: one of the eight torsion points;
//! * **the cofactorless equation fails** ([`SignatureError::Mismatch`]) —
//!   with k = SHA-512(R ‖ A ‖ M) mod L over the bytes as received, the
//!   canonical encoding of S·B - k·A differs from R's bytes.
//!
//! Points of mixed order (a prime-order point plus a nonzero torsion point)
//! are accepted as keys and as R; the cofactorless equation then decides.
//!
//! This is the acceptance set of `ed25519-dalek`'s `verify_strict` for every
//! key both accept. The two differ only in key decoding: `ed25519-dalek`
//! reduces a y at or above p instead of refusing it, and accepts x = 0 with
//! the sign bit set. Of those encodings, the ones naming a small-order point
//! (reduced y of 0 or 1, and every x = 0 encoding) are refused by both
//! verifiers anyway; the rest (reduced y of 3, 4, 5, 6, 9, 10, 14, 15, 16 or
//! 18, either sign) are keys `ed25519-dalek` accepts and this crate refuses at
//! [`VerifyingKey::from_bytes`], as RFC 8032 requires. R needs no such note:
//! the equation compares R's bytes against a canonical encoding, so a
//! non-canonical R never verifies under either implementation.
//!
//! # Side channels
//!
//! Field arithmetic is radix 2^51 over five `u64` limbs with `u128` products,
//! scalar arithmetic is Montgomery multiplication mod L in radix 2^52, and the
//! fixed-base multiplication that signing performs on the secret scalar and
//! the nonce reads its table by masked selection over every entry. No branch
//! and no memory index depends on a secret. Verification handles only public
//! data and uses variable-time double-scalar multiplication. The seed, the
//! expanded scalar and the nonce prefix are overwritten when a [`SigningKey`]
//! is dropped, and each nonce is overwritten once its signature is formed.
//! The SHA-512 states that absorb the seed and prefix belong to `sha2` and
//! are not wiped by this crate.

#![forbid(unsafe_code)]

mod ct;
mod field;
mod keys;
mod point;
mod scalar;

#[cfg(test)]
mod strictness_tests;

pub use keys::{
    PUBLIC_KEY_LENGTH, SECRET_KEY_LENGTH, SIGNATURE_LENGTH, Signature, SignatureError, SigningKey,
    VerifyingKey,
};
