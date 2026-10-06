// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! One strict detached GTS Sign1 parser and algorithm dispatch.

use std::collections::BTreeSet;

use purrdf_ed25519::{Signature, SigningKey, VerifyingKey};
use purrdf_lex::cbor::{self, Integer, Value};

use super::composite;
use crate::{model, wire};

const ALG: i64 = 1;
const CRIT: i64 = 2;
const KID: i64 = 4;
const TAG_SIGN1: u64 = 18;

/// Supported authenticated signing algorithms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Algorithm {
    /// EdDSA with the existing Ed25519 GTS contract, COSE -8.
    Ed25519,
    /// ML-DSA-65/Ed25519 from JOSE/COSE draft-04, requested -58.
    /// The value and pinned construction are provisional, not registered.
    CompositeMlDsa65Ed25519,
}

impl Algorithm {
    /// Declared COSE integer; -58 is the pinned draft's provisional request.
    #[must_use]
    pub const fn cose_id(self) -> i64 {
        match self {
            Self::Ed25519 => -8,
            Self::CompositeMlDsa65Ed25519 => -58,
        }
    }

    fn from_id(value: Integer) -> Result<Self, Sign1Error> {
        match value.get() {
            -8 => Ok(Self::Ed25519),
            -58 => Ok(Self::CompositeMlDsa65Ed25519),
            _ => Err(Sign1Error::UnsupportedAlgorithm),
        }
    }

    const fn signature_length(self) -> usize {
        match self {
            Self::Ed25519 => 64,
            Self::CompositeMlDsa65Ed25519 => composite::SIGNATURE_LENGTH,
        }
    }
}

/// Borrow a dedicated key for the selected signing algorithm.
#[derive(Debug, Clone, Copy)]
pub enum SigningKeyRef<'a> {
    /// Existing Ed25519 key.
    Ed25519(&'a SigningKey),
    /// Dedicated composite key.
    Composite(&'a composite::SigningKey),
}
impl SigningKeyRef<'_> {
    /// The authenticated algorithm associated with this key type.
    #[must_use]
    pub const fn algorithm(self) -> Algorithm {
        match self {
            Self::Ed25519(_) => Algorithm::Ed25519,
            Self::Composite(_) => Algorithm::CompositeMlDsa65Ed25519,
        }
    }
}

/// A public key of an explicit algorithm, without trust or discovery policy.
#[derive(Debug, Clone, Copy)]
pub enum VerifyingKeyRef<'a> {
    /// Existing Ed25519 key.
    Ed25519(&'a VerifyingKey),
    /// Dedicated composite key.
    Composite(&'a composite::VerifyingKey),
}

/// The verification outcome for a detached COSE_Sign1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SigStatus {
    /// Cryptographically valid under the resolved key.
    Valid,
    /// Malformed, unsupported, algorithm/key mismatch or failed verification.
    Invalid,
    /// Well-formed supported envelope whose key was not resolved.
    Unverified,
}

/// Structural/algorithm failure or a failed composite signing operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sign1Error {
    /// Invalid envelope, duplicate/ambiguous headers or unsupported critical data.
    Malformed,
    /// Protected alg is an integer, but not a supported signing algorithm.
    UnsupportedAlgorithm,
    /// The native composite signing operation failed without an output.
    Composite(composite::Error),
}
impl core::fmt::Display for Sign1Error {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Malformed => formatter.write_str("malformed or ambiguous GTS COSE_Sign1"),
            Self::UnsupportedAlgorithm => {
                formatter.write_str("unsupported protected COSE signing algorithm")
            }
            Self::Composite(error) => write!(formatter, "COSE composite signing: {error}"),
        }
    }
}
impl std::error::Error for Sign1Error {}

/// A validated supported detached envelope. Its protected bytes are retained
/// exactly as received, not normalized or re-encoded before authentication.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sign1 {
    algorithm: Algorithm,
    kid: Option<Vec<u8>>,
    protected: Vec<u8>,
    signature: Vec<u8>,
}

impl Sign1 {
    /// Authenticated algorithm declaration (authentication still requires verify).
    #[must_use]
    pub const fn algorithm(&self) -> Algorithm {
        self.algorithm
    }
    /// Exact optional opaque kid bytes (RFC 9052). Absence is distinct from
    /// an explicitly present empty identifier.
    #[must_use]
    pub fn kid(&self) -> Option<&[u8]> {
        self.kid.as_deref()
    }
    /// Text view for existing String-based key resolvers, without an implicit
    /// encoding fallback. Binary identifiers remain usable with supplied keys.
    #[must_use]
    pub fn kid_text(&self) -> Option<&str> {
        self.kid()
            .and_then(|bytes| core::str::from_utf8(bytes).ok())
    }
    /// Exact received protected bytes.
    #[must_use]
    pub fn protected(&self) -> &[u8] {
        &self.protected
    }
    /// Exact received fixed-length signature bytes.
    #[must_use]
    pub fn signature_bytes(&self) -> &[u8] {
        &self.signature
    }

    /// Verify through the algorithm/key-specific primitive. A wrong key type
    /// always fails, even if an Ed25519 equation could authenticate the bytes.
    #[must_use]
    pub fn verify(&self, frame_id: &[u8], public: VerifyingKeyRef<'_>) -> SigStatus {
        let message = sig_structure(&self.protected, frame_id);
        let valid = match (self.algorithm, public) {
            (Algorithm::Ed25519, VerifyingKeyRef::Ed25519(key)) => {
                Signature::try_from(self.signature.as_slice())
                    .and_then(|signature| key.verify_strict(&message, &signature))
                    .is_ok()
            }
            (Algorithm::CompositeMlDsa65Ed25519, VerifyingKeyRef::Composite(key)) => {
                composite::Signature::from_bytes(&self.signature)
                    .and_then(|signature| key.verify(&message, &signature))
                    .is_ok()
            }
            _ => false,
        };
        if valid {
            SigStatus::Valid
        } else {
            SigStatus::Invalid
        }
    }
}

fn protected_header(algorithm: Algorithm) -> Vec<u8> {
    wire::encode(&Value::Map(vec![(ALG.into(), algorithm.cose_id().into())]))
}

/// RFC 9052 §4.4 Sig_structure with empty external AAD. Payload is the detached
/// frame id in GTS; this byte construction also replays primary COSE examples.
#[must_use]
pub fn sig_structure(protected: &[u8], payload: &[u8]) -> Vec<u8> {
    wire::encode(&Value::Array(vec![
        Value::Text("Signature1".to_string()),
        Value::Bytes(protected.to_vec()),
        Value::Bytes(Vec::new()),
        Value::Bytes(payload.to_vec()),
    ]))
}

fn envelope(protected: Vec<u8>, kid: &[u8], signature: Vec<u8>) -> Vec<u8> {
    wire::encode(&Value::Tag(
        TAG_SIGN1,
        Box::new(Value::Array(vec![
            Value::Bytes(protected),
            Value::Map(vec![(KID.into(), Value::Bytes(kid.to_vec()))]),
            Value::Null,
            Value::Bytes(signature),
        ])),
    ))
}

/// Sign one detached Sign1 with caller-supplied hedging randomness. Composite
/// callers must provide fresh cryptographic bytes per signature; Ed25519 is
/// intrinsically deterministic and does not consume the randomizer.
pub fn sign_id_hedged(
    frame_id: &[u8],
    signing_key: SigningKeyRef<'_>,
    kid: impl AsRef<[u8]>,
    randomizer: &[u8; 32],
) -> Result<Vec<u8>, Sign1Error> {
    let protected = protected_header(signing_key.algorithm());
    let message = sig_structure(&protected, frame_id);
    let signature = match signing_key {
        SigningKeyRef::Ed25519(key) => key.sign(&message).to_bytes().to_vec(),
        SigningKeyRef::Composite(key) => key
            .sign_hedged(&message, randomizer)
            .map_err(Sign1Error::Composite)?
            .to_bytes()
            .to_vec(),
    };
    Ok(envelope(protected, kid.as_ref(), signature))
}

/// Explicit deterministic fixture entry point; no production entropy fallback.
pub fn sign_id_deterministic(
    frame_id: &[u8],
    signing_key: SigningKeyRef<'_>,
    kid: impl AsRef<[u8]>,
) -> Result<Vec<u8>, Sign1Error> {
    sign_id_hedged(frame_id, signing_key, kid, &[0; 32])
}

/// Existing deterministic Ed25519 convenience, byte-exact to frozen GTS vectors.
#[must_use]
pub fn sign_id(frame_id: &[u8], signing_key: &SigningKey, kid: &str) -> Vec<u8> {
    sign_id_deterministic(frame_id, SigningKeyRef::Ed25519(signing_key), kid)
        .expect("Ed25519 signing is infallible and requires no entropy")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum HeaderLabel<'a> {
    Integer(Integer),
    Text(&'a str),
}
impl<'a> HeaderLabel<'a> {
    fn from_value(value: &'a Value) -> Result<Self, Sign1Error> {
        match value {
            Value::Integer(integer) => Ok(Self::Integer(*integer)),
            Value::Text(text) => Ok(Self::Text(text)),
            _ => Err(Sign1Error::Malformed),
        }
    }
}

fn header_labels(headers: &[(Value, Value)]) -> Result<BTreeSet<HeaderLabel<'_>>, Sign1Error> {
    let mut labels = BTreeSet::new();
    for (label, value) in headers {
        if !labels.insert(HeaderLabel::from_value(label)?) {
            return Err(Sign1Error::Malformed);
        }
        // RFC 9052 common-header value types, even for ignored metadata.
        match label.as_integer().map(Integer::get) {
            Some(1) if value.as_integer().is_none() => return Err(Sign1Error::Malformed),
            Some(2) if value.as_array().is_none_or(Vec::is_empty) => {
                return Err(Sign1Error::Malformed);
            }
            Some(3) if !matches!(value, Value::Text(_) | Value::Integer(_)) => {
                return Err(Sign1Error::Malformed);
            }
            Some(3) if value.as_integer().is_some_and(|n| n.get() < 0) => {
                return Err(Sign1Error::Malformed);
            }
            Some(4..=6) if value.as_bytes().is_none() => return Err(Sign1Error::Malformed),
            // This GTS signing contract implements no counter-signature layer.
            Some(7) => return Err(Sign1Error::Malformed),
            _ => {}
        }
    }
    Ok(labels)
}

fn header_value(headers: &[(Value, Value)], label: i64) -> Option<&Value> {
    headers
        .iter()
        .find(|(key, _)| key.as_integer() == Some(label.into()))
        .map(|(_, value)| value)
}

fn validate_headers(
    protected: &[(Value, Value)],
    unprotected: &[(Value, Value)],
) -> Result<Algorithm, Sign1Error> {
    let protected_labels = header_labels(protected)?;
    let unprotected_labels = header_labels(unprotected)?;
    // RFC 9052 §3 recommends rejecting cross-bucket ambiguity. GTS does so.
    if !protected_labels.is_disjoint(&unprotected_labels)
        || header_value(unprotected, ALG).is_some()
        || header_value(unprotected, CRIT).is_some()
    {
        return Err(Sign1Error::Malformed);
    }
    let iv = HeaderLabel::Integer(5.into());
    let partial_iv = HeaderLabel::Integer(6.into());
    let present = |label| protected_labels.contains(&label) || unprotected_labels.contains(&label);
    if present(iv) && present(partial_iv) {
        return Err(Sign1Error::Malformed);
    }
    if let Some(critical) = header_value(protected, CRIT) {
        let mut labels = BTreeSet::new();
        for value in critical.as_array().ok_or(Sign1Error::Malformed)? {
            let label = HeaderLabel::from_value(value)?;
            // Only alg and kid are interpreted by this detached GTS contract.
            // Unknown noncritical metadata can be retained; it cannot become
            // a critical instruction the verifier silently ignores.
            if !labels.insert(label)
                || !protected_labels.contains(&label)
                || !matches!(label, HeaderLabel::Integer(n) if n.get() == i128::from(ALG) || n.get() == i128::from(KID))
            {
                return Err(Sign1Error::Malformed);
            }
        }
    }
    let algorithm = header_value(protected, ALG)
        .and_then(Value::as_integer)
        .ok_or(Sign1Error::Malformed)?;
    Algorithm::from_id(algorithm)
}

/// Parse exactly one complete tagged-18 or untagged detached Sign1. Require
/// four fields, protected supported integer alg, unique integer/text header
/// labels, no cross-bucket ambiguity, understood protected critical labels,
/// detached null payload, optional opaque byte-string kid and exact signature
/// encoding. Absence does not prevent verification with a supplied key.
/// Nested/wrong tags, COSE_Sign, attached payload and trailing input fail.
pub fn parse_sign1(bytes: &[u8]) -> Result<Sign1, Sign1Error> {
    let mut value =
        cbor::decode(bytes, cbor::Limits::DEFAULT).map_err(|_| Sign1Error::Malformed)?;
    let body = match &mut value {
        Value::Tag(TAG_SIGN1, inner) => inner.take(),
        Value::Tag(_, _) => return Err(Sign1Error::Malformed),
        other => other.take(),
    };
    let [protected, unprotected, payload, signature]: [Value; 4] = body
        .into_array()
        .map_err(|_| Sign1Error::Malformed)?
        .try_into()
        .map_err(|_| Sign1Error::Malformed)?;
    if !matches!(payload, Value::Null) {
        return Err(Sign1Error::Malformed);
    }
    let protected = protected.into_bytes().map_err(|_| Sign1Error::Malformed)?;
    let decoded =
        cbor::decode(&protected, cbor::Limits::DEFAULT).map_err(|_| Sign1Error::Malformed)?;
    let protected_map = decoded.as_map().ok_or(Sign1Error::Malformed)?;
    let unprotected = unprotected.into_map().map_err(|_| Sign1Error::Malformed)?;
    let algorithm = validate_headers(protected_map, &unprotected)?;
    // RFC 9052 §3.1 makes kid optional. Header validation already establishes
    // the bstr type when present; no absent-to-empty or text encoding fallback.
    let kid = header_value(protected_map, KID)
        .or_else(|| header_value(&unprotected, KID))
        .map(|value| value.as_bytes().ok_or(Sign1Error::Malformed))
        .transpose()?;
    let signature = signature.into_bytes().map_err(|_| Sign1Error::Malformed)?;
    if signature.len() != algorithm.signature_length() {
        return Err(Sign1Error::Malformed);
    }
    if algorithm == Algorithm::CompositeMlDsa65Ed25519 {
        composite::Signature::from_bytes(&signature).map_err(|_| Sign1Error::Malformed)?;
    }
    Ok(Sign1 {
        algorithm,
        kid: kid.map(<[u8]>::to_vec),
        protected,
        signature,
    })
}

/// Existing Ed25519 tuple convenience, using the sole strict parser. Composite
/// consumers use [`parse_sign1`]; this wrapper accepts only Ed25519 envelopes.
#[must_use]
pub fn parse(sig: &[u8]) -> Option<(String, Vec<u8>, [u8; 64])> {
    let parsed = parse_sign1(sig).ok()?;
    if parsed.algorithm != Algorithm::Ed25519 {
        return None;
    }
    Some((
        String::from_utf8(parsed.kid?).ok()?,
        parsed.protected,
        parsed.signature.try_into().ok()?,
    ))
}

/// Kid of a well-formed supported Sign1, regardless of its signing algorithm.
#[must_use]
pub fn signature_kid(sig: &[u8]) -> Option<String> {
    parse_sign1(sig)
        .ok()
        .and_then(|parsed| parsed.kid)
        .and_then(|kid| String::from_utf8(kid).ok())
}

/// Typed algorithm-aware verification, refusing mismatched key types.
#[must_use]
pub fn verify_sig_with_key(sig: &[u8], frame_id: &[u8], public: VerifyingKeyRef<'_>) -> SigStatus {
    parse_sign1(sig).map_or(SigStatus::Invalid, |parsed| parsed.verify(frame_id, public))
}

/// Existing Ed25519 convenience, routed through the same strict typed core.
#[must_use]
pub fn verify_sig(sig: &[u8], frame_id: &[u8], public: &VerifyingKey) -> SigStatus {
    verify_sig_with_key(sig, frame_id, VerifyingKeyRef::Ed25519(public))
}

/// Verify recorded signatures with the existing Ed25519 resolver. Parse and
/// algorithm validation precede lookup: malformed/unsupported is invalid;
/// a supported valid structure with no resolved key is unverified.
pub fn verify_signatures(
    signatures: &mut [model::Signature],
    resolve: impl Fn(&str) -> Option<VerifyingKey>,
) {
    for sig in signatures {
        let Some(cose) = &sig.cose else {
            continue;
        };
        let parsed = parse_sign1(cose);
        sig.kid = parsed
            .as_ref()
            .ok()
            .and_then(|parsed| parsed.kid_text().map(str::to_string));
        sig.status = match parsed {
            Err(_) => "invalid",
            Ok(parsed) => match parsed.kid_text().and_then(&resolve) {
                Some(key)
                    if parsed.verify(&sig.frame_id, VerifyingKeyRef::Ed25519(&key))
                        == SigStatus::Valid =>
                {
                    "valid"
                }
                Some(_) => "invalid",
                None => "unverified",
            },
        }
        .to_string();
    }
}
