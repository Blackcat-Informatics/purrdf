// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! ML-DSA-65 + Ed25519, pinned to LAMPS composite signatures draft-19 and
//! JOSE/COSE composite signatures draft-04. COSE application context is empty.
//! The requested COSE algorithm `-58` is provisional, not an IANA assignment.
//!
//! Both components authenticate the same domain-separated SHA-512 message
//! representative. The ML-DSA component additionally uses the pairing label
//! as its FIPS 204 context. Encodings concatenate ML-DSA then Ed25519: public
//! keys are 1952 + 32 bytes, private seeds 32 + 32, signatures 3309 + 64.
//!
//! Generate two fresh, independent component seeds for each composite key and
//! dedicate both keys to this algorithm. Import cannot detect external reuse.
//! Caller seed/export/randomizer storage belongs to the caller. Owned secrets
//! use the primitive homes' clearing discipline, including owned SHAKE buffers,
//! absorber/reader lanes and sampling scratch. Historical compiler copies,
//! registers, spills and external SHA-512 states are not guaranteed to be
//! cleared. No certification or complete constant-time claim is made; see
//! [`crate::mldsa65`]'s rejection-sampling and source-level timing contract.
//!
//! ```
//! use purrdf_gts::cose::{self, composite, SigningKeyRef, VerifyingKeyRef, SigStatus};
//! // Explicit fixed fixture seeds and deterministic signing for this example.
//! let key = composite::SigningKey::from_bytes(&[7; 64])?;
//! let signed = cose::sign_id_deterministic(b"frame id", SigningKeyRef::Composite(&key), "kid")?;
//! let parsed = cose::parse_sign1(&signed)?;
//! assert_eq!(parsed.verify(b"frame id", VerifyingKeyRef::Composite(key.verifying_key())), SigStatus::Valid);
//! assert_eq!(parsed.verify(b"changed", VerifyingKeyRef::Composite(key.verifying_key())), SigStatus::Invalid);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use purrdf_ed25519 as ed25519;
use purrdf_hash::Domain;
use sha2::{Digest as _, Sha512};

use crate::mldsa65;

/// Exact composite public-key encoding size.
pub const PUBLIC_KEY_LENGTH: usize = mldsa65::PUBLIC_KEY_LENGTH + ed25519::PUBLIC_KEY_LENGTH;
/// Exact interoperable private encoding: two independent 32-byte seeds.
pub const SECRET_KEY_LENGTH: usize = mldsa65::SEED_LENGTH + ed25519::SECRET_KEY_LENGTH;
/// Exact composite signature encoding size.
pub const SIGNATURE_LENGTH: usize = mldsa65::SIGNATURE_LENGTH + ed25519::SIGNATURE_LENGTH;

// These are standard-mandated domain spellings, frozen by the pinned drafts.
const PREFIX: Domain = Domain::new(b"CompositeAlgorithmSignatures2025");
const LABEL: Domain = Domain::new(b"COMPSIG-MLDSA65-Ed25519-SHA512");
/// Empty application context adds one length byte before the message prehash.
pub const REPRESENTATIVE_LENGTH: usize = PREFIX.len() + LABEL.len() + 1 + 64;

/// Input validation or component cryptographic failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// Public encoding has a wrong length, malformed Ed25519 point or weak key.
    InvalidPublicKey,
    /// Private encoding must contain exactly two 32-byte seeds.
    InvalidSecretKey,
    /// Signature encoding or either authentication component is invalid.
    InvalidSignature,
    /// The native ML-DSA operation exhausted a documented budget.
    MlDsa(mldsa65::Error),
}

impl core::fmt::Display for Error {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidPublicKey => formatter.write_str("invalid composite public key"),
            Self::InvalidSecretKey => {
                formatter.write_str("composite private key requires 64 seed bytes")
            }
            Self::InvalidSignature => formatter.write_str("invalid composite signature"),
            Self::MlDsa(error) => write!(formatter, "composite ML-DSA operation: {error}"),
        }
    }
}
impl std::error::Error for Error {}

/// Form `Prefix || Label || 0x00 || SHA512(message)`, exactly as the pinned
/// drafts require. For COSE, `message` is the complete encoded Sig_structure.
#[must_use]
pub fn representative(message: &[u8]) -> [u8; REPRESENTATIVE_LENGTH] {
    let mut output = [0; REPRESENTATIVE_LENGTH];
    output[..PREFIX.len()].copy_from_slice(PREFIX.as_bytes());
    output[PREFIX.len()..PREFIX.len() + LABEL.len()].copy_from_slice(LABEL.as_bytes());
    output[PREFIX.len() + LABEL.len() + 1..].copy_from_slice(&Sha512::digest(message));
    output
}

/// Caller-supplied composite public key, with no discovery or trust policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifyingKey {
    mldsa: mldsa65::VerifyingKey,
    ed25519: ed25519::VerifyingKey,
}

impl VerifyingKey {
    /// Decode exact ML-DSA || Ed25519 bytes; refuse malformed or weak Ed keys.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() != PUBLIC_KEY_LENGTH {
            return Err(Error::InvalidPublicKey);
        }
        let mldsa = mldsa65::VerifyingKey::from_bytes(&bytes[..mldsa65::PUBLIC_KEY_LENGTH])
            .map_err(|_| Error::InvalidPublicKey)?;
        let ed25519 = ed25519::VerifyingKey::try_from(&bytes[mldsa65::PUBLIC_KEY_LENGTH..])
            .map_err(|_| Error::InvalidPublicKey)?;
        if ed25519.is_weak() {
            return Err(Error::InvalidPublicKey);
        }
        Ok(Self { mldsa, ed25519 })
    }

    /// Export the canonical concatenation.
    #[must_use]
    pub fn to_bytes(&self) -> [u8; PUBLIC_KEY_LENGTH] {
        let mut bytes = [0; PUBLIC_KEY_LENGTH];
        bytes[..mldsa65::PUBLIC_KEY_LENGTH].copy_from_slice(self.mldsa.as_bytes());
        bytes[mldsa65::PUBLIC_KEY_LENGTH..].copy_from_slice(self.ed25519.as_bytes());
        bytes
    }

    /// Require BOTH pure ML-DSA and strict RFC 8032 Ed25519 authentication.
    pub fn verify(&self, message: &[u8], signature: &Signature) -> Result<(), Error> {
        let message = representative(message);
        // Evaluate both components; acceptance is their conjunction.
        let post_quantum = self
            .mldsa
            .verify(&message, LABEL.as_bytes(), &signature.mldsa);
        let classical = self.ed25519.verify_strict(&message, &signature.ed25519);
        if post_quantum.is_ok() & classical.is_ok() {
            Ok(())
        } else {
            Err(Error::InvalidSignature)
        }
    }
}

/// Owned dedicated component keys and private seeds. Debug is public-only;
/// cloning explicitly duplicates owned secrets with the same drop discipline.
#[derive(Clone)]
pub struct SigningKey {
    seeds: Seeds,
    mldsa: mldsa65::SigningKey,
    ed25519: ed25519::SigningKey,
    public: VerifyingKey,
}
purrdf_hash::debug_non_exhaustive!(SigningKey { public });

#[derive(Clone)]
struct Seeds([u8; SECRET_KEY_LENGTH]);
impl Drop for Seeds {
    fn drop(&mut self) {
        ed25519::wipe_secret(&mut self.0);
    }
}

impl SigningKey {
    /// Import the interoperable ML-DSA seed || Ed25519 seed encoding.
    /// Seeds must be independently generated and dedicated to this composite.
    /// Equal bytes are legal encodings (including the IETF fixture); external
    /// randomness quality or prior use cannot be inferred from seed bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        let seeds = Seeds(bytes.try_into().map_err(|_| Error::InvalidSecretKey)?);
        let mldsa_seed = seeds.0[..32].try_into().expect("exact seed prefix");
        let ed_seed = seeds.0[32..].try_into().expect("exact seed suffix");
        let mldsa = mldsa65::SigningKey::from_seed(mldsa_seed).map_err(Error::MlDsa)?;
        let ed25519 = ed25519::SigningKey::from_bytes(ed_seed);
        let public = VerifyingKey {
            mldsa: mldsa.verifying_key().clone(),
            ed25519: ed25519.verifying_key(),
        };
        Ok(Self {
            seeds,
            mldsa,
            ed25519,
            public,
        })
    }

    /// Export to caller-owned secret storage; the caller must clear it.
    pub fn export_seeds(&self, output: &mut [u8; SECRET_KEY_LENGTH]) {
        output.copy_from_slice(&self.seeds.0);
    }

    /// Borrow the derived composite public key.
    #[must_use]
    pub const fn verifying_key(&self) -> &VerifyingKey {
        &self.public
    }

    /// Explicit deterministic fixture variant, using the FIPS zero randomizer.
    pub fn sign_deterministic(&self, message: &[u8]) -> Result<Signature, Error> {
        self.sign_hedged(message, &[0; mldsa65::SEED_LENGTH])
    }

    /// Sign with a fresh caller-provided 32-byte cryptographic randomizer.
    /// No host entropy lookup or deterministic fallback occurs.
    pub fn sign_hedged(&self, message: &[u8], randomizer: &[u8; 32]) -> Result<Signature, Error> {
        let message = representative(message);
        let mldsa = self
            .mldsa
            .sign_hedged(&message, LABEL.as_bytes(), randomizer)
            .map_err(Error::MlDsa)?;
        let ed25519 = self.ed25519.sign(&message);
        Ok(Signature { mldsa, ed25519 })
    }
}

/// Exact ML-DSA || Ed25519 encoding. Decoding alone is not authentication.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signature {
    mldsa: mldsa65::Signature,
    ed25519: ed25519::Signature,
}

impl Signature {
    /// Decode exact length and canonical ML-DSA norms/hints. Ed25519's strict
    /// R/S and equation checks run in [`VerifyingKey::verify`], in their home.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() != SIGNATURE_LENGTH {
            return Err(Error::InvalidSignature);
        }
        let mldsa = mldsa65::Signature::from_bytes(&bytes[..mldsa65::SIGNATURE_LENGTH])
            .map_err(|_| Error::InvalidSignature)?;
        let ed25519 = ed25519::Signature::try_from(&bytes[mldsa65::SIGNATURE_LENGTH..])
            .map_err(|_| Error::InvalidSignature)?;
        Ok(Self { mldsa, ed25519 })
    }

    /// Export the two fixed-length components in their mandated order.
    #[must_use]
    pub fn to_bytes(&self) -> [u8; SIGNATURE_LENGTH] {
        let mut bytes = [0; SIGNATURE_LENGTH];
        bytes[..mldsa65::SIGNATURE_LENGTH].copy_from_slice(self.mldsa.as_bytes());
        bytes[mldsa65::SIGNATURE_LENGTH..].copy_from_slice(&self.ed25519.to_bytes());
        bytes
    }
}
