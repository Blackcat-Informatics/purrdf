// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original native ML-DSA-65, the pure-message algorithm in FIPS 204.
//!
//! Keys expand a caller-owned 32-byte seed. [`SigningKey::sign_hedged`] takes
//! fresh caller-supplied cryptographic randomness for each signature; this
//! portable core does not obtain entropy itself. The explicitly named
//! [`SigningKey::sign_deterministic`] uses the FIPS deterministic zero randomizer.
//! Contexts have at most 255 bytes. HashML-DSA is a different algorithm and is
//! not implemented by these pure-message APIs.
//!
//! ```
//! use purrdf_gts::mldsa65::{SigningKey, VerifyingKey};
//! let key = SigningKey::from_seed(&[7; 32])?;
//! let signature = key.sign_deterministic(b"message", b"application")?;
//! let public = VerifyingKey::from_bytes(key.verifying_key().as_bytes())?;
//! public.verify(b"message", b"application", &signature)?;
//! assert!(public.verify(b"changed", b"application", &signature).is_err());
//! # Ok::<(), purrdf_gts::mldsa65::Error>(())
//! ```
//!
//! # Validation and side channels
//!
//! Importing an expanded private key validates coefficient encodings, derived
//! public-key hash and low bits. All fixed-length public-key bit strings are
//! legal FIPS encodings. Signature decoding requires canonical sparse hints and
//! a response strictly below the verification norm bound. Verification also
//! checks the challenge hash; successful decoding alone is not authentication.
//!
//! NTT arithmetic, rounding, bit packing and coefficient norm scans use fixed
//! schedules without secret-dependent indices or early exits. FIPS rejection
//! sampling (including signing retries) is variable-time; no whole-operation
//! constant-time claim is made. The challenge sampler indexes derived challenge
//! positions, as FIPS Algorithm 29 requires; a successful challenge is public in
//! the signature. Sampling and signing have explicit finite budgets and return
//! an error without a signature if exhausted. These are source-level design
//! properties, not a measurement or certification of compiler, JIT or hardware
//! timing; target-dependent lowering of fixed-divisor arithmetic still matters.
//!
//! Owned expanded secrets and controlled polynomial/byte scratch are overwritten
//! on drop through [`purrdf_ed25519::wipe_secret`]. Exported secret bytes belong
//! to the caller and require the caller's clearing discipline. SHAKE absorber
//! and reader states, compiler-created copies, registers and stack spills are
//! not guaranteed to be cleared. Safe Rust overwriting is not a guarantee that
//! every historical copy of a secret is eliminated. See the fixture provenance
//! for independent standard answers and the original implementation boundary.

mod codec;
mod math;
mod sampling;

use core::fmt;
use math::{BETA, GAMMA1, GAMMA2, K, L, N, Poly};
use purrdf_ed25519::{constant_time_eq, wipe_secret};

/// Size of the key-generation seed and per-signature randomizer.
pub const SEED_LENGTH: usize = 32;
/// FIPS 204 encoded public-key length for ML-DSA-65.
pub const PUBLIC_KEY_LENGTH: usize = 1952;
/// FIPS 204 expanded private-key length for ML-DSA-65.
pub const SECRET_KEY_LENGTH: usize = 4032;
/// FIPS 204 encoded signature length for ML-DSA-65.
pub const SIGNATURE_LENGTH: usize = 3309;

/// A malformed input or exhausted cryptographic operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// A public key has the wrong encoded length.
    InvalidPublicKey,
    /// A secret key is malformed or inconsistent with its derived public key.
    InvalidSecretKey,
    /// A signature has the wrong length, norm, hint encoding or challenge hash.
    InvalidSignature,
    /// Pure ML-DSA contexts have at most 255 bytes.
    ContextTooLong,
    /// A rejection sampler exceeded its candidate budget.
    SamplingExhausted,
    /// Signing exhausted the distinct 16-bit polynomial nonce namespace.
    NonceExhausted,
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidPublicKey => "invalid ML-DSA-65 public-key length",
            Self::InvalidSecretKey => "malformed or inconsistent ML-DSA-65 secret key",
            Self::InvalidSignature => "invalid ML-DSA-65 signature",
            Self::ContextTooLong => "ML-DSA context exceeds 255 bytes",
            Self::SamplingExhausted => "ML-DSA rejection sampler exhausted its candidate budget",
            Self::NonceExhausted => "ML-DSA signing exhausted its nonce namespace",
        })
    }
}
impl std::error::Error for Error {}

struct SecretBytes(Vec<u8>);
impl SecretBytes {
    fn zeros(length: usize) -> Self {
        Self(vec![0; length])
    }
}
impl Drop for SecretBytes {
    fn drop(&mut self) {
        wipe_secret(&mut self.0);
    }
}
struct SecretPolys(Vec<Poly>);
impl SecretPolys {
    fn zeros(length: usize) -> Self {
        Self(vec![[0; N]; length])
    }
    fn transform(&mut self) {
        for poly in &mut self.0 {
            math::ntt(poly);
        }
    }
}
impl Drop for SecretPolys {
    fn drop(&mut self) {
        for poly in &mut self.0 {
            wipe_secret(poly);
        }
    }
}

/// A FIPS public key, with no ambient trust or key-discovery policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifyingKey([u8; PUBLIC_KEY_LENGTH]);

impl VerifyingKey {
    /// Decode an exact-length FIPS public key. Every bit pattern of this length
    /// is canonical: each compressed coefficient occupies exactly ten bits.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        bytes
            .try_into()
            .map(Self)
            .map_err(|_| Error::InvalidPublicKey)
    }

    /// Borrow the canonical public encoding.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; PUBLIC_KEY_LENGTH] {
        &self.0
    }

    /// Verify a pure-message signature under the supplied context.
    pub fn verify(
        &self,
        message: &[u8],
        context: &[u8],
        signature: &Signature,
    ) -> Result<(), Error> {
        let mu = representative(&self.public_hash(), message, context)?;
        let matrix = sampling::matrix(&self.0[..32])?;
        let mut response = signature.response();
        // Public signature data uses the same transform/multiplication home.
        response.transform();
        let mut approximation = matrix_product(&matrix, &response.0);
        let mut challenge = sampling::challenge(&signature.0[..codec::CHALLENGE_LENGTH])?;
        challenge.transform();
        let mut high = [0; N];
        for (row, poly) in approximation.0.iter_mut().enumerate() {
            codec::unpack(
                &self.0[32 + row * 320..32 + (row + 1) * 320],
                10,
                None,
                &mut high,
            );
            for value in &mut high {
                *value <<= 13;
            }
            math::ntt(&mut high);
            for ((value, c), t) in poly.iter_mut().zip(&challenge.0[0]).zip(&high) {
                *value = math::sub(*value, math::mul(*c, *t));
            }
            math::inverse(poly);
        }
        let hints = codec::hints_decode(&signature.0[codec::HINT_START..])?;
        for (poly, hints) in approximation.0.iter_mut().zip(&hints) {
            for (value, hint) in poly.iter_mut().zip(hints) {
                *value = math::use_hint(*hint, *value);
            }
        }
        let expected = commitment(&mu, &approximation.0);
        if constant_time_eq(&expected.0, &signature.0[..codec::CHALLENGE_LENGTH]) {
            Ok(())
        } else {
            Err(Error::InvalidSignature)
        }
    }

    fn public_hash(&self) -> [u8; 64] {
        let mut hash = [0; 64];
        sampling::hash(&[&self.0], &mut hash);
        hash
    }
}

/// An owned, redacted expanded secret. Drop overwrites its owned encoding.
/// Cloning explicitly creates another owned secret with the same discipline.
pub struct SigningKey {
    encoded: SecretBytes,
    public: VerifyingKey,
}
purrdf_hash::debug_non_exhaustive!(SigningKey { public });
impl Clone for SigningKey {
    fn clone(&self) -> Self {
        Self {
            encoded: SecretBytes(self.encoded.0.clone()),
            public: self.public.clone(),
        }
    }
}

impl SigningKey {
    /// FIPS 204 Algorithm 6. The caller owns and must clear the seed itself.
    pub fn from_seed(seed: &[u8; SEED_LENGTH]) -> Result<Self, Error> {
        let mut expanded = SecretBytes::zeros(128);
        sampling::hash(&[seed, &[K as u8, L as u8]], &mut expanded.0);
        let secrets = sampling::secrets(&expanded.0[32..96])?;
        let (public, low) = derive_public(&expanded.0[..32], &secrets.0)?;
        let mut encoded = SecretBytes::zeros(SECRET_KEY_LENGTH);
        encoded.0[..32].copy_from_slice(&expanded.0[..32]);
        encoded.0[32..64].copy_from_slice(&expanded.0[96..]);
        encoded.0[64..128].copy_from_slice(&public.public_hash());
        for (index, poly) in secrets.0.iter().enumerate() {
            codec::pack(
                poly,
                4,
                Some(4),
                &mut encoded.0[128 + 128 * index..128 + 128 * (index + 1)],
            );
        }
        let start = 128 + 128 * (L + K);
        for (index, poly) in low.0.iter().enumerate() {
            codec::pack(
                poly,
                13,
                Some(4096),
                &mut encoded.0[start + 416 * index..start + 416 * (index + 1)],
            );
        }
        Ok(Self { encoded, public })
    }

    /// Import expanded FIPS encoding after validating coefficient ranges, the
    /// public-key hash and t0 against A*s1+s2. The private randomness seed K is
    /// independent arbitrary key material, so it cannot be derived/validated.
    pub fn from_expanded_bytes(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() != SECRET_KEY_LENGTH {
            return Err(Error::InvalidSecretKey);
        }
        let encoded = SecretBytes(bytes.to_vec());
        let polys = decode_secret(&encoded.0);
        if math::norm_fails(&polys.0[..L + K], 5) {
            return Err(Error::InvalidSecretKey);
        }
        let (public, low) = derive_public(&encoded.0[..32], &polys.0[..L + K])?;
        let mut difference = 0;
        for (derived, imported) in low
            .0
            .iter()
            .flatten()
            .zip(polys.0[L + K..].iter().flatten())
        {
            difference |= derived ^ imported;
        }
        let valid = constant_time_eq(&public.public_hash(), &encoded.0[64..128])
            & (core::hint::black_box(difference) == 0);
        if !valid {
            return Err(Error::InvalidSecretKey);
        }
        Ok(Self { encoded, public })
    }

    /// Copy the expanded encoding into caller-owned storage. The caller must
    /// clear that storage; this method does not create an unguarded return copy.
    pub fn export_expanded(&self, out: &mut [u8; SECRET_KEY_LENGTH]) {
        out.copy_from_slice(&self.encoded.0);
    }

    /// The derived public key.
    #[must_use]
    pub const fn verifying_key(&self) -> &VerifyingKey {
        &self.public
    }

    /// Sign using the FIPS deterministic all-zero randomizer, explicitly.
    pub fn sign_deterministic(&self, message: &[u8], context: &[u8]) -> Result<Signature, Error> {
        self.sign_hedged(message, context, &[0; SEED_LENGTH])
    }

    /// Sign using caller-owned fresh cryptographic randomness. No entropy is
    /// obtained here, and no deterministic fallback occurs. Context checking
    /// precedes secret decoding. Caller randomizer storage is not cleared here.
    pub fn sign_hedged(
        &self,
        message: &[u8],
        context: &[u8],
        randomizer: &[u8; SEED_LENGTH],
    ) -> Result<Signature, Error> {
        let mu = representative(&self.encoded.0[64..128], message, context)?;
        let mut private_seed = SecretBytes::zeros(64);
        sampling::hash(
            &[&self.encoded.0[32..64], randomizer, &mu],
            &mut private_seed.0,
        );
        let mut secrets = decode_secret(&self.encoded.0);
        secrets.transform();
        let matrix = sampling::matrix(&self.encoded.0[..32])?;
        let mut nonce = 0u32;
        loop {
            let mask = sampling::mask(&private_seed.0, take_nonce(&mut nonce)?);
            let mut transformed = SecretPolys(mask.0.clone());
            transformed.transform();
            let mut w = matrix_product(&matrix, &transformed.0);
            for poly in &mut w.0 {
                math::inverse(poly);
            }
            let mut w_high = SecretPolys::zeros(K);
            for (high, poly) in w_high.0.iter_mut().zip(&w.0) {
                for (out, value) in high.iter_mut().zip(poly) {
                    *out = math::decompose(*value).0;
                }
            }
            let commitment = commitment(&mu, &w_high.0);
            let mut challenge = sampling::challenge(&commitment.0)?;
            challenge.transform();
            let products = challenge_products(&challenge.0[0], &secrets.0);
            // The challenge is public on successful output, but rejected
            // candidates are controlled scratch and are overwritten as well.
            drop(challenge);
            let mut response = SecretPolys::zeros(L);
            for ((out, y), product) in response.0.iter_mut().zip(&mask.0).zip(&products.0[..L]) {
                for ((z, y), product) in out.iter_mut().zip(y).zip(product) {
                    *z = y + product;
                }
            }
            let mut low = SecretPolys::zeros(K);
            let mut residual = SecretPolys::zeros(K);
            for (((out, residual), w), product) in low
                .0
                .iter_mut()
                .zip(&mut residual.0)
                .zip(&w.0)
                .zip(&products.0[L..L + K])
            {
                for (((low, residual), w), product) in
                    out.iter_mut().zip(residual).zip(w).zip(product)
                {
                    *residual = math::sub(*w, math::canonical(*product));
                    *low = math::decompose(*residual).1;
                }
            }
            if math::norm_fails(&response.0, GAMMA1 - BETA)
                | math::norm_fails(&low.0, GAMMA2 - BETA)
            {
                continue;
            }
            let mut hints = SecretPolys::zeros(K);
            let mut weight = 0;
            for (((hints, residual), product), w_high) in hints
                .0
                .iter_mut()
                .zip(&residual.0)
                .zip(&products.0[L + K..])
                .zip(&w_high.0)
            {
                for (((hint, residual), product), w_high) in
                    hints.iter_mut().zip(residual).zip(product).zip(w_high)
                {
                    let adjusted = math::add(*residual, math::canonical(*product));
                    *hint = i32::from(math::decompose(adjusted).0 != *w_high);
                    weight += *hint;
                }
            }
            let product_invalid = math::norm_fails(&products.0[L + K..], GAMMA2);
            let weight_invalid = weight > codec::OMEGA as i32;
            if product_invalid || weight_invalid {
                continue;
            }
            let mut signature = [0; SIGNATURE_LENGTH];
            signature[..codec::CHALLENGE_LENGTH].copy_from_slice(&commitment.0);
            for (index, poly) in response.0.iter().enumerate() {
                let start = codec::CHALLENGE_LENGTH + index * codec::RESPONSE_BYTES;
                codec::pack(
                    poly,
                    20,
                    Some(GAMMA1),
                    &mut signature[start..start + codec::RESPONSE_BYTES],
                );
            }
            codec::hints_encode(&hints.0, &mut signature[codec::HINT_START..]);
            return Ok(Signature(signature));
        }
    }
}

fn take_nonce(next: &mut u32) -> Result<u16, Error> {
    // Reserve all five polynomial nonces together before deriving any mask.
    // Public counter failure preserves the counter and emits no signature.
    let last = next
        .checked_add(L as u32 - 1)
        .ok_or(Error::NonceExhausted)?;
    u16::try_from(last).map_err(|_| Error::NonceExhausted)?;
    let nonce = *next as u16;
    *next += L as u32;
    Ok(nonce)
}

/// A canonical signature encoding. Decoding does not establish authenticity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signature([u8; SIGNATURE_LENGTH]);
impl Signature {
    /// Decode canonical hints and require the strict ML-DSA-65 response bound.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        let signature = Self(bytes.try_into().map_err(|_| Error::InvalidSignature)?);
        codec::hints_decode(&signature.0[codec::HINT_START..])?;
        if math::norm_fails(&signature.response().0, GAMMA1 - BETA) {
            return Err(Error::InvalidSignature);
        }
        Ok(signature)
    }

    /// Borrow the canonical encoding.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; SIGNATURE_LENGTH] {
        &self.0
    }

    fn response(&self) -> SecretPolys {
        let mut response = SecretPolys::zeros(L);
        for (index, poly) in response.0.iter_mut().enumerate() {
            let start = codec::CHALLENGE_LENGTH + index * codec::RESPONSE_BYTES;
            codec::unpack(
                &self.0[start..start + codec::RESPONSE_BYTES],
                20,
                Some(GAMMA1),
                poly,
            );
        }
        response
    }
}

fn decode_secret(bytes: &[u8]) -> SecretPolys {
    let mut polys = SecretPolys::zeros(L + 2 * K);
    for (index, poly) in polys.0[..L + K].iter_mut().enumerate() {
        codec::unpack(
            &bytes[128 + index * 128..128 + (index + 1) * 128],
            4,
            Some(4),
            poly,
        );
    }
    let start = 128 + 128 * (L + K);
    for (index, poly) in polys.0[L + K..].iter_mut().enumerate() {
        codec::unpack(
            &bytes[start + index * 416..start + (index + 1) * 416],
            13,
            Some(4096),
            poly,
        );
    }
    polys
}

fn matrix_product(matrix: &[Poly], vector: &[Poly]) -> SecretPolys {
    let mut output = SecretPolys::zeros(K);
    for (row, poly) in output.0.iter_mut().enumerate() {
        for column in 0..L {
            for ((out, a), s) in poly
                .iter_mut()
                .zip(&matrix[row * L + column])
                .zip(&vector[column])
            {
                *out = math::add(*out, math::mul(*a, *s));
            }
        }
    }
    output
}

fn derive_public(rho: &[u8], secrets: &[Poly]) -> Result<(VerifyingKey, SecretPolys), Error> {
    let matrix = sampling::matrix(rho)?;
    let mut s1 = SecretPolys(secrets[..L].to_vec());
    s1.transform();
    let mut t = matrix_product(&matrix, &s1.0);
    let mut encoded = [0; PUBLIC_KEY_LENGTH];
    encoded[..32].copy_from_slice(rho);
    let mut low = SecretPolys::zeros(K);
    for (index, ((poly, s2), low)) in
        t.0.iter_mut()
            .zip(&secrets[L..])
            .zip(&mut low.0)
            .enumerate()
    {
        math::inverse(poly);
        for ((value, secret), low) in poly.iter_mut().zip(s2).zip(low) {
            let (high, bottom) = math::power_round(math::add(*value, math::canonical(*secret)));
            *value = high;
            *low = bottom;
        }
        codec::pack(
            poly,
            10,
            None,
            &mut encoded[32 + index * 320..32 + (index + 1) * 320],
        );
    }
    Ok((VerifyingKey(encoded), low))
}

fn challenge_products(challenge: &Poly, secrets: &[Poly]) -> SecretPolys {
    let mut products = SecretPolys::zeros(secrets.len());
    for (product, secret) in products.0.iter_mut().zip(secrets) {
        for ((out, c), s) in product.iter_mut().zip(challenge).zip(secret) {
            *out = math::mul(*c, *s);
        }
        math::inverse(product);
        for value in product {
            *value = math::centered(*value);
        }
    }
    products
}

fn representative(tr: &[u8], message: &[u8], context: &[u8]) -> Result<[u8; 64], Error> {
    let length = u8::try_from(context.len()).map_err(|_| Error::ContextTooLong)?;
    let mut mu = [0; 64];
    sampling::hash(&[tr, &[0, length], context, message], &mut mu);
    Ok(mu)
}

fn commitment(mu: &[u8], highs: &[Poly]) -> SecretBytes {
    let mut encoded = SecretBytes::zeros(128 * K);
    for (index, poly) in highs.iter().enumerate() {
        codec::pack(
            poly,
            4,
            None,
            &mut encoded.0[index * 128..(index + 1) * 128],
        );
    }
    let mut commitment = SecretBytes::zeros(codec::CHALLENGE_LENGTH);
    sampling::hash(&[mu, &encoded.0], &mut commitment.0);
    commitment
}

#[cfg(test)]
mod tests {
    use super::{Error, take_nonce};

    #[test]
    fn signing_nonce_groups_are_disjoint_and_exhaustion_is_typed() {
        let mut next = 0;
        for group in 0..13_107 {
            assert_eq!(u32::from(take_nonce(&mut next).unwrap()), group * 5);
        }
        assert_eq!(next, 65_535);
        assert_eq!(take_nonce(&mut next), Err(Error::NonceExhausted));
        assert_eq!(next, 65_535);
        assert_eq!(take_nonce(&mut next), Err(Error::NonceExhausted));
        next = u32::MAX;
        assert_eq!(take_nonce(&mut next), Err(Error::NonceExhausted));
        assert_eq!(next, u32::MAX);
    }
}
