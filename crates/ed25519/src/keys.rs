// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The Ed25519 key, signature and error types (RFC 8032 section 5.1).

use core::fmt;

use sha2::{Digest as _, Sha512};

use crate::ct;
use crate::point::Point;
use crate::scalar::Scalar;

/// Length in bytes of a secret key (the RFC 8032 seed).
pub const SECRET_KEY_LENGTH: usize = 32;
/// Length in bytes of an encoded public key.
pub const PUBLIC_KEY_LENGTH: usize = 32;
/// Length in bytes of a signature, R ‖ S.
pub const SIGNATURE_LENGTH: usize = 64;

/// Why a key or signature was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SignatureError {
    /// The public key is not an RFC 8032 5.1.3 point encoding: its
    /// y-coordinate is not below p, no x satisfies the curve equation for it,
    /// or it encodes x = 0 with the sign bit set.
    InvalidPublicKey,
    /// The signature's R is not an RFC 8032 5.1.3 point encoding, by the same
    /// three tests as [`Self::InvalidPublicKey`].
    InvalidR,
    /// The signature's S, read as a 256-bit little-endian integer, is not
    /// below the group order L (RFC 8032 5.1.7 step 1).
    NonCanonicalS,
    /// The public key or R lies in the torsion subgroup (8·P is the
    /// identity). Such a key admits signatures valid for many messages, and
    /// such an R makes a signature malleable, so both are refused.
    SmallOrder,
    /// Every component is well formed, but S·B ≠ R + k·A.
    Mismatch,
    /// A byte slice had the wrong length for the value it was read as.
    InvalidLength {
        /// The length the value requires.
        expected: usize,
        /// The length supplied.
        actual: usize,
    },
}

impl fmt::Display for SignatureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPublicKey => {
                f.write_str("Ed25519 public key is not a canonical point encoding")
            }
            Self::InvalidR => f.write_str("Ed25519 signature R is not a canonical point encoding"),
            Self::NonCanonicalS => f.write_str("Ed25519 signature S is not below the group order"),
            Self::SmallOrder => f.write_str("Ed25519 public key or signature R has small order"),
            Self::Mismatch => f.write_str("Ed25519 verification equation was not satisfied"),
            Self::InvalidLength { expected, actual } => {
                write!(f, "Ed25519 value must be {expected} bytes, got {actual}")
            }
        }
    }
}

impl std::error::Error for SignatureError {}

fn exact<const N: usize>(bytes: &[u8]) -> Result<[u8; N], SignatureError> {
    bytes.try_into().map_err(|_| SignatureError::InvalidLength {
        expected: N,
        actual: bytes.len(),
    })
}

/// An Ed25519 signature: the encoded point R followed by the scalar S.
///
/// Constructing one from bytes checks nothing; [`VerifyingKey::verify_strict`]
/// decides whether R and S are acceptable.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Signature {
    r: [u8; 32],
    s: [u8; 32],
}

impl Signature {
    /// The signature whose 64-byte encoding is `bytes`.
    pub fn from_bytes(bytes: &[u8; SIGNATURE_LENGTH]) -> Self {
        let mut r = [0u8; 32];
        let mut s = [0u8; 32];
        r.copy_from_slice(&bytes[..32]);
        s.copy_from_slice(&bytes[32..]);
        Self { r, s }
    }

    /// The signature with the given R and S encodings.
    pub const fn from_components(r: [u8; 32], s: [u8; 32]) -> Self {
        Self { r, s }
    }

    /// The 64-byte encoding R ‖ S.
    pub fn to_bytes(&self) -> [u8; SIGNATURE_LENGTH] {
        let mut out = [0u8; SIGNATURE_LENGTH];
        out[..32].copy_from_slice(&self.r);
        out[32..].copy_from_slice(&self.s);
        out
    }

    /// The encoded point R.
    pub const fn r_bytes(&self) -> &[u8; 32] {
        &self.r
    }

    /// The encoded scalar S.
    pub const fn s_bytes(&self) -> &[u8; 32] {
        &self.s
    }
}

impl TryFrom<&[u8]> for Signature {
    type Error = SignatureError;

    fn try_from(bytes: &[u8]) -> Result<Self, SignatureError> {
        exact::<SIGNATURE_LENGTH>(bytes).map(|b| Self::from_bytes(&b))
    }
}

impl fmt::Debug for Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Signature")
            .field("r", &purrdf_hash::hex::encode(&self.r))
            .field("s", &purrdf_hash::hex::encode(&self.s))
            .finish()
    }
}

/// An Ed25519 public key: a decoded curve point together with the exact
/// encoding it was read from (which is what the challenge hash covers).
#[derive(Clone, Copy)]
pub struct VerifyingKey {
    bytes: [u8; PUBLIC_KEY_LENGTH],
    point: Point,
}

impl VerifyingKey {
    /// Decode a public key, refusing every encoding RFC 8032 5.1.3 rejects:
    /// y not below p, a y with no matching x, and x = 0 with the sign bit set.
    /// Small-order keys decode (they are points); [`Self::verify_strict`]
    /// refuses them, and [`Self::is_weak`] reports them.
    pub fn from_bytes(bytes: &[u8; PUBLIC_KEY_LENGTH]) -> Result<Self, SignatureError> {
        let point = Point::decode(bytes).ok_or(SignatureError::InvalidPublicKey)?;
        Ok(Self {
            bytes: *bytes,
            point,
        })
    }

    /// The 32-byte encoding.
    pub const fn to_bytes(&self) -> [u8; PUBLIC_KEY_LENGTH] {
        self.bytes
    }

    /// The 32-byte encoding, borrowed.
    pub const fn as_bytes(&self) -> &[u8; PUBLIC_KEY_LENGTH] {
        &self.bytes
    }

    /// Whether the key lies in the torsion subgroup, where a signature can
    /// verify for many messages. [`Self::verify_strict`] refuses such keys.
    pub fn is_weak(&self) -> bool {
        self.point.is_small_order()
    }

    /// Verify `signature` over `message` (pure Ed25519, no context, no
    /// pre-hash), strictly:
    ///
    /// 1. S must be below L, else [`SignatureError::NonCanonicalS`];
    /// 2. R must be an RFC 8032 5.1.3 point encoding, else
    ///    [`SignatureError::InvalidR`];
    /// 3. neither this key nor R may have small order, else
    ///    [`SignatureError::SmallOrder`];
    /// 4. with k = SHA-512(R ‖ A ‖ message) mod L over the bytes as given,
    ///    the cofactorless equation S·B = R + k·A must hold, compared as the
    ///    canonical encoding of S·B - k·A against R's bytes, else
    ///    [`SignatureError::Mismatch`].
    pub fn verify_strict(
        &self,
        message: &[u8],
        signature: &Signature,
    ) -> Result<(), SignatureError> {
        let s = Scalar::from_canonical_bytes(&signature.s).ok_or(SignatureError::NonCanonicalS)?;
        let r = Point::decode(&signature.r).ok_or(SignatureError::InvalidR)?;
        if r.is_small_order() || self.point.is_small_order() {
            return Err(SignatureError::SmallOrder);
        }
        let k = challenge(&signature.r, &self.bytes, message);
        let expected = Point::double_scalar_mul_vartime(&k, &self.point.neg(), &s);
        if expected.encode() == signature.r {
            Ok(())
        } else {
            Err(SignatureError::Mismatch)
        }
    }

    /// Verify `signature` over `message`. This is [`Self::verify_strict`]:
    /// there is one verification rule, and it is the strict one.
    pub fn verify(&self, message: &[u8], signature: &Signature) -> Result<(), SignatureError> {
        self.verify_strict(message, signature)
    }
}

impl TryFrom<&[u8]> for VerifyingKey {
    type Error = SignatureError;

    fn try_from(bytes: &[u8]) -> Result<Self, SignatureError> {
        Self::from_bytes(&exact::<PUBLIC_KEY_LENGTH>(bytes)?)
    }
}

impl PartialEq for VerifyingKey {
    fn eq(&self, other: &Self) -> bool {
        self.bytes == other.bytes
    }
}

impl Eq for VerifyingKey {}

impl core::hash::Hash for VerifyingKey {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.bytes.hash(state);
    }
}

impl fmt::Debug for VerifyingKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("VerifyingKey")
            .field(&purrdf_hash::hex::encode(&self.bytes))
            .finish()
    }
}

impl From<&SigningKey> for VerifyingKey {
    fn from(key: &SigningKey) -> Self {
        key.verifying
    }
}

/// An Ed25519 secret key: the 32-byte seed and what RFC 8032 5.1.5 expands
/// it into (the clamped secret scalar s and the nonce prefix), plus the
/// public key A = s·B.
///
/// The seed, scalar and prefix are overwritten when the key is dropped, and
/// [`fmt::Debug`] prints only the public key.
#[derive(Clone)]
pub struct SigningKey {
    seed: [u8; SECRET_KEY_LENGTH],
    scalar: Scalar,
    prefix: [u8; 32],
    verifying: VerifyingKey,
}

impl SigningKey {
    /// Expand a 32-byte seed (RFC 8032 5.1.5): h = SHA-512(seed); s is the
    /// low half with bits 0–2 and 255 cleared and bit 254 set; the high half
    /// is the nonce prefix; A = s·B.
    pub fn from_bytes(seed: &[u8; SECRET_KEY_LENGTH]) -> Self {
        let mut h: [u8; 64] = Sha512::digest(seed).into();
        let mut clamped = [0u8; 32];
        clamped.copy_from_slice(&h[..32]);
        clamped[0] &= 0xf8;
        clamped[31] &= 0x7f;
        clamped[31] |= 0x40;
        let mut prefix = [0u8; 32];
        prefix.copy_from_slice(&h[32..]);
        // B has order L, so [s]B = [s mod L]B.
        let scalar = Scalar::from_bytes_mod_order(&clamped);
        let bytes = Point::mul_base(&scalar).encode();
        ct::wipe(&mut h);
        ct::wipe(&mut clamped);
        let point = Point::decode(&bytes).expect("a multiple of B encodes canonically");
        Self {
            seed: *seed,
            scalar,
            prefix,
            verifying: VerifyingKey { bytes, point },
        }
    }

    /// The 32-byte seed.
    pub const fn to_bytes(&self) -> [u8; SECRET_KEY_LENGTH] {
        self.seed
    }

    /// The 32-byte seed, borrowed.
    pub const fn as_bytes(&self) -> &[u8; SECRET_KEY_LENGTH] {
        &self.seed
    }

    /// The expanded secret scalar, for tests that build signatures by hand.
    #[cfg(test)]
    pub(crate) const fn secret_scalar(&self) -> Scalar {
        self.scalar
    }

    /// The public key.
    pub const fn verifying_key(&self) -> VerifyingKey {
        self.verifying
    }

    /// Sign `message` (RFC 8032 5.1.6): r = SHA-512(prefix ‖ message) mod L,
    /// R = r·B, k = SHA-512(R ‖ A ‖ message) mod L, S = r + k·s mod L.
    /// Deterministic; the secret scalar and the nonce only ever pass through
    /// constant-time code, and the nonce is overwritten before returning.
    pub fn sign(&self, message: &[u8]) -> Signature {
        let mut hasher = Sha512::new();
        hasher.update(self.prefix);
        hasher.update(message);
        let mut nonce_wide: [u8; 64] = hasher.finalize().into();
        let mut nonce = Scalar::from_bytes_wide(&nonce_wide);
        ct::wipe(&mut nonce_wide);
        let r = Point::mul_base(&nonce).encode();
        let k = challenge(&r, &self.verifying.bytes, message);
        let s = k.mul_add(self.scalar, nonce);
        nonce.wipe();
        Signature { r, s: s.to_bytes() }
    }
}

impl Drop for SigningKey {
    fn drop(&mut self) {
        ct::wipe(&mut self.seed);
        ct::wipe(&mut self.prefix);
        self.scalar.wipe();
    }
}

impl fmt::Debug for SigningKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SigningKey")
            .field("verifying_key", &self.verifying)
            .finish_non_exhaustive()
    }
}

/// k = SHA-512(R ‖ A ‖ message) mod L (RFC 8032 5.1.6 step 4, 5.1.7 step 2).
fn challenge(r: &[u8; 32], a: &[u8; 32], message: &[u8]) -> Scalar {
    let mut hasher = Sha512::new();
    hasher.update(r);
    hasher.update(a);
    hasher.update(message);
    let wide: [u8; 64] = hasher.finalize().into();
    Scalar::from_bytes_wide(&wide)
}
