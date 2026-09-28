// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! 64-bit FNV-1a (Fowler–Noll–Vo): a byte-at-a-time fingerprint for short
//! identifiers, dictionary ids and schema fingerprints.
//!
//! `hash = offset; for each byte: hash ^= byte; hash *= prime`, wrapping,
//! with the standard 64-bit parameters [`FNV1A64_OFFSET`] and
//! [`FNV1A64_PRIME`]. Unlike the [`fixed`](crate::fixed) table hasher, FNV-1a
//! is the same function on every build and target, so a value may be
//! persisted, compared across processes and written into a file format. It
//! is a *fingerprint*, not a digest: it offers no resistance to chosen
//! collisions, so anything adversarial needs BLAKE3 or SHA-3.
//!
//! ```
//! use purrdf_hash::fnv::{fnv1a64, Fnv1a};
//!
//! assert_eq!(fnv1a64(b"foobar"), 0x8594_4171_f739_67e8);
//! let mut streaming = Fnv1a::new();
//! streaming.update(b"foo");
//! streaming.update(b"bar");
//! assert_eq!(streaming.finish(), fnv1a64(b"foobar"));
//! ```

/// The FNV-1a 64-bit offset basis: the hash of the empty input.
pub const FNV1A64_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;

/// The FNV-1a 64-bit prime, `2^40 + 2^8 + 0xb3`.
pub const FNV1A64_PRIME: u64 = 0x0000_0100_0000_01b3;

/// The FNV-1a 64-bit hash of `bytes`.
#[must_use]
#[inline]
pub const fn fnv1a64(bytes: &[u8]) -> u64 {
    fnv1a64_with(FNV1A64_OFFSET, bytes)
}

/// The FNV-1a 64-bit hash of `bytes` continued from `basis`: with the hash
/// of a prefix as the basis, this is the hash of the concatenation, so
/// `fnv1a64_with(fnv1a64(a), b) == fnv1a64(a ++ b)`.
#[must_use]
#[inline]
pub const fn fnv1a64_with(basis: u64, bytes: &[u8]) -> u64 {
    let mut hash = basis;
    let mut i = 0;
    while i < bytes.len() {
        hash = (hash ^ bytes[i] as u64).wrapping_mul(FNV1A64_PRIME);
        i += 1;
    }
    hash
}

/// A streaming FNV-1a 64-bit hash: any split of the input over
/// [`update`](Self::update) and [`update_byte`](Self::update_byte) gives
/// [`fnv1a64`] of the concatenation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fnv1a(u64);

impl Fnv1a {
    /// A hash over no bytes yet: [`finish`](Self::finish) is
    /// [`FNV1A64_OFFSET`].
    #[must_use]
    #[inline]
    pub const fn new() -> Self {
        Self(FNV1A64_OFFSET)
    }

    /// A hash continued from `basis`, as [`fnv1a64_with`] does.
    #[must_use]
    #[inline]
    pub const fn with_basis(basis: u64) -> Self {
        Self(basis)
    }

    /// Absorb `bytes`.
    #[inline]
    pub const fn update(&mut self, bytes: &[u8]) {
        self.0 = fnv1a64_with(self.0, bytes);
    }

    /// Absorb one byte.
    #[inline]
    pub const fn update_byte(&mut self, byte: u8) {
        self.0 = (self.0 ^ byte as u64).wrapping_mul(FNV1A64_PRIME);
    }

    /// The hash of everything absorbed. The state is unchanged, so more
    /// bytes may follow.
    #[must_use]
    #[inline]
    pub const fn finish(&self) -> u64 {
        self.0
    }
}

impl Default for Fnv1a {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{FNV1A64_OFFSET, FNV1A64_PRIME, Fnv1a, fnv1a64, fnv1a64_with};

    #[test]
    fn matches_the_published_fnv1a_64_answers() {
        assert_eq!(FNV1A64_OFFSET, 0xcbf2_9ce4_8422_2325);
        assert_eq!(FNV1A64_PRIME, (1 << 40) + (1 << 8) + 0xb3);
        assert_eq!(fnv1a64(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a64(b"a"), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(fnv1a64(b"foobar"), 0x8594_4171_f739_67e8);
        // Evaluated at compile time too.
        const A: u64 = fnv1a64(b"a");
        assert_eq!(A, 0xaf63_dc4c_8601_ec8c);
    }

    #[test]
    fn streaming_matches_the_one_shot_on_every_split() {
        let input = b"foobar";
        for split in 0..=input.len() {
            let mut hash = Fnv1a::new();
            hash.update(&input[..split]);
            hash.update(&input[split..]);
            assert_eq!(hash.finish(), fnv1a64(input), "split at {split}");
            assert_eq!(
                fnv1a64_with(fnv1a64(&input[..split]), &input[split..]),
                fnv1a64(input)
            );
        }
        let mut bytewise = Fnv1a::default();
        for &byte in input {
            bytewise.update_byte(byte);
        }
        assert_eq!(bytewise.finish(), 0x8594_4171_f739_67e8);
        assert_eq!(Fnv1a::new().finish(), FNV1A64_OFFSET);
        let mut continued = Fnv1a::with_basis(fnv1a64(b"foo"));
        continued.update(b"bar");
        assert_eq!(continued.finish(), 0x8594_4171_f739_67e8);
        assert_eq!(fnv1a64_with(0, b""), 0);
    }
}
