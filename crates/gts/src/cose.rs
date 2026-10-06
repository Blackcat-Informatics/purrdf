// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
//! Algorithm-aware detached COSE_Sign1 over a frame id — GTS-SPEC §9.2.
//! Ed25519 and pinned draft ML-DSA-65/Ed25519 share one strict envelope path.
//!
//! The Ed25519 contract is byte-compatible with the Python reference: the detached payload is the frame
//! `id`, the protected header is `{1: -8}` (EdDSA), and the unprotected header
//! carries the `kid` (label 4). Ed25519 is deterministic (RFC 8032), so the same
//! key + id always yields the same signature — gated by `vectors/cose/*.json`.

use aes_gcm::aead::{Aead, AeadInOut, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Nonce, Tag};
#[cfg(test)]
use purrdf_ed25519::{SigningKey, VerifyingKey};
use purrdf_lex::cbor::{self, Integer, Value, head};
use std::borrow::Cow;

#[cfg(test)]
use crate::model;
use crate::wire;

mod borrowed;
/// Pinned draft composite primitive and canonical key/signature encodings.
pub mod composite;
mod sign1;
#[cfg(test)]
mod tests;

const ALG: i64 = 1;
const KID: i64 = 4;
const IV: i64 = 5;
const ALG_A256GCM: i64 = 3;
const TAG_ENCRYPT0: u64 = 16;

pub use sign1::{
    Algorithm, SigStatus, Sign1, Sign1Error, SigningKeyRef, VerifyingKeyRef, parse, parse_sign1,
    sig_structure, sign_id, sign_id_deterministic, sign_id_hedged, signature_kid, verify_sig,
    verify_sig_with_key, verify_signatures,
};

// -- COSE_Encrypt0 (AES-256-GCM, keyed by kid) — GTS-SPEC §9.3 -----------------

/// Why a `decrypt0` could not return plaintext.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encrypt0Error {
    /// The COSE_Encrypt0 structure could not be parsed.
    Malformed,
    /// No content key was resolved for the recipient `kid`.
    MissingKey,
    /// AES-GCM authentication failed (wrong key or tampered ciphertext).
    AuthFailed,
}

impl std::fmt::Display for Encrypt0Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match self {
            Self::Malformed => "malformed COSE_Encrypt0",
            Self::MissingKey => "no content key for recipient",
            Self::AuthFailed => "authentication failed (AES-GCM tag mismatch)",
        };
        f.write_str(msg)
    }
}

impl std::error::Error for Encrypt0Error {}

/// The protected header for A256GCM: `{1: 3}` (canonical CBOR).
fn encrypt0_protected() -> Vec<u8> {
    wire::encode(&Value::Map(vec![(
        Value::Integer(Integer::from(ALG)),
        Value::Integer(Integer::from(ALG_A256GCM)),
    )]))
}

/// The COSE `Enc_structure` bound as AAD (RFC 9052 §5.3): no external AAD.
fn enc_structure(protected: &[u8]) -> Vec<u8> {
    // One array byte, nine context bytes, at most nine byte-string header
    // bytes, and one empty byte string. No owned CBOR tree or growth copies.
    let mut aad = Vec::with_capacity(protected.len().saturating_add(20));
    head::push_head(&mut aad, head::ARRAY, 3);
    head::write_text(&mut aad, "Encrypt0").expect("writing to a Vec cannot fail");
    head::write_bytes(&mut aad, protected).expect("writing to a Vec cannot fail");
    head::write_bytes(&mut aad, &[]).expect("writing to a Vec cannot fail");
    aad
}

/// Seal `plaintext` as a COSE_Encrypt0 with an explicit 12-byte `iv` (§9.3).
///
/// Splitting the IV out keeps the transform deterministic so it can be frozen
/// in `vectors/encrypt0/*.json` and keeps the core crate wasm-portable. Callers
/// that need randomized authoring generate the IV outside this crate and pass it
/// in here.
pub fn encrypt0_with_iv(plaintext: &[u8], kid: &str, key: &[u8; 32], iv: &[u8; 12]) -> Vec<u8> {
    let protected = encrypt0_protected();
    let aad = enc_structure(&protected);
    let cipher = Aes256Gcm::new(key.into());
    let ciphertext = cipher
        .encrypt(
            &Nonce::from(*iv),
            Payload {
                msg: plaintext,
                aad: &aad,
            },
        )
        .expect("AES-256-GCM encryption cannot fail for a valid key/nonce");
    // Unprotected header keys in canonical order: kid (4) before iv (5).
    let cose = Value::Tag(
        TAG_ENCRYPT0,
        Box::new(Value::Array(vec![
            Value::Bytes(protected),
            Value::Map(vec![
                (
                    Value::Integer(Integer::from(KID)),
                    Value::Bytes(kid.as_bytes().to_vec()),
                ),
                (Value::Integer(Integer::from(IV)), Value::Bytes(iv.to_vec())),
            ]),
            Value::Bytes(ciphertext),
        ])),
    );
    wire::canonical(&cose)
}

/// Seal `plaintext` as a COSE_Encrypt0 to the recipient `kid` (§9.3).
///
/// This is the portable production entry point. The caller owns nonce policy and
/// must provide a fresh 12-byte IV for randomized encryption.
pub fn encrypt0(plaintext: &[u8], kid: &str, key: &[u8; 32], iv: &[u8; 12]) -> Vec<u8> {
    encrypt0_with_iv(plaintext, kid, key, iv)
}

/// The cleartext fields of a parsed COSE_Encrypt0.
struct Encrypt0Parts<'a> {
    kid: Cow<'a, str>,
    protected: Cow<'a, [u8]>,
    iv: Cow<'a, [u8]>,
    ciphertext: Cow<'a, [u8]>,
}

/// Parse a COSE_Encrypt0 into its cleartext fields, or `None` if malformed.
fn parse_encrypt0(blob: &[u8]) -> Option<Encrypt0Parts<'_>> {
    borrowed::parse(blob).or_else(|| parse_encrypt0_owned(blob))
}

/// Preserve the complete decoder's accepted forms outside the borrowed view.
fn parse_encrypt0_owned(blob: &[u8]) -> Option<Encrypt0Parts<'_>> {
    let (mut value, _) = cbor::decode_prefix(blob, cbor::Limits::DEFAULT).ok()?;
    let body = match &mut value {
        Value::Tag(_, inner) => inner.take(),
        other => other.take(),
    };
    let [protected, unprotected, ciphertext]: [Value; 3] =
        body.into_array().ok()?.try_into().ok()?;
    let protected = protected.into_bytes().ok()?;
    let unprotected = unprotected.into_map().ok()?;
    let ciphertext = ciphertext.into_bytes().ok()?;
    let kid_target = Integer::from(KID);
    let iv_target = Integer::from(IV);
    let kid = unprotected.iter().find_map(|(k, v)| match (k, v) {
        (Value::Integer(i), Value::Bytes(b)) if *i == kid_target => {
            String::from_utf8(b.clone()).ok()
        }
        _ => None,
    })?;
    let iv = unprotected.iter().find_map(|(k, v)| match (k, v) {
        (Value::Integer(i), Value::Bytes(b)) if *i == iv_target => Some(b.clone()),
        _ => None,
    })?;
    Some(Encrypt0Parts {
        kid: kid.into(),
        protected: protected.into(),
        iv: iv.into(),
        ciphertext: ciphertext.into(),
    })
}

/// The recipient `kid` of a COSE_Encrypt0 (for key lookup), or `None`.
pub fn recipient_kid(blob: &[u8]) -> Option<String> {
    parse_encrypt0(blob).map(|p| p.kid.into_owned())
}

/// Open a COSE_Encrypt0 using a content key resolved by `kid` (§9.3).
///
/// Standalone COSE consumers use the same bounded implementation as the GTS
/// reader. Plaintext cannot exceed its encoded envelope's length, so this
/// entry point does not need a second AES-GCM implementation or an unbounded
/// decoded-output allowance.
///
/// # Errors
/// Returns an error for a malformed envelope, an unresolved key, or failed
/// authentication. No unauthenticated plaintext is returned.
pub fn decrypt0(
    blob: &[u8],
    resolve: impl Fn(&str) -> Option<[u8; 32]>,
) -> Result<Vec<u8>, Encrypt0Error> {
    decrypt0_bounded(blob, resolve, blob.len()).map_err(|error| match error {
        BoundedDecrypt0Error::Crypto(error) => error,
        BoundedDecrypt0Error::Limit => Encrypt0Error::Malformed,
    })
}

/// Internal bounded decryption failures, without extending the public error enum.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum BoundedDecrypt0Error {
    Crypto(Encrypt0Error),
    Limit,
}

/// Decrypt after checking the exact plaintext size.
///
/// The usual definite envelope borrows its fields from encoded input and copies
/// only the plaintext-sized ciphertext prefix. Other CBOR forms reuse the full
/// decoder's buffer. The AES-GCM tag is excluded from the decoded limit.
pub(crate) fn decrypt0_bounded(
    blob: &[u8],
    resolve: impl Fn(&str) -> Option<[u8; 32]>,
    limit: usize,
) -> Result<Vec<u8>, BoundedDecrypt0Error> {
    let parts =
        parse_encrypt0(blob).ok_or(BoundedDecrypt0Error::Crypto(Encrypt0Error::Malformed))?;
    decrypt_parts(parts, resolve, limit)
}

fn decrypt_parts(
    parts: Encrypt0Parts<'_>,
    resolve: impl Fn(&str) -> Option<[u8; 32]>,
    limit: usize,
) -> Result<Vec<u8>, BoundedDecrypt0Error> {
    use BoundedDecrypt0Error::{Crypto, Limit};

    let key = resolve(&parts.kid).ok_or(Crypto(Encrypt0Error::MissingKey))?;
    if parts.iv.len() != 12 {
        return Err(Crypto(Encrypt0Error::Malformed));
    }
    let plaintext_len = parts
        .ciphertext
        .len()
        .checked_sub(16)
        .ok_or(Crypto(Encrypt0Error::AuthFailed))?;
    if plaintext_len > limit {
        return Err(Limit);
    }
    let tag = Tag::try_from(&parts.ciphertext[plaintext_len..])
        .map_err(|_| Crypto(Encrypt0Error::AuthFailed))?;
    let nonce = Nonce::try_from(&parts.iv[..]).map_err(|_| Crypto(Encrypt0Error::Malformed))?;
    let mut plaintext = match parts.ciphertext {
        Cow::Borrowed(bytes) => bytes[..plaintext_len].to_vec(),
        Cow::Owned(mut bytes) => {
            bytes.truncate(plaintext_len);
            bytes
        }
    };
    let aad = enc_structure(&parts.protected);
    let cipher = Aes256Gcm::new((&key).into());
    cipher
        .decrypt_inout_detached(&nonce, &aad, plaintext.as_mut_slice().into(), &tag)
        .map_err(|_| Crypto(Encrypt0Error::AuthFailed))?;
    Ok(plaintext)
}
