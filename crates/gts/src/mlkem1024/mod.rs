// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native ML-KEM-1024 from FIPS 203, with caller-owned entropy.
//!
//! [`DecapsulationKey::from_seed`] expands the exact 64-byte `(d, z)` seed.
//! [`EncapsulationKey::encapsulate`] consumes the caller's 32-byte `m` directly,
//! as Algorithm 17 specifies. Neither operation obtains entropy from a host.
//! Encodings are the FIPS byte arrays, without a container or hybrid protocol.
//!
//! Immutable encapsulation keys cache the required length and modulus checks.
//! Expanded decapsulation keys cache exactly the required length and embedded
//! public-key hash checks. Those checks do not establish key-pair consistency.
//! Every correctly sized ciphertext produces a secret; decapsulation never
//! exposes whether its internal re-encryption matched.
//!
//! # Secret handling
//!
//! Secret arithmetic, CBD sampling, compression and packing have fixed schedules
//! and no secret-dependent memory indices. Matrix rejection sampling varies
//! only with the public matrix seed. This is a source-level property, not a
//! certification of compiler, JIT or hardware timing. Fixed-divisor integer
//! arithmetic still depends on target lowering.
//!
//! Owned private keys, shared secrets, secret polynomials, hash outputs,
//! re-encrypted candidate ciphertext and rejection masks use
//! [`purrdf_hash::SecretArray`], which overwrites its owned storage on drop.
//! SHA-3 and SHAKE clear their owned state through the same existing home.
//! Caller seeds, randomizers and exported private bytes remain the caller's
//! clearing responsibility. Historical compiler copies, registers and spills
//! are not guaranteed to be erased, and safe overwriting is not physical erasure.

mod math;
mod sampling;

use core::fmt;
use purrdf_ed25519::constant_time_eq;
use purrdf_hash::SecretArray;
use purrdf_hash::sha3::{Sha3_256, Sha3_512, Shake256};

/// Encoded `(d, z)` key-generation seed length.
pub const SEED_LENGTH: usize = 64;
/// Caller-supplied encapsulation randomness length.
pub const RANDOMNESS_LENGTH: usize = 32;
/// FIPS 203 ML-KEM-1024 encapsulation key length.
pub const ENCAPSULATION_KEY_LENGTH: usize = 1568;
/// FIPS 203 ML-KEM-1024 expanded decapsulation key length.
pub const DECAPSULATION_KEY_LENGTH: usize = 3168;
/// FIPS 203 ML-KEM-1024 ciphertext length.
pub const CIPHERTEXT_LENGTH: usize = 1568;
/// ML-KEM shared-secret length.
pub const SHARED_SECRET_LENGTH: usize = 32;

const PKE_SECRET_LENGTH: usize = 384 * math::K;
const PUBLIC_HASH_START: usize = PKE_SECRET_LENGTH + ENCAPSULATION_KEY_LENGTH;
const REJECTION_SEED_START: usize = PUBLIC_HASH_START + 32;
const DU: usize = 11;
const DV: usize = 5;
const U_LENGTH: usize = 32 * DU * math::K;

/// A failed FIPS input check, distinct from implicit ciphertext rejection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// Encapsulation key bytes have the wrong length.
    EncapsulationKeyLength {
        /// Original byte count.
        actual: usize,
    },
    /// An encoded public coefficient is outside the canonical field range.
    EncapsulationKeyModulus {
        /// Coefficient ordinal in the concatenated public vector.
        coefficient: usize,
        /// Original unreduced 12-bit value.
        value: u16,
    },
    /// Expanded decapsulation key bytes have the wrong length.
    DecapsulationKeyLength {
        /// Original byte count.
        actual: usize,
    },
    /// The encoded public-key hash disagrees with the embedded public bytes.
    DecapsulationKeyHash {
        /// Public-key hash stored in the original encoding.
        expected: [u8; 32],
        /// Hash of the original embedded public bytes.
        actual: [u8; 32],
    },
    /// Ciphertext bytes have the wrong public length.
    CiphertextLength {
        /// Original byte count.
        actual: usize,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EncapsulationKeyLength { actual } => write!(
                f,
                "ML-KEM-1024 encapsulation key has {actual} bytes; expected {ENCAPSULATION_KEY_LENGTH}"
            ),
            Self::EncapsulationKeyModulus { coefficient, value } => write!(
                f,
                "ML-KEM-1024 public coefficient {coefficient} is {value}; expected a value below {}",
                math::Q
            ),
            Self::DecapsulationKeyLength { actual } => write!(
                f,
                "ML-KEM-1024 decapsulation key has {actual} bytes; expected {DECAPSULATION_KEY_LENGTH}"
            ),
            Self::DecapsulationKeyHash { .. } => {
                f.write_str("ML-KEM-1024 decapsulation key public-key hash mismatch")
            }
            Self::CiphertextLength { actual } => write!(
                f,
                "ML-KEM-1024 ciphertext has {actual} bytes; expected {CIPHERTEXT_LENGTH}"
            ),
        }
    }
}
impl std::error::Error for Error {}

impl purrdf_hash::error::StructuredError for Error {
    purrdf_hash::error_visit! {
        EncapsulationKeyLength { actual } => "mlkem1024-encapsulation-key-length" {
            "actual" => ByteCount(*actual as u128), "expected" => ByteCount(ENCAPSULATION_KEY_LENGTH as u128),
        },
        EncapsulationKeyModulus { coefficient, value } => "mlkem1024-encapsulation-key-modulus" {
            "coefficient" => Unsigned(*coefficient as u128), "value" => Unsigned(u128::from(*value)),
            "modulus" => Unsigned(math::Q as u128),
        },
        DecapsulationKeyLength { actual } => "mlkem1024-decapsulation-key-length" {
            "actual" => ByteCount(*actual as u128), "expected" => ByteCount(DECAPSULATION_KEY_LENGTH as u128),
        },
        DecapsulationKeyHash { expected, actual } => "mlkem1024-decapsulation-key-hash" {
            "expected" => Bytes(expected), "actual" => Bytes(actual),
        },
        CiphertextLength { actual } => "mlkem1024-ciphertext-length" {
            "actual" => ByteCount(*actual as u128), "expected" => ByteCount(CIPHERTEXT_LENGTH as u128),
        },
    }
}

/// An immutable encapsulation key with its FIPS public input checks complete.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncapsulationKey([u8; ENCAPSULATION_KEY_LENGTH]);

impl EncapsulationKey {
    /// Check the exact length and canonical coefficient encoding (FIPS 203 §7.2).
    ///
    /// # Errors
    /// Returns the original length or first out-of-range public coefficient.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        let encoded = <&[u8; ENCAPSULATION_KEY_LENGTH]>::try_from(bytes).map_err(|_| {
            Error::EncapsulationKeyLength {
                actual: bytes.len(),
            }
        })?;
        let mut coefficients = [0; math::N];
        for (row, bytes) in encoded[..PKE_SECRET_LENGTH]
            .as_chunks::<384>()
            .0
            .iter()
            .enumerate()
        {
            crate::lattice::unpack(bytes, 12, None, &mut coefficients);
            for (column, &value) in coefficients.iter().enumerate() {
                if value >= math::Q {
                    return Err(Error::EncapsulationKeyModulus {
                        coefficient: row * math::N + column,
                        // The shared twelve-bit decoder returns 0..4096.
                        value: value as u16,
                    });
                }
            }
        }
        Ok(Self(*encoded))
    }

    /// Borrow the unwrapped FIPS public encoding.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; ENCAPSULATION_KEY_LENGTH] {
        &self.0
    }

    /// Encapsulate with the caller's uniformly random `m`, consumed directly.
    /// Returns `(ciphertext, shared_secret)`; no entropy is obtained internally.
    #[must_use]
    pub fn encapsulate(&self, m: &[u8; RANDOMNESS_LENGTH]) -> (Ciphertext, SharedSecret) {
        let expanded = expand_message(m, &Sha3_256::digest(&self.0));
        let mut ciphertext = Ciphertext([0; CIPHERTEXT_LENGTH]);
        pke_encrypt(&self.0, m, &expanded[32..], &mut ciphertext.0);
        let mut secret = SecretArray::new([0; SHARED_SECRET_LENGTH]);
        secret.copy_from_slice(&expanded[..32]);
        (ciphertext, SharedSecret(secret))
    }
}

/// An expanded private key whose owned bytes are cleared on drop.
pub struct DecapsulationKey {
    bytes: SecretArray<u8, DECAPSULATION_KEY_LENGTH>,
}
purrdf_hash::debug_non_exhaustive!(DecapsulationKey {});

impl DecapsulationKey {
    /// Deterministically expand the caller's exact `(d, z)` seed (Algorithm 16).
    #[must_use]
    pub fn from_seed(seed: &[u8; SEED_LENGTH]) -> Self {
        let mut key = Self {
            bytes: SecretArray::new([0; DECAPSULATION_KEY_LENGTH]),
        };
        let mut public = [0; ENCAPSULATION_KEY_LENGTH];
        pke_keygen(
            &seed[..32],
            &mut key.bytes[..PKE_SECRET_LENGTH],
            &mut public,
        );
        key.bytes[PKE_SECRET_LENGTH..PUBLIC_HASH_START].copy_from_slice(&public);
        key.bytes[PUBLIC_HASH_START..REJECTION_SEED_START]
            .copy_from_slice(&Sha3_256::digest(&public));
        key.bytes[REJECTION_SEED_START..].copy_from_slice(&seed[32..]);
        key
    }

    /// Import expanded bytes with exactly the length and hash checks in §7.3.
    /// This does not validate secret coefficients or prove key-pair consistency.
    ///
    /// # Errors
    /// Returns the original wrong length or the two disagreeing public hashes.
    pub fn from_expanded_bytes(bytes: &[u8]) -> Result<Self, Error> {
        let encoded = <&[u8; DECAPSULATION_KEY_LENGTH]>::try_from(bytes).map_err(|_| {
            Error::DecapsulationKeyLength {
                actual: bytes.len(),
            }
        })?;
        let actual = Sha3_256::digest(&encoded[PKE_SECRET_LENGTH..PUBLIC_HASH_START]);
        let mut expected = [0; 32];
        expected.copy_from_slice(&encoded[PUBLIC_HASH_START..REJECTION_SEED_START]);
        if !constant_time_eq(&actual, &expected) {
            return Err(Error::DecapsulationKeyHash { expected, actual });
        }
        Ok(Self {
            bytes: SecretArray::new(*encoded),
        })
    }

    /// Extract and validate the embedded public key independently under §7.2.
    ///
    /// # Errors
    /// A hash-consistent imported private key may embed noncanonical public
    /// coefficients. Extraction then fails, while decapsulation still follows
    /// the exact FIPS private-key input contract.
    pub fn encapsulation_key(&self) -> Result<EncapsulationKey, Error> {
        EncapsulationKey::from_bytes(&self.bytes[PKE_SECRET_LENGTH..PUBLIC_HASH_START])
    }

    /// Copy the exact expanded private encoding into caller-owned storage.
    /// The caller must clear this exported copy independently.
    pub fn export_expanded(&self, output: &mut [u8; DECAPSULATION_KEY_LENGTH]) {
        output.copy_from_slice(&self.bytes[..]);
    }

    /// Decapsulate every correctly sized ciphertext, including implicit rejection.
    /// No ciphertext validity flag or failure indication is returned.
    #[must_use]
    pub fn decapsulate(&self, ciphertext: &Ciphertext) -> SharedSecret {
        let m = pke_decrypt(&self.bytes[..PKE_SECRET_LENGTH], &ciphertext.0);
        let candidate =
            expand_message(&m[..], &self.bytes[PUBLIC_HASH_START..REJECTION_SEED_START]);
        let mut fallback = SecretArray::new([0; SHARED_SECRET_LENGTH]);
        let mut rejection = Shake256::new();
        rejection.update(&self.bytes[REJECTION_SEED_START..]);
        rejection.update(&ciphertext.0);
        rejection.finalize().squeeze(&mut fallback[..]);
        let mut compared = SecretArray::new([0; CIPHERTEXT_LENGTH]);
        // Use the raw embedded encoding: §7.3 imposes no public modulus check.
        pke_encrypt(
            &self.bytes[PKE_SECRET_LENGTH..PUBLIC_HASH_START],
            &m[..],
            &candidate[32..],
            &mut compared[..],
        );
        let mask = SecretArray::new([core::hint::black_box(u8::from(constant_time_eq(
            &ciphertext.0,
            &compared[..],
        )))
        .wrapping_neg()]);
        let mut secret = SecretArray::new([0; SHARED_SECRET_LENGTH]);
        for index in 0..SHARED_SECRET_LENGTH {
            secret[index] = (candidate[index] & mask[0]) | (fallback[index] & !mask[0]);
        }
        SharedSecret(secret)
    }
}

/// A fixed-length FIPS ciphertext. Every bit string of this length is admitted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ciphertext([u8; CIPHERTEXT_LENGTH]);

impl Ciphertext {
    /// Check the public ciphertext length; contents are handled by decapsulation.
    ///
    /// # Errors
    /// Returns only an incorrect byte length, never a ciphertext validity result.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        bytes
            .try_into()
            .map(Self)
            .map_err(|_| Error::CiphertextLength {
                actual: bytes.len(),
            })
    }

    /// Borrow the unwrapped FIPS ciphertext encoding.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; CIPHERTEXT_LENGTH] {
        &self.0
    }
}

/// A shared secret whose owned bytes are cleared on drop, with redacted debug.
pub struct SharedSecret(SecretArray<u8, SHARED_SECRET_LENGTH>);
purrdf_hash::debug_non_exhaustive!(SharedSecret {});

impl SharedSecret {
    /// Borrow the secret for the caller's key derivation or protocol.
    /// Copies made by that caller require independent clearing.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8; SHARED_SECRET_LENGTH] {
        &self.0
    }
}

fn expand_message(message: &[u8], public_hash: &[u8]) -> SecretArray<u8, 64> {
    let mut hash = Sha3_512::new();
    hash.update(message);
    hash.update(public_hash);
    SecretArray::new(hash.finalize())
}

fn secret_vector() -> [SecretArray<i32, { math::N }>; math::K] {
    std::array::from_fn(|_| SecretArray::new([0; math::N]))
}

/// Algorithm 13, expanding matrix rows on demand without retaining a matrix.
fn pke_keygen(seed: &[u8], secret: &mut [u8], public: &mut [u8]) {
    let mut hash = Sha3_512::new();
    hash.update(seed);
    hash.update(&[math::K as u8]);
    let expanded = SecretArray::new(hash.finalize());
    let mut s = secret_vector();
    for (row, poly) in s.iter_mut().enumerate() {
        sampling::noise(&expanded[32..], row as u8, poly);
        math::Field::ntt(poly);
        crate::lattice::pack(poly, 12, None, &mut secret[row * 384..(row + 1) * 384]);
    }
    let mut entry = SecretArray::new([0; math::N]);
    for row in 0..math::K {
        let mut t = SecretArray::new([0; math::N]);
        sampling::noise(&expanded[32..], (math::K + row) as u8, &mut t);
        math::Field::ntt(&mut t);
        for (column, poly) in s.iter().enumerate() {
            sampling::matrix_entry(&expanded[..32], row as u8, column as u8, &mut entry);
            math::accumulate_product(&mut t, &entry, poly);
        }
        crate::lattice::pack(&t, 12, None, &mut public[row * 384..(row + 1) * 384]);
    }
    public[PKE_SECRET_LENGTH..].copy_from_slice(&expanded[..32]);
}

/// Algorithm 14. Public matrix entries are regenerated as needed; all secret
/// polynomial owners survive until their complete use and clear on drop.
fn pke_encrypt(public: &[u8], message: &[u8], randomizer: &[u8], output: &mut [u8]) {
    let mut y = secret_vector();
    for (row, poly) in y.iter_mut().enumerate() {
        sampling::noise(randomizer, row as u8, poly);
        math::Field::ntt(poly);
    }
    let rho = &public[PKE_SECRET_LENGTH..];
    let mut entry = SecretArray::new([0; math::N]);
    let mut noise = SecretArray::new([0; math::N]);
    for row in 0..math::K {
        let mut u = SecretArray::new([0; math::N]);
        for (column, poly) in y.iter().enumerate() {
            sampling::matrix_entry(rho, column as u8, row as u8, &mut entry);
            math::accumulate_product(&mut u, &entry, poly);
        }
        math::Field::inverse(&mut u);
        sampling::noise(randomizer, (math::K + row) as u8, &mut noise);
        for (value, error) in u.iter_mut().zip(noise.iter()) {
            *value = math::Field::add(*value, *error);
        }
        math::compress_encode(&u, DU, &mut output[row * 32 * DU..(row + 1) * 32 * DU]);
    }
    let mut v = SecretArray::new([0; math::N]);
    for (row, poly) in y.iter().enumerate() {
        math::decode(&public[row * 384..(row + 1) * 384], 12, &mut entry);
        math::accumulate_product(&mut v, &entry, poly);
    }
    math::Field::inverse(&mut v);
    sampling::noise(randomizer, (2 * math::K) as u8, &mut noise);
    let mut encoded_message = SecretArray::new([0; math::N]);
    math::decode_decompress(message, 1, &mut encoded_message);
    for ((value, error), message) in v.iter_mut().zip(noise.iter()).zip(encoded_message.iter()) {
        *value = math::Field::add(math::Field::add(*value, *error), *message);
    }
    math::compress_encode(&v, DV, &mut output[U_LENGTH..]);
}

/// Algorithm 15. Decode each vector coordinate under a scoped secret owner.
fn pke_decrypt(secret: &[u8], ciphertext: &[u8]) -> SecretArray<u8, 32> {
    let mut product = SecretArray::new([0; math::N]);
    let mut u = SecretArray::new([0; math::N]);
    let mut s = SecretArray::new([0; math::N]);
    for row in 0..math::K {
        math::decode_decompress(&ciphertext[row * 32 * DU..(row + 1) * 32 * DU], DU, &mut u);
        math::Field::ntt(&mut u);
        math::decode(&secret[row * 384..(row + 1) * 384], 12, &mut s);
        math::accumulate_product(&mut product, &s, &u);
    }
    math::Field::inverse(&mut product);
    let mut v = SecretArray::new([0; math::N]);
    math::decode_decompress(&ciphertext[U_LENGTH..], DV, &mut v);
    for (value, product) in v.iter_mut().zip(product.iter()) {
        *value = math::Field::sub(*value, *product);
    }
    let mut message = SecretArray::new([0; 32]);
    math::compress_encode(&v, 1, &mut message[..]);
    message
}
