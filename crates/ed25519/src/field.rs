// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Arithmetic in GF(p), p = 2^255 - 19 (RFC 8032 section 5.1), in radix 2^51.
//!
//! An element is five unsigned 64-bit limbs `l[0] + l[1]·2^51 + … + l[4]·2^204`.
//! Limbs are kept *loosely* reduced: every operation's output has limbs below
//! 2^52, and [`Fe::mul`] accepts inputs whose limbs are below 2^54, so one
//! unreduced [`Fe::add`] may feed a multiplication directly. Products are formed
//! in `u128`; the reduction uses 2^255 ≡ 19 (mod p).
//!
//! Every operation here is straight-line code over the limbs: no branch and no
//! memory index depends on an element's value, so the field layer is constant
//! time for the signing path. Only [`Fe::to_bytes`] fully reduces, and it does
//! so with a carry chain, not a comparison.

use crate::ct;

/// 2^51 - 1: the mask of one limb.
const MASK51: u64 = (1 << 51) - 1;

/// An element of GF(2^255 - 19) in radix 2^51.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Fe(pub(crate) [u64; 5]);

impl Fe {
    pub(crate) const ZERO: Self = Self([0, 0, 0, 0, 0]);
    pub(crate) const ONE: Self = Self([1, 0, 0, 0, 0]);

    /// The twisted Edwards curve constant d = -121665/121666 (RFC 8032 5.1).
    pub(crate) const D: Self = Self([
        0x3_4dca_1359_78a3,
        0x1_a828_3b15_6ebd,
        0x5_e7a2_6001_c029,
        0x7_39c6_63a0_3cbb,
        0x5_2036_cee2_b6ff,
    ]);

    /// 2·d, the factor the extended-coordinate addition law carries on T.
    pub(crate) const D2: Self = Self([
        0x6_9b94_26b2_f159,
        0x3_5050_762a_dd7a,
        0x3_cf44_c003_8052,
        0x6_738c_c740_7977,
        0x2_406d_9dc5_6dff,
    ]);

    /// sqrt(-1) = 2^((p-1)/4), the root RFC 8032 5.1.3 step 3 multiplies by.
    pub(crate) const SQRT_M1: Self = Self([
        0x6_1b27_4a0e_a0b0,
        0x0_d5a5_fc8f_189d,
        0x7_ef5e_9cbd_0c60,
        0x7_8595_a680_4c9e,
        0x2_b832_4804_fc1d,
    ]);

    /// Load a little-endian 255-bit integer, ignoring bit 255 (the sign bit of
    /// a point encoding). The value is NOT reduced: an input in `[p, 2^255)`
    /// loads as itself, which [`Self::to_bytes`] would re-encode canonically.
    pub(crate) fn from_bytes(bytes: &[u8; 32]) -> Self {
        let load = |at: usize| {
            let mut word = [0u8; 8];
            word.copy_from_slice(&bytes[at..at + 8]);
            u64::from_le_bytes(word)
        };
        Self([
            load(0) & MASK51,
            (load(6) >> 3) & MASK51,
            (load(12) >> 6) & MASK51,
            (load(19) >> 1) & MASK51,
            (load(24) >> 12) & MASK51,
        ])
    }

    /// The canonical 32-byte little-endian encoding of the value mod p (bit 255
    /// clear).
    pub(crate) fn to_bytes(self) -> [u8; 32] {
        let mut l = self.carry().0;
        // After `carry` the value is below 2p. Adding 19 overflows 2^255 exactly
        // when the value is at least p; `q` is that overflow bit.
        let mut q = (l[0] + 19) >> 51;
        q = (l[1] + q) >> 51;
        q = (l[2] + q) >> 51;
        q = (l[3] + q) >> 51;
        q = (l[4] + q) >> 51;
        // Subtract q·p by adding 19·q and discarding bit 255.
        l[0] += 19 * q;
        l[1] += l[0] >> 51;
        l[0] &= MASK51;
        l[2] += l[1] >> 51;
        l[1] &= MASK51;
        l[3] += l[2] >> 51;
        l[2] &= MASK51;
        l[4] += l[3] >> 51;
        l[3] &= MASK51;
        l[4] &= MASK51;

        let mut out = [0u8; 32];
        let mut acc: u128 = 0;
        let mut bits = 0u32;
        let mut at = 0;
        for limb in l {
            acc |= u128::from(limb) << bits;
            bits += 51;
            while bits >= 8 {
                out[at] = acc as u8;
                acc >>= 8;
                bits -= 8;
                at += 1;
            }
        }
        // 255 bits: the last seven land in the final byte.
        out[at] = acc as u8;
        out
    }

    /// One parallel carry pass: every limb drops below 2^51 plus a small
    /// carry-in, the top carry folded back as 19·c.
    fn carry(self) -> Self {
        let l = self.0;
        let c = [l[0] >> 51, l[1] >> 51, l[2] >> 51, l[3] >> 51, l[4] >> 51];
        Self([
            (l[0] & MASK51) + 19 * c[4],
            (l[1] & MASK51) + c[0],
            (l[2] & MASK51) + c[1],
            (l[3] & MASK51) + c[2],
            (l[4] & MASK51) + c[3],
        ])
    }

    /// `self + rhs` without reduction (limbs grow by one bit).
    pub(crate) fn add(self, rhs: Self) -> Self {
        let (a, b) = (self.0, rhs.0);
        Self([
            a[0] + b[0],
            a[1] + b[1],
            a[2] + b[2],
            a[3] + b[3],
            a[4] + b[4],
        ])
    }

    /// `self - rhs`, computed as `self + 16p - rhs` so no limb underflows while
    /// `rhs`'s limbs stay below 2^55, then carried.
    pub(crate) fn sub(self, rhs: Self) -> Self {
        const P16_LOW: u64 = 16 * ((1 << 51) - 19);
        const P16_HIGH: u64 = 16 * MASK51;
        let (a, b) = (self.0, rhs.0);
        Self([
            (a[0] + P16_LOW) - b[0],
            (a[1] + P16_HIGH) - b[1],
            (a[2] + P16_HIGH) - b[2],
            (a[3] + P16_HIGH) - b[3],
            (a[4] + P16_HIGH) - b[4],
        ])
        .carry()
    }

    /// `-self`.
    pub(crate) fn neg(self) -> Self {
        Self::ZERO.sub(self)
    }

    /// `self · rhs`: schoolbook in `u128` with the high half folded by 19.
    pub(crate) fn mul(self, rhs: Self) -> Self {
        let a = self.0.map(u128::from);
        let b = rhs.0.map(u128::from);
        let b19 = [0, b[1] * 19, b[2] * 19, b[3] * 19, b[4] * 19];

        let c0 = a[0] * b[0] + a[1] * b19[4] + a[2] * b19[3] + a[3] * b19[2] + a[4] * b19[1];
        let c1 = a[0] * b[1] + a[1] * b[0] + a[2] * b19[4] + a[3] * b19[3] + a[4] * b19[2];
        let c2 = a[0] * b[2] + a[1] * b[1] + a[2] * b[0] + a[3] * b19[4] + a[4] * b19[3];
        let c3 = a[0] * b[3] + a[1] * b[2] + a[2] * b[1] + a[3] * b[0] + a[4] * b19[4];
        let c4 = a[0] * b[4] + a[1] * b[3] + a[2] * b[2] + a[3] * b[1] + a[4] * b[0];

        Self::reduce_wide([c0, c1, c2, c3, c4])
    }

    /// `self²`.
    pub(crate) fn square(self) -> Self {
        let a = self.0.map(u128::from);
        let d0 = a[0] * 2;
        let d1 = a[1] * 2;
        let a3_19 = a[3] * 19;
        let a4_19 = a[4] * 19;

        let c0 = a[0].pow(2) + d1 * a4_19 + 2 * a[2] * a3_19;
        let c1 = d0 * a[1] + 2 * a[2] * a4_19 + a[3] * a3_19;
        let c2 = d0 * a[2] + a[1] * a[1] + 2 * a[3] * a4_19;
        let c3 = d0 * a[3] + d1 * a[2] + a[4] * a4_19;
        let c4 = d0 * a[4] + d1 * a[3] + a[2] * a[2];

        Self::reduce_wide([c0, c1, c2, c3, c4])
    }

    /// Carry five `u128` column sums into loosely reduced limbs.
    fn reduce_wide(c: [u128; 5]) -> Self {
        let mask = u128::from(MASK51);
        let mut l = [0u64; 5];
        let mut carry: u128 = 0;
        for (limb, column) in l.iter_mut().zip(c) {
            let t = column + carry;
            *limb = (t & mask) as u64;
            carry = t >> 51;
        }
        // carry < 2^66: fold it back as 19·carry into limb 0, then one more
        // carry into limb 1 keeps limb 0 below 2^51.
        let t = u128::from(l[0]) + carry * 19;
        l[0] = (t & mask) as u64;
        l[1] += (t >> 51) as u64;
        Self(l)
    }

    /// `self^(2^k)`.
    fn pow2k(self, k: u32) -> Self {
        let mut x = self;
        for _ in 0..k {
            x = x.square();
        }
        x
    }

    /// `(self^(2^250 - 1), self^11)`, the shared prefix of the inversion and
    /// square-root exponents.
    fn pow_2_250_minus_1(self) -> (Self, Self) {
        let z2 = self.square();
        let z9 = z2.pow2k(2).mul(self);
        let z11 = z9.mul(z2);
        let z_5 = z11.square().mul(z9); // 2^5 - 1
        let z_10 = z_5.pow2k(5).mul(z_5); // 2^10 - 1
        let z_20 = z_10.pow2k(10).mul(z_10); // 2^20 - 1
        let z_40 = z_20.pow2k(20).mul(z_20); // 2^40 - 1
        let z_50 = z_40.pow2k(10).mul(z_10); // 2^50 - 1
        let z_100 = z_50.pow2k(50).mul(z_50); // 2^100 - 1
        let z_200 = z_100.pow2k(100).mul(z_100); // 2^200 - 1
        let z_250 = z_200.pow2k(50).mul(z_50); // 2^250 - 1
        (z_250, z11)
    }

    /// `self^(p-2)`, the inverse of a nonzero element (zero maps to zero).
    pub(crate) fn invert(self) -> Self {
        let (z_250, z11) = self.pow_2_250_minus_1();
        // (2^250 - 1)·2^5 + 11 = 2^255 - 21 = p - 2.
        z_250.pow2k(5).mul(z11)
    }

    /// `self^((p-5)/8)`: (2^250 - 1)·2^2 + 1 = 2^252 - 3.
    fn pow_p58(self) -> Self {
        let (z_250, _) = self.pow_2_250_minus_1();
        z_250.pow2k(2).mul(self)
    }

    /// The square root of `u/v` when it exists, per RFC 8032 5.1.3 step 3:
    /// the candidate x = u·v³·(u·v⁷)^((p-5)/8), corrected by sqrt(-1) when
    /// v·x² = -u. `None` when `u/v` is not a square (or `v` is zero while
    /// `u` is not). Used only to decode public points, so it may branch.
    pub(crate) fn sqrt_ratio(u: Self, v: Self) -> Option<Self> {
        let v3 = v.square().mul(v);
        let v7 = v3.square().mul(v);
        let x = u.mul(v3).mul(u.mul(v7).pow_p58());
        let vxx = v.mul(x.square());
        if vxx.ct_eq(u) {
            Some(x)
        } else if vxx.ct_eq(u.neg()) {
            Some(x.mul(Self::SQRT_M1))
        } else {
            None
        }
    }

    /// Whether the canonical encoding's least significant bit is set: the
    /// "negative" x of RFC 8032 5.1.2.
    pub(crate) fn is_negative(self) -> bool {
        self.to_bytes()[0] & 1 == 1
    }

    /// Whether the value is zero mod p.
    pub(crate) fn is_zero(self) -> bool {
        self.ct_eq(Self::ZERO)
    }

    /// Equality mod p over the canonical encodings, compared without an early
    /// exit.
    pub(crate) fn ct_eq(self, rhs: Self) -> bool {
        ct::bytes_eq(&self.to_bytes(), &rhs.to_bytes())
    }

    /// `rhs` when `mask` is all ones, `self` when it is zero; `mask` must be
    /// one of the two.
    pub(crate) fn select(self, rhs: Self, mask: u64) -> Self {
        let (a, b) = (self.0, rhs.0);
        Self([
            a[0] ^ (mask & (a[0] ^ b[0])),
            a[1] ^ (mask & (a[1] ^ b[1])),
            a[2] ^ (mask & (a[2] ^ b[2])),
            a[3] ^ (mask & (a[3] ^ b[3])),
            a[4] ^ (mask & (a[4] ^ b[4])),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::Fe;
    use purrdf_testkit::rng::SplitMix64;

    /// p = 2^255 - 19 in little-endian bytes.
    const P_BYTES: [u8; 32] = {
        let mut p = [0xff; 32];
        p[0] = 0xed;
        p[31] = 0x7f;
        p
    };

    fn small(n: u64) -> Fe {
        Fe([n, 0, 0, 0, 0])
    }

    #[test]
    fn curve_constants_satisfy_their_defining_equations() {
        // d·121666 = -121665.
        assert!(Fe::D.mul(small(121_666)).ct_eq(small(121_665).neg()));
        assert!(Fe::D.add(Fe::D).ct_eq(Fe::D2));
        assert!(Fe::SQRT_M1.square().ct_eq(Fe::ONE.neg()));
    }

    #[test]
    fn encoding_reduces_values_at_and_above_p() {
        assert_eq!(Fe::from_bytes(&P_BYTES).to_bytes(), [0u8; 32]);
        let mut p_plus_5 = P_BYTES;
        p_plus_5[0] += 5;
        assert_eq!(Fe::from_bytes(&p_plus_5).to_bytes(), small(5).to_bytes());
        // p - 1 is canonical and survives the round trip.
        let mut p_minus_1 = P_BYTES;
        p_minus_1[0] -= 1;
        assert_eq!(Fe::from_bytes(&p_minus_1).to_bytes(), p_minus_1);
        // Bit 255 is not part of the value.
        let mut signed = small(9).to_bytes();
        signed[31] |= 0x80;
        assert_eq!(Fe::from_bytes(&signed).to_bytes(), small(9).to_bytes());
    }

    #[test]
    fn inversion_and_square_agree_with_multiplication() {
        let mut rng = SplitMix64::new(0x0123_4567_89ab_cdef);
        for _ in 0..64 {
            let mut bytes = [0u8; 32];
            for chunk in bytes.chunks_mut(8) {
                chunk.copy_from_slice(&rng.next_u64().to_le_bytes());
            }
            let x = Fe::from_bytes(&bytes);
            assert!(x.square().ct_eq(x.mul(x)));
            if !x.is_zero() {
                assert!(x.mul(x.invert()).ct_eq(Fe::ONE));
            }
            // An unreduced sum feeds multiplication correctly.
            let y = x.add(x).add(x);
            assert!(y.mul(y).ct_eq(x.square().mul(small(9))));
            assert!(x.sub(x).is_zero());
            assert!(x.add(x.neg()).is_zero());
        }
    }

    #[test]
    fn square_roots_exist_exactly_for_squares() {
        // 4/1 has root 2 (or -2); -1/1 has root sqrt(-1); 2 is a non-square mod p
        // (p ≡ 5 mod 8).
        let two = Fe::sqrt_ratio(small(4), Fe::ONE).expect("4 is a square");
        assert!(two.square().ct_eq(small(4)));
        let i = Fe::sqrt_ratio(Fe::ONE.neg(), Fe::ONE).expect("-1 is a square");
        assert!(i.square().ct_eq(Fe::ONE.neg()));
        assert!(Fe::sqrt_ratio(small(2), Fe::ONE).is_none());
        assert!(Fe::sqrt_ratio(Fe::ONE, Fe::ZERO).is_none());
    }
}
