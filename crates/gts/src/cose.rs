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
mod headers;
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
    verify_sig_with_key, verify_signatures, verify_signatures_with_resolver,
};

// -- COSE_Encrypt0 (AES-256-GCM, keyed by kid) — GTS-SPEC §9.3 -----------------

/// Why protected-header authoring or authenticated opening could not complete.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encrypt0Error {
    /// The envelope or caller-supplied protected header is invalid.
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

impl purrdf_hash::error::StructuredError for Encrypt0Error {
    purrdf_hash::error_visit! {
        Malformed => "gts-encrypt0-malformed" {},
        MissingKey => "gts-encrypt0-missing-key" {},
        AuthFailed => "gts-encrypt0-auth-failed" {},
    }
}

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
    // Preserve the legacy infallible API's existing oversized-input boundary.
    seal_with_protected(plaintext, kid, key, iv, protected)
        .expect("legacy COSE_Encrypt0 encryption requires inputs within AES-GCM limits")
}

/// Seal with extra authenticated protected-header entries.
///
/// Labels are CBOR integers or text. Integer label 1 is reserved for A256GCM;
/// labels 4, 5 and 6 would conflict with the fixed unprotected kid/full IV.
/// Label 7 is unsupported: these envelopes implement no countersignature layer.
/// Text labels are distinct from integers, including text `"1"`.
/// The map and its nested values use the shared canonical CBOR ordering.
/// Nonce freshness remains the caller's responsibility.
///
/// Application metadata and critical-header meaning are caller-owned. Critical
/// declarations must be nonempty lists of unique labels present in this map;
/// callers must enforce their meaning before using the opened plaintext.
///
/// # Errors
/// Returns [`Encrypt0Error::Malformed`] for reserved, duplicate or invalid
/// labels, invalid registered-header values or critical declarations, or a
/// header exceeding the shared CBOR decoder's default nesting bound, or inputs
/// exceeding the AES-GCM plaintext or associated-data length bounds.
pub fn encrypt0_with_protected(
    plaintext: &[u8],
    kid: &str,
    key: &[u8; 32],
    iv: &[u8; 12],
    extra: &[(Value, Value)],
) -> Result<Vec<u8>, Encrypt0Error> {
    // Validate borrowed input before cloning caller-owned values.
    headers::header_labels(extra).ok_or(Encrypt0Error::Malformed)?;
    if extra
        .iter()
        .any(|(label, _)| matches!(label.as_integer().map(Integer::get), Some(1 | 4..=6)))
    {
        return Err(Encrypt0Error::Malformed);
    }
    let mut entries = vec![(Value::from(ALG), Value::from(ALG_A256GCM))];
    entries.extend_from_slice(extra);
    let protected = wire::canonical(&Value::Map(entries));
    // Use the same accepted depth and map judgment as the matching opener.
    decode_protected(&protected)?;
    seal_with_protected(plaintext, kid, key, iv, protected)
}

fn seal_with_protected(
    plaintext: &[u8],
    kid: &str,
    key: &[u8; 32],
    iv: &[u8; 12],
    protected: Vec<u8>,
) -> Result<Vec<u8>, Encrypt0Error> {
    if u64::try_from(plaintext.len()).map_err(|_| Encrypt0Error::Malformed)? > aes_gcm::P_MAX {
        return Err(Encrypt0Error::Malformed);
    }
    let aad = enc_structure(&protected);
    if u64::try_from(aad.len()).map_err(|_| Encrypt0Error::Malformed)? > aes_gcm::A_MAX {
        return Err(Encrypt0Error::Malformed);
    }
    let cipher = Aes256Gcm::new(key.into());
    let ciphertext = cipher
        .encrypt(
            &Nonce::from(*iv),
            Payload {
                msg: plaintext,
                aad: &aad,
            },
        )
        .map_err(|_| Encrypt0Error::Malformed)?;
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
    Ok(wire::canonical(&cose))
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

/// A complete authenticated plaintext and protected-header map.
///
/// The map includes the algorithm and all protected entries. It excludes
/// unauthenticated unprotected headers. Authentication does not establish
/// application meaning: the caller must enforce its metadata and critical
/// header policy before using the plaintext.
#[derive(Debug, Clone, PartialEq)]
pub struct OpenedEncrypt0 {
    /// Authenticated plaintext.
    pub plaintext: Vec<u8>,
    /// Complete authenticated map, in its received entry order.
    pub protected: Vec<(Value, Value)>,
}

/// Open an A256GCM envelope and return its complete authenticated protected map.
///
/// Original protected bytes, including noncanonical valid CBOR spellings, are
/// authenticated verbatim. Metadata is decoded locally and published only after
/// successful authentication. Unknown application and critical-header meaning
/// remains the caller's responsibility; this API validates declaration shape
/// and references, not application-specific instructions.
///
/// # Errors
/// Returns [`Encrypt0Error::Malformed`] for invalid protected maps, labels,
/// algorithm or critical declarations, or the same key/authentication errors as
/// [`decrypt0`]. No plaintext or protected entries are returned on failure.
pub fn decrypt0_with_protected(
    blob: &[u8],
    resolve: impl Fn(&str) -> Option<[u8; 32]>,
) -> Result<OpenedEncrypt0, Encrypt0Error> {
    let parts = parse_encrypt0(blob).ok_or(Encrypt0Error::Malformed)?;
    let protected = decode_protected(&parts.protected)?;
    let plaintext = decrypt_parts(parts, resolve, blob.len()).map_err(|error| match error {
        BoundedDecrypt0Error::Crypto(error) => error,
        BoundedDecrypt0Error::Limit => Encrypt0Error::Malformed,
    })?;
    Ok(OpenedEncrypt0 {
        plaintext,
        protected,
    })
}

fn decode_protected(bytes: &[u8]) -> Result<Vec<(Value, Value)>, Encrypt0Error> {
    let entries = cbor::decode(bytes, cbor::Limits::DEFAULT)
        .map_err(|_| Encrypt0Error::Malformed)?
        .into_map()
        .map_err(|_| Encrypt0Error::Malformed)?;
    let labels = headers::header_labels(&entries).ok_or(Encrypt0Error::Malformed)?;
    headers::critical_labels(&entries, &labels).ok_or(Encrypt0Error::Malformed)?;
    if headers::header_value(&entries, ALG).and_then(Value::as_integer) != Some(ALG_A256GCM.into())
        || entries
            .iter()
            .any(|(label, _)| matches!(label.as_integer().map(Integer::get), Some(4..=6)))
    {
        return Err(Encrypt0Error::Malformed);
    }
    Ok(entries)
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
