// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Integers modulo the prime group order L = 2^252 + 27742317777372353535851937790883648493
//! (RFC 8032 section 5.1), in radix 2^52 with Montgomery multiplication.
//!
//! A [`Scalar`] is five 52-bit limbs holding a value already reduced below L.
//! Multiplication uses Montgomery reduction with R = 2^260: the product of two
//! reduced values is below L·R, so one reduction pass lands below 2L and a
//! masked subtraction finishes it. Reducing a 512-bit SHA-512 output splits it
//! at bit 260 and recombines the halves through the constants R mod L and
//! R² mod L. Every path is straight-line limb arithmetic with masked
//! corrections: the nonce and the secret scalar pass through here while
//! signing, so nothing branches on or indexes by a value.

use crate::ct;

/// 2^52 - 1: the mask of one limb.
const MASK52: u64 = (1 << 52) - 1;

/// L in radix 2^52.
const L: Scalar = Scalar([
    0x2_631a_5cf5_d3ed,
    0xd_ea2f_79cd_6581,
    0x0_0000_0014_def9,
    0,
    0x0_1000_0000_0000,
]);

/// -L⁻¹ mod 2^52: the Montgomery quotient factor.
const LFACTOR: u64 = 0x5_1da3_1254_7e1b;

/// R mod L = 2^260 mod L.
const R: Scalar = Scalar([
    0xf_48bd_6721_e6ed,
    0x3_bab5_ac67_e45a,
    0xf_ffff_eb35_e51b,
    0xf_ffff_ffff_ffff,
    0x0_0fff_ffff_ffff,
]);

/// R² mod L = 2^520 mod L.
const RR: Scalar = Scalar([
    0x9_d265_e952_d13b,
    0xd_63c7_15be_a69f,
    0x5_be65_cb68_7604,
    0x3_dcee_c73d_217f,
    0x0_0941_1b7c_309a,
]);

/// An integer mod L, reduced, in radix 2^52.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Scalar(pub(crate) [u64; 5]);

/// The 52-bit limb starting at bit `52·index` of the little-endian words.
fn limb_at(words: &[u64], index: usize) -> u64 {
    let start = 52 * index;
    let (word, shift) = (start / 64, start % 64);
    let mut value = words[word] >> shift;
    if shift > 12 && word + 1 < words.len() {
        value |= words[word + 1] << (64 - shift);
    }
    value & MASK52
}

fn words<const N: usize, const W: usize>(bytes: &[u8; N]) -> [u64; W] {
    let mut out = [0u64; W];
    for (word, chunk) in out.iter_mut().zip(bytes.as_chunks::<8>().0) {
        *word = u64::from_le_bytes(*chunk);
    }
    out
}

impl Scalar {
    #[cfg(test)]
    pub(crate) const ZERO: Self = Self([0; 5]);

    /// The 512-bit little-endian integer `bytes` reduced mod L: how RFC 8032
    /// 5.1.6 and 5.1.7 read the SHA-512 outputs r and k.
    pub(crate) fn from_bytes_wide(bytes: &[u8; 64]) -> Self {
        let w: [u64; 8] = words(bytes);
        let lo = Self([
            limb_at(&w, 0),
            limb_at(&w, 1),
            limb_at(&w, 2),
            limb_at(&w, 3),
            limb_at(&w, 4),
        ]);
        let hi = Self([
            limb_at(&w, 5),
            limb_at(&w, 6),
            limb_at(&w, 7),
            limb_at(&w, 8),
            limb_at(&w, 9),
        ]);
        // lo·R/R + hi·R²/R = lo + hi·2^260 (mod L).
        Self::montgomery_mul(lo, R).add(Self::montgomery_mul(hi, RR))
    }

    /// The 256-bit little-endian integer `bytes` reduced mod L.
    pub(crate) fn from_bytes_mod_order(bytes: &[u8; 32]) -> Self {
        let mut wide = [0u8; 64];
        wide[..32].copy_from_slice(bytes);
        let scalar = Self::from_bytes_wide(&wide);
        ct::wipe(&mut wide);
        scalar
    }

    /// `bytes` as a scalar when the integer it encodes is below L, and `None`
    /// otherwise: the canonical-S test of RFC 8032 5.1.7 step 1.
    pub(crate) fn from_canonical_bytes(bytes: &[u8; 32]) -> Option<Self> {
        let w: [u64; 4] = words(bytes);
        // Five 52-bit limbs span 260 bits, so all 256 input bits are loaded.
        let candidate = Self([
            limb_at(&w, 0),
            limb_at(&w, 1),
            limb_at(&w, 2),
            limb_at(&w, 3),
            limb_at(&w, 4),
        ]);
        // candidate - L borrows exactly when candidate < L.
        let (_, borrow) = candidate.sub_raw(L);
        (borrow == 1).then_some(candidate)
    }

    /// The canonical 32-byte little-endian encoding.
    pub(crate) fn to_bytes(self) -> [u8; 32] {
        let l = self.0;
        let w = [
            l[0] | (l[1] << 52),
            (l[1] >> 12) | (l[2] << 40),
            (l[2] >> 24) | (l[3] << 28),
            (l[3] >> 36) | (l[4] << 16),
        ];
        let mut out = [0u8; 32];
        for (chunk, word) in out.as_chunks_mut::<8>().0.iter_mut().zip(w) {
            chunk.copy_from_slice(&word.to_le_bytes());
        }
        out
    }

    /// `self - rhs` over limbs, with the final borrow (1 when `self < rhs`).
    fn sub_raw(self, rhs: Self) -> (Self, u64) {
        let mut out = [0u64; 5];
        let mut borrow = 0u64;
        for (slot, (a, b)) in out.iter_mut().zip(self.0.into_iter().zip(rhs.0)) {
            let diff = a.wrapping_sub(b).wrapping_sub(borrow);
            borrow = diff >> 63;
            *slot = diff & MASK52;
        }
        (Self(out), borrow)
    }

    /// `self - rhs mod L` for reduced inputs: subtract, then add L back under
    /// the borrow mask.
    fn sub(self, rhs: Self) -> Self {
        let (diff, borrow) = self.sub_raw(rhs);
        let mask = ct::bit_mask(borrow);
        let mut out = [0u64; 5];
        let mut carry = 0u64;
        for (slot, (d, l)) in out.iter_mut().zip(diff.0.into_iter().zip(L.0)) {
            let sum = d + (l & mask) + carry;
            carry = sum >> 52;
            *slot = sum & MASK52;
        }
        Self(out)
    }

    /// `self + rhs mod L` for inputs below L.
    pub(crate) fn add(self, rhs: Self) -> Self {
        let mut sum = [0u64; 5];
        let mut carry = 0u64;
        for (slot, (a, b)) in sum.iter_mut().zip(self.0.into_iter().zip(rhs.0)) {
            let t = a + b + carry;
            carry = t >> 52;
            *slot = t & MASK52;
        }
        // The sum is below 2L < 2^254, so it fits five limbs with no carry out.
        Self(sum).sub(L)
    }

    /// `self · rhs mod L`.
    pub(crate) fn mul(self, rhs: Self) -> Self {
        // (a·b/R)·R²/R = a·b.
        Self::montgomery_mul(Self::montgomery_mul(self, rhs), RR)
    }

    /// `self · rhs + addend mod L`: the S = r + k·s of RFC 8032 5.1.6 step 5.
    pub(crate) fn mul_add(self, rhs: Self, addend: Self) -> Self {
        self.mul(rhs).add(addend)
    }

    /// `a · b / R mod L`, for `a · b < L · R`.
    fn montgomery_mul(a: Self, b: Self) -> Self {
        let (a, b) = (a.0.map(u128::from), b.0.map(u128::from));
        let mut t = [0u128; 9];
        for i in 0..5 {
            for j in 0..5 {
                t[i + j] += a[i] * b[j];
            }
        }
        Self::montgomery_reduce(t)
    }

    /// `t / R mod L` for a nine-column product `t < L · R`.
    fn montgomery_reduce(mut t: [u128; 9]) -> Self {
        let l = L.0.map(u128::from);
        let mut carry: u128 = 0;
        for i in 0..5 {
            let ti = t[i] + carry;
            // m makes ti + m·L divisible by 2^52.
            let m = u128::from((ti as u64).wrapping_mul(LFACTOR) & MASK52);
            carry = (ti + m * l[0]) >> 52;
            for j in 1..5 {
                t[i + j] += m * l[j];
            }
        }
        let mut out = [0u64; 5];
        for (slot, column) in out.iter_mut().take(4).zip(&t[5..9]) {
            let ti = column + carry;
            *slot = (ti as u64) & MASK52;
            carry = ti >> 52;
        }
        out[4] = carry as u64;
        // (t + m·L) / R < 2L: one masked subtraction reduces it.
        Self(out).sub(L)
    }

    /// The 64 signed radix-16 digits e_i in [-8, 8) with self = Σ e_i·16^i
    /// (the last digit may be 8), for the fixed-base comb. Computed with
    /// arithmetic shifts only, so the digits of a secret scalar are produced
    /// without a branch.
    pub(crate) fn radix16(self) -> [i8; 64] {
        let bytes = self.to_bytes();
        let mut digits = [0i8; 64];
        for (pair, byte) in digits.as_chunks_mut::<2>().0.iter_mut().zip(bytes) {
            pair[0] = (byte & 15) as i8;
            pair[1] = (byte >> 4) as i8;
        }
        let mut carry = 0i8;
        for digit in digits.iter_mut().take(63) {
            let d = *digit + carry;
            carry = (d + 8) >> 4;
            *digit = d - (carry << 4);
        }
        digits[63] += carry;
        digits
    }

    /// The width-`w` non-adjacent form: 256 digits, each zero or odd with
    /// |digit| < 2^(w-1), no two nonzero digits within w positions. Used only
    /// on public scalars (verification), so it branches freely.
    pub(crate) fn non_adjacent_form(self, w: usize) -> [i8; 256] {
        let b = self.to_bytes();
        let mut x = [0u64; 5];
        x[..4].copy_from_slice(&words::<32, 4>(&b));
        let width = 1u64 << w;
        let window_mask = width - 1;
        let mut naf = [0i8; 256];
        let mut pos = 0;
        let mut carry = 0u64;
        while pos < 256 {
            let (index, shift) = (pos / 64, pos % 64);
            let bits = if shift < 64 - w {
                x[index] >> shift
            } else {
                (x[index] >> shift) | (x[index + 1] << (64 - shift))
            };
            let window = carry + (bits & window_mask);
            if window & 1 == 0 {
                // An even window emits a zero here and defers the carry.
                pos += 1;
                continue;
            }
            if window < width / 2 {
                carry = 0;
                naf[pos] = window as i8;
            } else {
                carry = 1;
                naf[pos] = (window as i8).wrapping_sub(width as i8);
            }
            pos += w;
        }
        naf
    }

    /// Overwrite the limbs with zero.
    pub(crate) fn wipe(&mut self) {
        ct::wipe(&mut self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::{L, R, RR, Scalar};
    use purrdf_testkit::rng::SplitMix64;

    /// L in little-endian bytes.
    const L_BYTES: [u8; 32] = [
        0xed, 0xd3, 0xf5, 0x5c, 0x1a, 0x63, 0x12, 0x58, 0xd6, 0x9c, 0xf7, 0xa2, 0xde, 0xf9, 0xde,
        0x14, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x10,
    ];

    /// An independent reduction: bit-serial double-and-subtract over four
    /// 64-bit words, most significant bit first.
    fn reference_mod_l(bytes: &[u8]) -> [u8; 32] {
        let l = words_of(&L_BYTES);
        let mut r = [0u64; 4];
        for byte in bytes.iter().rev() {
            for bit in (0..8).rev() {
                // r = 2r + bit (r < L < 2^253, so no overflow).
                let mut carry = u64::from((byte >> bit) & 1);
                for word in &mut r {
                    let next = *word >> 63;
                    *word = (*word << 1) | carry;
                    carry = next;
                }
                if !less_than(&r, &l) {
                    let mut borrow = 0u64;
                    for (word, lw) in r.iter_mut().zip(l) {
                        let (d1, b1) = word.overflowing_sub(lw);
                        let (d2, b2) = d1.overflowing_sub(borrow);
                        *word = d2;
                        borrow = u64::from(b1 | b2);
                    }
                }
            }
        }
        let mut out = [0u8; 32];
        for (chunk, word) in out.as_chunks_mut::<8>().0.iter_mut().zip(r) {
            chunk.copy_from_slice(&word.to_le_bytes());
        }
        out
    }

    fn words_of(bytes: &[u8; 32]) -> [u64; 4] {
        super::words(bytes)
    }

    fn less_than(a: &[u64; 4], b: &[u64; 4]) -> bool {
        for i in (0..4).rev() {
            if a[i] != b[i] {
                return a[i] < b[i];
            }
        }
        false
    }

    fn random_wide(rng: &mut SplitMix64) -> [u8; 64] {
        let mut out = [0u8; 64];
        for chunk in out.chunks_mut(8) {
            chunk.copy_from_slice(&rng.next_u64().to_le_bytes());
        }
        out
    }

    #[test]
    fn montgomery_constants_match_their_definitions() {
        assert_eq!(L.to_bytes(), L_BYTES);
        let mut two_260 = [0u8; 33];
        two_260[32] = 0x10;
        assert_eq!(R.to_bytes(), reference_mod_l(&two_260));
        let mut two_520 = [0u8; 66];
        two_520[65] = 0x01;
        assert_eq!(RR.to_bytes(), reference_mod_l(&two_520));
    }

    #[test]
    fn wide_reduction_matches_the_bit_serial_reference() {
        let mut rng = SplitMix64::new(0x5eed_0001);
        let mut cases = vec![[0u8; 64], [0xffu8; 64]];
        let mut l_wide = [0u8; 64];
        l_wide[..32].copy_from_slice(&L_BYTES);
        cases.push(l_wide);
        for _ in 0..256 {
            cases.push(random_wide(&mut rng));
        }
        for wide in cases {
            assert_eq!(
                Scalar::from_bytes_wide(&wide).to_bytes(),
                reference_mod_l(&wide)
            );
        }
    }

    #[test]
    fn multiply_add_matches_the_reference() {
        let mut rng = SplitMix64::new(0x5eed_0002);
        for _ in 0..128 {
            let a = Scalar::from_bytes_wide(&random_wide(&mut rng));
            let b = Scalar::from_bytes_wide(&random_wide(&mut rng));
            let c = Scalar::from_bytes_wide(&random_wide(&mut rng));
            // Reference: the 512-bit product a·b plus c, reduced bit-serially.
            let product = wide_product(&a.to_bytes(), &b.to_bytes(), &c.to_bytes());
            assert_eq!(a.mul_add(b, c).to_bytes(), reference_mod_l(&product));
        }
    }

    /// a·b + c as a 64-byte little-endian integer, by schoolbook on bytes.
    fn wide_product(a: &[u8; 32], b: &[u8; 32], c: &[u8; 32]) -> [u8; 64] {
        let mut acc = [0u32; 65];
        for (i, &x) in a.iter().enumerate() {
            for (j, &y) in b.iter().enumerate() {
                acc[i + j] += u32::from(x) * u32::from(y);
            }
        }
        for (i, &z) in c.iter().enumerate() {
            acc[i] += u32::from(z);
        }
        let mut out = [0u8; 64];
        let mut carry = 0u32;
        for (slot, column) in out.iter_mut().zip(acc) {
            let t = column + carry;
            *slot = t as u8;
            carry = t >> 8;
        }
        out
    }

    #[test]
    fn canonical_scalars_stop_exactly_at_l() {
        assert!(Scalar::from_canonical_bytes(&L_BYTES).is_none());
        let mut below = L_BYTES;
        below[0] -= 1;
        assert_eq!(
            Scalar::from_canonical_bytes(&below).map(Scalar::to_bytes),
            Some(below)
        );
        assert!(Scalar::from_canonical_bytes(&[0xff; 32]).is_none());
        // 2^255 and above: only the high bits set.
        let mut high = [0u8; 32];
        high[31] = 0x80;
        assert!(Scalar::from_canonical_bytes(&high).is_none());
        assert!(Scalar::from_canonical_bytes(&[0; 32]).is_some());
    }

    #[test]
    fn signed_digits_recompose_the_scalar() {
        let mut rng = SplitMix64::new(0x5eed_0003);
        for _ in 0..64 {
            let s = Scalar::from_bytes_wide(&random_wide(&mut rng));
            let digits = s.radix16();
            assert!(digits.iter().all(|&d| (-8..=8).contains(&d)));
            assert_eq!(
                recompose(digits.iter().map(|&d| i64::from(d)), 4),
                s.to_bytes()
            );
            let naf = s.non_adjacent_form(5);
            assert!(
                naf.iter()
                    .all(|&d| d == 0 || (d % 2 != 0 && (-16..16).contains(&d)))
            );
            assert_eq!(
                recompose(naf.iter().map(|&d| i64::from(d)), 1),
                s.to_bytes()
            );
        }
    }

    /// Σ digit_i · 2^(shift·i) mod L, through the scalar arithmetic under test
    /// checked separately above.
    fn recompose(digits: impl DoubleEndedIterator<Item = i64>, shift: u32) -> [u8; 32] {
        let base = {
            let mut b = [0u8; 32];
            b[0] = 1 << shift;
            Scalar::from_bytes_mod_order(&b)
        };
        let mut acc = Scalar::ZERO;
        for d in digits.rev() {
            acc = acc.mul(base);
            let mut magnitude = [0u8; 32];
            magnitude[0] = d.unsigned_abs() as u8;
            let m = Scalar::from_bytes_mod_order(&magnitude);
            acc = if d < 0 { acc.sub(m) } else { acc.add(m) };
        }
        acc.to_bytes()
    }
}
