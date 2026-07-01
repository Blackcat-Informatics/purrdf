// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0
//! COSE_Sign1 (detached payload, EdDSA/Ed25519) over a frame id — GTS-SPEC §9.2.
//!
//! Byte-compatible with the Python reference: the detached payload is the frame
//! `id`, the protected header is `{1: -8}` (EdDSA), and the unprotected header
//! carries the `kid` (label 4). Ed25519 is deterministic (RFC 8032), so the same
//! key + id always yields the same signature — gated by `vectors/cose/*.json`.

use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Nonce};
use ciborium::value::{Integer, Value};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};

use crate::model;
use crate::wire;

const ALG: i64 = 1;
const KID: i64 = 4;
const IV: i64 = 5;
const ALG_EDDSA: i64 = -8;
const ALG_A256GCM: i64 = 3;
const TAG_SIGN1: u64 = 18;
const TAG_ENCRYPT0: u64 = 16;

/// The verification outcome for a detached COSE_Sign1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SigStatus {
    /// Cryptographically valid under the resolved key.
    Valid,
    /// Present but malformed or failed verification.
    Invalid,
    /// Well-formed, but no key was resolved to check it.
    Unverified,
}

fn protected_header() -> Vec<u8> {
    wire::encode(&Value::Map(vec![(
        Value::Integer(Integer::from(ALG)),
        Value::Integer(Integer::from(ALG_EDDSA)),
    )]))
}

/// The COSE `Sig_structure` to be signed/verified (RFC 9052 §4.4).
fn sig_structure(protected: &[u8], frame_id: &[u8]) -> Vec<u8> {
    wire::encode(&Value::Array(vec![
        Value::Text("Signature1".to_string()),
        Value::Bytes(protected.to_vec()),
        Value::Bytes(Vec::new()),
        Value::Bytes(frame_id.to_vec()),
    ]))
}

/// Produce a detached COSE_Sign1 over `frame_id` with the given Ed25519 key.
pub fn sign_id(frame_id: &[u8], signing_key: &SigningKey, kid: &str) -> Vec<u8> {
    let protected = protected_header();
    let signature: Signature = signing_key.sign(&sig_structure(&protected, frame_id));
    let cose = Value::Tag(
        TAG_SIGN1,
        Box::new(Value::Array(vec![
            Value::Bytes(protected),
            Value::Map(vec![(
                Value::Integer(Integer::from(KID)),
                Value::Bytes(kid.as_bytes().to_vec()),
            )]),
            Value::Null,
            Value::Bytes(signature.to_bytes().to_vec()),
        ])),
    );
    wire::encode(&cose)
}

/// Parse a COSE_Sign1 into `(kid, protected, signature)`, or `None` if malformed.
pub fn parse(sig: &[u8]) -> Option<(String, Vec<u8>, [u8; 64])> {
    let value: Value = ciborium::de::from_reader(sig).ok()?;
    let body = match value {
        Value::Tag(_, inner) => *inner,
        other => other,
    };
    let array = body.as_array()?;
    if array.len() != 4 {
        return None;
    }
    let protected = array[0].as_bytes()?.clone();
    let unprotected = array[1].as_map()?;
    let signature: [u8; 64] = array[3].as_bytes()?.as_slice().try_into().ok()?;
    let kid_target = Integer::from(KID);
    let kid = unprotected.iter().find_map(|(k, v)| match (k, v) {
        (Value::Integer(i), Value::Bytes(b)) if *i == kid_target => {
            String::from_utf8(b.clone()).ok()
        }
        _ => None,
    })?;
    Some((kid, protected, signature))
}

/// The `kid` of a COSE_Sign1 (for key lookup), or `None` if malformed.
pub fn signature_kid(sig: &[u8]) -> Option<String> {
    parse(sig).map(|(kid, _, _)| kid)
}

/// Verify a detached COSE_Sign1 over `frame_id` against `public`.
pub fn verify_sig(sig: &[u8], frame_id: &[u8], public: &VerifyingKey) -> SigStatus {
    let Some((_kid, protected, signature)) = parse(sig) else {
        return SigStatus::Invalid;
    };
    let signature = Signature::from_bytes(&signature);
    match public.verify(&sig_structure(&protected, frame_id), &signature) {
        Ok(()) => SigStatus::Valid,
        Err(_) => SigStatus::Invalid,
    }
}

/// Verify the COSE signatures recorded in a folded graph against keys resolved
/// by `kid`. Updates each signature's `kid` and `status` in place: `"valid"` /
/// `"invalid"` when a key resolves, `"unverified"` when none does (§9.2).
pub fn verify_signatures(
    signatures: &mut [model::Signature],
    resolve: impl Fn(&str) -> Option<VerifyingKey>,
) {
    for sig in signatures.iter_mut() {
        let Some(cose) = sig.cose.clone() else {
            continue;
        };
        let kid = signature_kid(&cose);
        sig.kid.clone_from(&kid);
        sig.status = match kid.as_deref().and_then(&resolve) {
            Some(key) => match verify_sig(&cose, &sig.frame_id, &key) {
                SigStatus::Valid => "valid",
                _ => "invalid",
            },
            None => "unverified",
        }
        .to_string();
    }
}

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
    wire::encode(&Value::Array(vec![
        Value::Text("Encrypt0".to_string()),
        Value::Bytes(protected.to_vec()),
        Value::Bytes(Vec::new()),
    ]))
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
            Nonce::from_slice(iv),
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
struct Encrypt0Parts {
    kid: String,
    protected: Vec<u8>,
    iv: Vec<u8>,
    ciphertext: Vec<u8>,
}

/// Parse a COSE_Encrypt0 into its cleartext fields, or `None` if malformed.
fn parse_encrypt0(blob: &[u8]) -> Option<Encrypt0Parts> {
    let value: Value = ciborium::de::from_reader(blob).ok()?;
    let body = match value {
        Value::Tag(_, inner) => *inner,
        other => other,
    };
    let array = body.as_array()?;
    if array.len() != 3 {
        return None;
    }
    let protected = array[0].as_bytes()?.clone();
    let unprotected = array[1].as_map()?;
    let ciphertext = array[2].as_bytes()?.clone();
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
        kid,
        protected,
        iv,
        ciphertext,
    })
}

/// The recipient `kid` of a COSE_Encrypt0 (for key lookup), or `None`.
pub fn recipient_kid(blob: &[u8]) -> Option<String> {
    parse_encrypt0(blob).map(|p| p.kid)
}

/// Open a COSE_Encrypt0 using a content key resolved by `kid` (§9.3).
pub fn decrypt0(
    blob: &[u8],
    resolve: impl Fn(&str) -> Option<[u8; 32]>,
) -> Result<Vec<u8>, Encrypt0Error> {
    let parts = parse_encrypt0(blob).ok_or(Encrypt0Error::Malformed)?;
    let key = resolve(&parts.kid).ok_or(Encrypt0Error::MissingKey)?;
    if parts.iv.len() != 12 {
        return Err(Encrypt0Error::Malformed);
    }
    let aad = enc_structure(&parts.protected);
    let cipher = Aes256Gcm::new((&key).into());
    cipher
        .decrypt(
            Nonce::from_slice(&parts.iv),
            Payload {
                msg: &parts.ciphertext,
                aad: &aad,
            },
        )
        .map_err(|_| Encrypt0Error::AuthFailed)
}
