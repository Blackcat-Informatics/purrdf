// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! One strict detached GTS Sign1 parser and algorithm dispatch.

use purrdf_ed25519::{Signature, SigningKey, VerifyingKey};
use purrdf_lex::cbor::{self, Integer, Value};

use super::{
    composite,
    headers::{HeaderLabel, critical_labels, header_labels, header_value},
};
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
    /// Malformed, algorithm/key mismatch or failed verification.
    Invalid,
    /// Not checked: the key was not resolved, or the declared signing
    /// algorithm is one this engine does not implement.
    Unverified,
}

/// Structural/algorithm failure or a failed composite signing operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sign1Error {
    /// Invalid envelope, duplicate/ambiguous headers or unsupported critical data.
    Malformed,
    /// The protected header names an algorithm this engine does not implement,
    /// by an integer or by a text string (RFC 9052 section 3.1).
    UnsupportedAlgorithm,
    /// The protected header bytes are not deterministically encoded (RFC 8949
    /// section 4.2): a non-shortest integer or length, an indefinite-length
    /// item or map keys out of bytewise order. Specification section 4 requires
    /// every signed byte string to use that encoding, so no conforming writer
    /// produces such a header; a valid signature over one is not tampering, but
    /// it is not a signature the format admits.
    NonCanonicalProtectedHeader,
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
            Self::NonCanonicalProtectedHeader => formatter.write_str(
                "protected header is not deterministically encoded (RFC 8949 section 4.2)",
            ),
            Self::Composite(error) => write!(formatter, "COSE composite signing: {error}"),
        }
    }
}
purrdf_lex::error_source!(Sign1Error { Composite });

impl purrdf_hash::error::StructuredError for Sign1Error {
    purrdf_hash::error_visit! {
        Malformed => "gts-sign1-malformed" {},
        UnsupportedAlgorithm => "gts-sign1-unsupported-algorithm" {},
        NonCanonicalProtectedHeader => "gts-sign1-noncanonical-protected-header" {},
        Composite(error) => "gts-sign1-composite-failure" { "cause" => Cause(error) },
    }
}

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

/// Validate both header buckets. `Ok(None)` is a well-formed header declaring an
/// algorithm this engine does not implement.
fn validate_headers(
    protected: &[(Value, Value)],
    unprotected: &[(Value, Value)],
) -> Result<Option<Algorithm>, Sign1Error> {
    let protected_labels = header_labels(protected).ok_or(Sign1Error::Malformed)?;
    let unprotected_labels = header_labels(unprotected).ok_or(Sign1Error::Malformed)?;
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
    for label in critical_labels(protected, &protected_labels).ok_or(Sign1Error::Malformed)? {
        // Only alg and kid are interpreted by this detached GTS contract.
        if !matches!(label, HeaderLabel::Integer(n) if n.get() == i128::from(ALG) || n.get() == i128::from(KID))
        {
            return Err(Sign1Error::Malformed);
        }
    }
    // RFC 9052 section 3.1 types `alg` as an integer or a text string. A text
    // name is never one of the integer identifiers this engine implements.
    match header_value(protected, ALG).ok_or(Sign1Error::Malformed)? {
        Value::Text(_) => Ok(None),
        value => match Algorithm::from_id(value.as_integer().ok_or(Sign1Error::Malformed)?) {
            Ok(algorithm) => Ok(Some(algorithm)),
            Err(Sign1Error::UnsupportedAlgorithm) => Ok(None),
            Err(error) => Err(error),
        },
    }
}

/// Specification section 4: the protected header is a hashed and signed byte
/// string, so it is deterministically encoded, and its integers fit the signed
/// 64-bit range the header grammar admits (-2^63 to 2^63-1).
fn decode_protected_header(protected: &[u8]) -> Result<Value, Sign1Error> {
    let decoded = cbor::decode_deterministic(protected, cbor::Limits::DEFAULT).map_err(
        |error| match error.kind() {
            cbor::DecodeErrorKind::NotDeterministic(_) => Sign1Error::NonCanonicalProtectedHeader,
            _ => Sign1Error::Malformed,
        },
    )?;
    let fits = |value: &Value| {
        value
            .as_integer()
            .is_none_or(|integer| i64::try_from(integer.get()).is_ok())
    };
    let map = decoded.as_map().ok_or(Sign1Error::Malformed)?;
    if map.iter().all(|(key, value)| fits(key) && fits(value)) {
        Ok(decoded)
    } else {
        Err(Sign1Error::Malformed)
    }
}

/// The structure of a detached Sign1 whose algorithm may be one this engine
/// does not implement: everything except the algorithm-specific signature check.
struct Envelope {
    /// `None` when the header declares an algorithm this engine does not implement.
    algorithm: Option<Algorithm>,
    kid: Option<Vec<u8>>,
    protected: Vec<u8>,
    signature: Vec<u8>,
}

impl Envelope {
    /// The supported envelope, or [`Sign1Error::UnsupportedAlgorithm`], after the
    /// exact signature-encoding check of the declared algorithm.
    fn into_sign1(self) -> Result<Sign1, Sign1Error> {
        let algorithm = self.algorithm.ok_or(Sign1Error::UnsupportedAlgorithm)?;
        if self.signature.len() != algorithm.signature_length() {
            return Err(Sign1Error::Malformed);
        }
        if algorithm == Algorithm::CompositeMlDsa65Ed25519 {
            composite::Signature::from_bytes(&self.signature).map_err(|_| Sign1Error::Malformed)?;
        }
        Ok(Sign1 {
            algorithm,
            kid: self.kid,
            protected: self.protected,
            signature: self.signature,
        })
    }
}

fn parse_envelope(bytes: &[u8]) -> Result<Envelope, Sign1Error> {
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
    let decoded = decode_protected_header(&protected)?;
    let protected_map = decoded.as_map().ok_or(Sign1Error::Malformed)?;
    let unprotected = unprotected.into_map().map_err(|_| Sign1Error::Malformed)?;
    let algorithm = validate_headers(protected_map, &unprotected)?;
    // RFC 9052 §3.1 makes kid optional. Header validation already establishes
    // the bstr type when present; no absent-to-empty or text encoding fallback.
    let kid = header_value(protected_map, KID)
        .or_else(|| header_value(&unprotected, KID))
        .map(|value| value.as_bytes().ok_or(Sign1Error::Malformed))
        .transpose()?
        .map(<[u8]>::to_vec);
    let signature = signature.into_bytes().map_err(|_| Sign1Error::Malformed)?;
    Ok(Envelope {
        algorithm,
        kid,
        protected,
        signature,
    })
}

/// Parse exactly one complete tagged-18 or untagged detached Sign1.
/// Require four fields, a deterministically encoded protected header with a
/// supported integer alg, unique integer/text header labels, no cross-bucket
/// ambiguity, understood protected critical labels, detached null payload,
/// optional opaque byte-string kid and exact signature encoding. Absence of a
/// kid does not prevent verification with a supplied key. Nested/wrong tags,
/// COSE_Sign, attached payload and trailing input fail. A protected header
/// that is not deterministically encoded is
/// [`Sign1Error::NonCanonicalProtectedHeader`]; a declared algorithm this
/// engine does not implement, named by an integer or a text string, is
/// [`Sign1Error::UnsupportedAlgorithm`].
pub fn parse_sign1(bytes: &[u8]) -> Result<Sign1, Sign1Error> {
    parse_envelope(bytes)?.into_sign1()
}

/// Why a retained COSE_Sign1 is not a signature the format admits, or `None`
/// when its structure is sound (whether it verifies is a separate question).
/// A store that finds a signature `invalid` reads this to tell a header that is
/// not deterministically encoded
/// ([`Sign1Error::NonCanonicalProtectedHeader`]) from any other malformed
/// envelope and from a signature that simply fails to verify. An envelope that
/// declares an unsupported algorithm is structurally sound.
#[must_use]
pub fn signature_refusal(cose: &[u8]) -> Option<Sign1Error> {
    match parse_envelope(cose).and_then(Envelope::into_sign1) {
        Ok(_) | Err(Sign1Error::UnsupportedAlgorithm) => None,
        Err(error) => Some(error),
    }
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
    match parse_sign1(sig) {
        Ok(parsed) => parsed.verify(frame_id, public),
        Err(Sign1Error::UnsupportedAlgorithm) => SigStatus::Unverified,
        Err(_) => SigStatus::Invalid,
    }
}

/// Existing Ed25519 convenience, routed through the same strict typed core.
#[must_use]
pub fn verify_sig(sig: &[u8], frame_id: &[u8], public: &VerifyingKey) -> SigStatus {
    verify_sig_with_key(sig, frame_id, VerifyingKeyRef::Ed25519(public))
}

/// Verify recorded signatures with the existing Ed25519 resolver. Parse and
/// algorithm validation precede lookup: a malformed envelope is invalid; an
/// unsupported algorithm, or a supported valid structure with no resolved
/// key, is unverified.
pub fn verify_signatures(
    signatures: &mut [model::Signature],
    resolve: impl Fn(&str) -> Option<VerifyingKey>,
) {
    verify_resolved(signatures, |parsed, frame_id| {
        parsed
            .kid_text()
            .and_then(&resolve)
            .map(|key| parsed.verify(frame_id, VerifyingKeyRef::Ed25519(&key)))
    });
}

/// Verify folded signatures using explicit opaque/absent identifiers and typed
/// keys. The declared algorithm is supplied to the caller's resolver;
/// returning a key of another algorithm produces invalid, never a downgrade.
/// Malformed or unsupported envelopes never invoke the resolver: a malformed
/// one is invalid and one with an unsupported algorithm is unverified.
pub fn verify_signatures_with_resolver<'a>(
    signatures: &mut [model::Signature],
    resolve: impl Fn(Option<&[u8]>, Algorithm) -> Option<VerifyingKeyRef<'a>>,
) {
    verify_resolved(signatures, |parsed, frame_id| {
        resolve(parsed.kid(), parsed.algorithm()).map(|key| parsed.verify(frame_id, key))
    });
}

fn verify_resolved(
    signatures: &mut [model::Signature],
    verify: impl Fn(&Sign1, &[u8]) -> Option<SigStatus>,
) {
    for sig in signatures {
        let Some(cose) = &sig.cose else {
            continue;
        };
        let envelope = parse_envelope(cose);
        // An envelope that declares an algorithm this engine does not implement
        // is structurally sound, so its kid is reported as for any other
        // (RFC 9052 section 3.1: `kid` is independent of `alg`). A malformed
        // envelope reports none.
        let unsupported_kid = envelope
            .as_ref()
            .ok()
            .filter(|envelope| envelope.algorithm.is_none())
            .and_then(|envelope| envelope.kid.as_deref())
            .and_then(|kid| core::str::from_utf8(kid).ok())
            .map(str::to_string);
        let parsed = envelope.and_then(Envelope::into_sign1);
        sig.kid = parsed
            .as_ref()
            .ok()
            .and_then(|parsed| parsed.kid_text().map(str::to_string))
            .or(unsupported_kid);
        sig.status = match parsed {
            Err(Sign1Error::UnsupportedAlgorithm) => "unverified",
            Err(_) => "invalid",
            Ok(parsed) => match verify(&parsed, &sig.frame_id) {
                Some(SigStatus::Valid) => "valid",
                Some(SigStatus::Invalid) => "invalid",
                Some(SigStatus::Unverified) | None => "unverified",
            },
        }
        .to_string();
    }
}
