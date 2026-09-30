// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! FNV-1a, 64-bit (Fowler, Noll and Vo; the IETF draft
//! `draft-eastlake-fnv`): for each byte, XOR it into the state, then multiply
//! the state by the FNV prime modulo 2^64, starting from the offset basis.
//!
//! [`fold`] is the byte loop over a caller's state, so a caller can digest a
//! sequence of byte strings, or start from its own seed; [`fnv1a64`] is
//! [`fold`] from [`BASIS`]. Both are `const` and target-independent: the
//! multiply is defined modulo 2^64, so it is a wrapping multiply that never
//! panics under overflow checks, and a value is the same on every target,
//! wasm32 included.
//!
//! FNV-1a is a fast, well-dispersed checksum over short keys, not a
//! cryptographic hash: a digest that must resist deliberate collisions uses
//! [`blake3`](crate::blake3) or [`sha3`](crate::sha3).

/// The 64-bit FNV offset basis, `0xCBF29CE484222325`.
pub const BASIS: u64 = 0xCBF2_9CE4_8422_2325;

/// The 64-bit FNV prime, `2^40 + 2^8 + 0xB3` (`0x100000001B3`).
pub const PRIME: u64 = 0x0000_0100_0000_01B3;

/// FNV-1a over `bytes`, folded into `state`: each byte is XORed into the
/// state, which is then multiplied by [`PRIME`] modulo 2^64.
#[must_use]
pub const fn fold(state: u64, bytes: &[u8]) -> u64 {
    let mut state = state;
    let mut index = 0;
    while index < bytes.len() {
        state ^= bytes[index] as u64;
        state = state.wrapping_mul(PRIME);
        index += 1;
    }
    state
}

/// The 64-bit FNV-1a hash of `bytes`: [`fold`] from [`BASIS`].
#[must_use]
pub const fn fnv1a64(bytes: &[u8]) -> u64 {
    fold(BASIS, bytes)
}

#[cfg(test)]
mod tests {
    use super::{BASIS, PRIME, fnv1a64, fold};

    /// The published FNV-1a 64-bit test values (the FNV reference test suite).
    #[test]
    fn fnv1a64_matches_the_reference_test_values() {
        assert_eq!(fnv1a64(b""), 0xCBF2_9CE4_8422_2325);
        assert_eq!(fnv1a64(b"a"), 0xAF63_DC4C_8601_EC8C);
        assert_eq!(fnv1a64(b"b"), 0xAF63_DF4C_8601_F1A5);
        assert_eq!(fnv1a64(b"foobar"), 0x8594_4171_F739_67E8);
    }

    /// The prime is `2^40 + 2^8 + 0xB3`, and the basis is the FNV-0 hash of
    /// the signature string the FNV authors specify.
    #[test]
    fn the_constants_are_the_specified_ones() {
        assert_eq!(PRIME, (1 << 40) + (1 << 8) + 0xB3);
        let signature = b"chongo <Landon Curt Noll> /\\../\\";
        let mut fnv0 = 0u64;
        for &byte in signature {
            fnv0 = fnv0.wrapping_mul(PRIME) ^ u64::from(byte);
        }
        assert_eq!(fnv0, BASIS);
    }

    /// Folding a split input equals folding it whole, so a caller may feed a
    /// sequence of byte strings; and order matters.
    #[test]
    fn fold_composes_over_a_split_input() {
        let whole = b"http://example.org/resource";
        for split in 0..=whole.len() {
            let (head, tail) = whole.split_at(split);
            assert_eq!(
                fold(fold(BASIS, head), tail),
                fnv1a64(whole),
                "split {split}"
            );
        }
        assert_ne!(fnv1a64(b"ab"), fnv1a64(b"ba"));
        const AT_COMPILE_TIME: u64 = fnv1a64(b"foobar");
        assert_eq!(AT_COMPILE_TIME, 0x8594_4171_F739_67E8);
    }
}
