// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The object-safe streaming interface every hasher in this crate implements.

/// The longest output of any [`Digest`] in this crate (SHA3-512), so a caller
/// dispatching through `&mut dyn Digest` can finalize into a stack buffer.
pub const MAX_OUTPUT_LEN: usize = 64;

/// A streaming hash function, usable as `&mut dyn Digest`.
///
/// Each hasher also has inherent `update`/`finalize` methods and a one-shot
/// `digest` function; this trait exists for callers that pick the algorithm
/// at run time.
///
/// ```
/// use purrdf_hash::{Digest, MAX_OUTPUT_LEN};
/// use purrdf_hash::{md5::Md5, sha1::Sha1, sha3::Sha3_256};
///
/// for hasher in [
///     &mut Md5::new() as &mut dyn Digest,
///     &mut Sha1::new(),
///     &mut Sha3_256::new(),
/// ] {
///     hasher.update(b"a");
///     hasher.update(b"bc");
///     let mut out = [0u8; MAX_OUTPUT_LEN];
///     let len = hasher.finalize_reset(&mut out);
///     assert_eq!(len, hasher.output_len());
/// }
/// ```
pub trait Digest {
    /// The number of bytes [`finalize_reset`](Self::finalize_reset) writes.
    fn output_len(&self) -> usize;

    /// Absorb `data`.
    fn update(&mut self, data: &[u8]);

    /// Write the digest of everything absorbed since the last reset into the
    /// first [`output_len`](Self::output_len) bytes of `out`, return that
    /// length, and reset the hasher to its initial state.
    ///
    /// # Panics
    ///
    /// If `out` is shorter than [`output_len`](Self::output_len).
    fn finalize_reset(&mut self, out: &mut [u8]) -> usize;

    /// Discard everything absorbed and return to the initial state.
    fn reset(&mut self);
}

/// A 32-byte digest — BLAKE3-256, SHA3-256 or any other 256-bit output — as
/// bytes plus one canonical text form.
///
/// The text form is 64 lowercase base16 digits: `Display`, `Debug` and
/// [`to_hex`](Self::to_hex) all render it, through the same kernel as
/// [`hex::Lower`](crate::hex::Lower), and [`from_hex`](Self::from_hex) reads
/// it back, accepting uppercase digits as well. Ordering is byte order, so a
/// list sorted by digest is also sorted by its hex text.
///
/// ```
/// use purrdf_hash::{blake3, Digest32};
///
/// let digest = Digest32::from(blake3::hash(b"abc"));
/// assert_eq!(digest.to_hex().len(), 64);
/// assert_eq!(Digest32::from_hex(&digest.to_hex()), Ok(digest));
/// assert_eq!(format!("{digest}"), format!("{digest:?}"));
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Digest32(pub [u8; 32]);

impl Digest32 {
    /// The digest bytes.
    #[must_use]
    #[inline]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Parses exactly 64 base16 digits, in either case, through
    /// [`hex::decode_32`](crate::hex::decode_32).
    ///
    /// # Errors
    ///
    /// [`HexError`](crate::hex::HexError) for any other length or any byte
    /// that is not a base16 digit: no sign, prefix or whitespace is accepted.
    #[inline]
    pub fn from_hex(text: &str) -> Result<Self, crate::hex::HexError> {
        crate::hex::decode_32(text).map(Self)
    }

    /// The 64 lowercase base16 digits, as a `String`.
    #[must_use]
    #[inline]
    pub fn to_hex(&self) -> String {
        crate::hex::lower(&self.0)
    }
}

impl core::fmt::Display for Digest32 {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Display::fmt(&crate::hex::Lower(&self.0), f)
    }
}

impl core::fmt::Debug for Digest32 {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Display::fmt(self, f)
    }
}

impl From<[u8; 32]> for Digest32 {
    #[inline]
    fn from(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

impl From<Digest32> for [u8; 32] {
    #[inline]
    fn from(digest: Digest32) -> Self {
        digest.0
    }
}

impl From<crate::blake3::Hash> for Digest32 {
    #[inline]
    fn from(hash: crate::blake3::Hash) -> Self {
        Self(*hash.as_bytes())
    }
}

impl AsRef<[u8]> for Digest32 {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::Digest32;
    use crate::hex::HexError;

    const BYTES: [u8; 32] = [
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e,
        0x0f, 0xf0, 0xf1, 0xf2, 0xf3, 0xf4, 0xf5, 0xf6, 0xf7, 0xf8, 0xf9, 0xfa, 0xfb, 0xfc, 0xfd,
        0xfe, 0xff,
    ];
    const TEXT: &str = "000102030405060708090a0b0c0d0e0ff0f1f2f3f4f5f6f7f8f9fafbfcfdfeff";

    #[test]
    fn renders_as_64_lowercase_digits_in_every_text_form() {
        let digest = Digest32(BYTES);
        assert_eq!(digest.to_hex(), TEXT);
        assert_eq!(digest.to_string(), TEXT);
        assert_eq!(format!("{digest:?}"), TEXT);
        assert_eq!(format!("{digest:.6}"), "000102");
        assert_eq!(format!("[{:>6.4}]", Digest32([0xab; 32])), "[  abab]");
        assert_eq!(*digest.as_bytes(), BYTES);
        assert_eq!(digest.as_ref(), &BYTES[..]);
        assert_eq!(<[u8; 32]>::from(digest), BYTES);
        assert_eq!(Digest32::from(BYTES), digest);
    }

    #[test]
    fn parses_either_case_and_refuses_everything_else() {
        assert_eq!(Digest32::from_hex(TEXT), Ok(Digest32(BYTES)));
        assert_eq!(
            Digest32::from_hex(&TEXT.to_ascii_uppercase()),
            Ok(Digest32(BYTES))
        );
        assert_eq!(
            Digest32::from_hex(&TEXT[..63]),
            Err(HexError::Length {
                expected: 64,
                actual: 63
            })
        );
        assert_eq!(
            Digest32::from_hex(&format!("{TEXT}0")),
            Err(HexError::Length {
                expected: 64,
                actual: 65
            })
        );
        assert_eq!(
            Digest32::from_hex(&format!("+{}", &TEXT[1..])),
            Err(HexError::InvalidDigit {
                index: 0,
                byte: b'+'
            })
        );
        assert_eq!(
            Digest32::from_hex(&format!("{} ", &TEXT[..63])),
            Err(HexError::InvalidDigit {
                index: 63,
                byte: b' '
            })
        );
    }

    #[test]
    fn orders_by_bytes_and_hashes_as_its_bytes() {
        let low = Digest32([0x00; 32]);
        let mut high = low;
        high.0[31] = 1;
        let mut higher = low;
        higher.0[0] = 1;
        assert!(low < high && high < higher);
        assert!(low.to_hex() < high.to_hex() && high.to_hex() < higher.to_hex());
        assert_eq!(
            crate::fixed::hash_one(&low),
            crate::fixed::hash_one(&Digest32([0x00; 32]))
        );
    }

    #[test]
    fn converts_from_a_blake3_hash_and_renders_it_alike() {
        let hash = crate::blake3::hash(b"abc");
        let digest = Digest32::from(hash);
        assert_eq!(digest.as_bytes(), hash.as_bytes());
        assert_eq!(digest.to_string(), hash.to_string());
        assert_eq!(Digest32::from_hex(&hash.to_string()), Ok(digest));
    }
}
