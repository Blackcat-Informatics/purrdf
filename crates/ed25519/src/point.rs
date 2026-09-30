// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The edwards25519 group: -x² + y² = 1 + d·x²·y² over GF(2^255 - 19)
//! (RFC 8032 section 5.1).
//!
//! Points are held in extended coordinates (X : Y : Z : T) with x = X/Z,
//! y = Y/Z and T = X·Y/Z. Addition and doubling use the Hisil–Wong–Carter–Dawson
//! formulas for a = -1 ("add-2008-hwcd-3", "dbl-2008-hwcd"); because d is not a
//! square in GF(p) the addition law is complete, so the same straight-line code
//! adds any two points, the identity and a point to itself included. The
//! right-hand operand of an addition is in "cached" form (Y+X, Y-X, Z, 2d·T),
//! which is also the form the precomputed tables hold.
//!
//! Two scalar multiplications:
//!
//! * [`Point::mul_base`] — the fixed-base product used while signing, over a
//!   secret scalar. The scalar is recoded into 64 signed radix-16 digits and
//!   each digit selects its multiple from a 32 × 8 table of (j+1)·256^i·B by a
//!   masked scan of the whole row, then a masked conditional negation. The
//!   operation sequence and every memory access are the same for every scalar.
//! * [`Point::double_scalar_mul_vartime`] — k·P + s·B for verification, over
//!   public inputs only, with width-5 non-adjacent forms.

use std::sync::OnceLock;

use crate::ct;
use crate::field::Fe;
use crate::scalar::Scalar;

/// The encoding of the base point B = (x, 4/5) with x even (RFC 8032 5.1).
const BASE_ENCODING: [u8; 32] = [
    0x58, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66,
    0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66,
];

/// A point in extended twisted Edwards coordinates.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Point {
    x: Fe,
    y: Fe,
    z: Fe,
    t: Fe,
}

/// A point prepared as the right-hand operand of an addition.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Cached {
    y_plus_x: Fe,
    y_minus_x: Fe,
    z: Fe,
    t2d: Fe,
}

impl Cached {
    /// The identity (0, 1): Y+X = Y-X = Z = 1, T = 0.
    const IDENTITY: Self = Self {
        y_plus_x: Fe::ONE,
        y_minus_x: Fe::ONE,
        z: Fe::ONE,
        t2d: Fe::ZERO,
    };

    /// `-self`: negating x swaps Y+X with Y-X and negates T.
    fn neg(self) -> Self {
        Self {
            y_plus_x: self.y_minus_x,
            y_minus_x: self.y_plus_x,
            z: self.z,
            t2d: self.t2d.neg(),
        }
    }

    /// `rhs` under an all-ones mask, `self` under zero.
    fn select(self, rhs: &Self, mask: u64) -> Self {
        Self {
            y_plus_x: self.y_plus_x.select(rhs.y_plus_x, mask),
            y_minus_x: self.y_minus_x.select(rhs.y_minus_x, mask),
            z: self.z.select(rhs.z, mask),
            t2d: self.t2d.select(rhs.t2d, mask),
        }
    }
}

/// The fixed-base comb: 32 rows, row i holding (j+1)·256^i·B for j in 0..8.
type Comb = [[Cached; 8]];

fn base_point() -> &'static Point {
    static BASE: OnceLock<Point> = OnceLock::new();
    BASE.get_or_init(|| Point::decode(&BASE_ENCODING).expect("the RFC 8032 base point decodes"))
}

fn base_comb() -> &'static Comb {
    static COMB: OnceLock<Vec<[Cached; 8]>> = OnceLock::new();
    COMB.get_or_init(|| {
        let mut table = vec![[Cached::IDENTITY; 8]; 32];
        let mut row_base = *base_point();
        for row in &mut table {
            let unit = row_base.to_cached();
            let mut multiple = row_base;
            row[0] = unit;
            for slot in row.iter_mut().skip(1) {
                multiple = multiple.add(&unit);
                *slot = multiple.to_cached();
            }
            for _ in 0..8 {
                row_base = row_base.double();
            }
        }
        table
    })
}

fn base_odd_multiples() -> &'static [Cached; 8] {
    static ODD: OnceLock<[Cached; 8]> = OnceLock::new();
    ODD.get_or_init(|| base_point().odd_multiples())
}

/// The entry `digit`·row\[0\] of a comb row, for `digit` in [-8, 8], read by
/// scanning all eight entries under equality masks and negating under the
/// sign mask: the same loads and arithmetic for every digit.
fn select_signed(row: &[Cached; 8], digit: i8) -> Cached {
    let sign = u64::from((digit as u8) >> 7);
    // |digit| without a branch: (d ^ s) - s with s = 0 or -1.
    let s = -(sign as i8);
    let magnitude = u64::from((digit ^ s).wrapping_sub(s) as u8);
    let mut chosen = Cached::IDENTITY;
    for (index, entry) in (1u64..).zip(row) {
        chosen = chosen.select(entry, ct::eq_mask(magnitude, index));
    }
    let negated = chosen.neg();
    chosen.select(&negated, ct::bit_mask(sign))
}

impl Point {
    pub(crate) const IDENTITY: Self = Self {
        x: Fe::ZERO,
        y: Fe::ONE,
        z: Fe::ONE,
        t: Fe::ZERO,
    };

    /// Decode a 32-byte point encoding exactly as RFC 8032 5.1.3 specifies:
    /// the y-coordinate must be canonical (below p), x² = (y² - 1)/(d·y² + 1)
    /// must have a root, and the encoding x = 0 with the sign bit set is
    /// refused. Anything else is `None`.
    pub(crate) fn decode(bytes: &[u8; 32]) -> Option<Self> {
        let sign = bytes[31] >> 7;
        let y = Fe::from_bytes(bytes);
        let mut unsigned = *bytes;
        unsigned[31] &= 0x7f;
        if y.to_bytes() != unsigned {
            return None;
        }
        let yy = y.square();
        let u = yy.sub(Fe::ONE);
        let v = yy.mul(Fe::D).add(Fe::ONE);
        let mut x = Fe::sqrt_ratio(u, v)?;
        if x.is_zero() && sign == 1 {
            return None;
        }
        if u8::from(x.is_negative()) != sign {
            x = x.neg();
        }
        Some(Self {
            x,
            y,
            z: Fe::ONE,
            t: x.mul(y),
        })
    }

    /// The canonical 32-byte encoding (RFC 8032 5.1.2): y little-endian with
    /// the low bit of x in bit 255.
    pub(crate) fn encode(&self) -> [u8; 32] {
        let z_inv = self.z.invert();
        let x = self.x.mul(z_inv);
        let y = self.y.mul(z_inv);
        let mut out = y.to_bytes();
        out[31] |= u8::from(x.is_negative()) << 7;
        out
    }

    fn to_cached(self) -> Cached {
        Cached {
            y_plus_x: self.y.add(self.x),
            y_minus_x: self.y.sub(self.x),
            z: self.z,
            t2d: self.t.mul(Fe::D2),
        }
    }

    /// `self + q` (complete: valid for every pair of points).
    fn add(&self, q: &Cached) -> Self {
        let a = self.y.sub(self.x).mul(q.y_minus_x);
        let b = self.y.add(self.x).mul(q.y_plus_x);
        let c = self.t.mul(q.t2d);
        let zz = self.z.mul(q.z);
        let d = zz.add(zz);
        let e = b.sub(a);
        let f = d.sub(c);
        let g = d.add(c);
        let h = b.add(a);
        Self {
            x: e.mul(f),
            y: g.mul(h),
            z: f.mul(g),
            t: e.mul(h),
        }
    }

    /// `self + other` for two points in extended form.
    #[cfg(test)]
    pub(crate) fn sum(&self, other: &Self) -> Self {
        self.add(&other.to_cached())
    }

    /// `self - q`.
    fn sub(&self, q: &Cached) -> Self {
        self.add(&q.neg())
    }

    /// `2·self`.
    fn double(&self) -> Self {
        let a = self.x.square();
        let b = self.y.square();
        let zz = self.z.square();
        let c = zz.add(zz);
        let sum = a.add(b);
        let e = self.x.add(self.y).square().sub(sum);
        let g = b.sub(a);
        let f = g.sub(c);
        let h = sum.neg();
        Self {
            x: e.mul(f),
            y: g.mul(h),
            z: f.mul(g),
            t: e.mul(h),
        }
    }

    /// `-self`.
    pub(crate) fn neg(&self) -> Self {
        Self {
            x: self.x.neg(),
            y: self.y,
            z: self.z,
            t: self.t.neg(),
        }
    }

    fn is_identity(&self) -> bool {
        self.x.is_zero() && self.y.ct_eq(self.z)
    }

    /// Whether 8·self is the identity: the point lies in the torsion
    /// subgroup of order dividing the cofactor.
    pub(crate) fn is_small_order(&self) -> bool {
        self.double().double().double().is_identity()
    }

    /// P, 3P, 5P, …, 15P, for width-5 non-adjacent forms.
    fn odd_multiples(&self) -> [Cached; 8] {
        let twice = self.double().to_cached();
        let mut out = [Cached::IDENTITY; 8];
        let mut acc = *self;
        out[0] = acc.to_cached();
        for slot in out.iter_mut().skip(1) {
            acc = acc.add(&twice);
            *slot = acc.to_cached();
        }
        out
    }

    /// `scalar · B`, in constant time with respect to `scalar`.
    ///
    /// With digits e_i (i in 0..64) of `scalar` in signed radix 16,
    /// scalar·B = 16·(Σ_k e_(2k+1)·256^k·B) plus Σ_k e_(2k)·256^k·B: 64 table
    /// additions and four doublings, each addend chosen by [`select_signed`].
    pub(crate) fn mul_base(scalar: &Scalar) -> Self {
        let comb = base_comb();
        let mut digits = scalar.radix16();
        let mut acc = Self::IDENTITY;
        for (row, pair) in comb.iter().zip(digits.as_chunks::<2>().0) {
            acc = acc.add(&select_signed(row, pair[1]));
        }
        acc = acc.double().double().double().double();
        for (row, pair) in comb.iter().zip(digits.as_chunks::<2>().0) {
            acc = acc.add(&select_signed(row, pair[0]));
        }
        ct::wipe(&mut digits);
        acc
    }

    /// `k · point + s · B` over public inputs, by interleaved width-5
    /// non-adjacent forms. Variable time: verification only.
    pub(crate) fn double_scalar_mul_vartime(k: &Scalar, point: &Self, s: &Scalar) -> Self {
        let k_naf = k.non_adjacent_form(5);
        let s_naf = s.non_adjacent_form(5);
        let point_table = point.odd_multiples();
        let base_table = base_odd_multiples();
        let Some(top) = (0..256).rev().find(|&i| k_naf[i] != 0 || s_naf[i] != 0) else {
            return Self::IDENTITY;
        };
        let mut acc = Self::IDENTITY;
        for i in (0..=top).rev() {
            acc = acc.double();
            acc = Self::add_digit(acc, &point_table, k_naf[i]);
            acc = Self::add_digit(acc, base_table, s_naf[i]);
        }
        acc
    }

    fn add_digit(acc: Self, table: &[Cached; 8], digit: i8) -> Self {
        match digit {
            0 => acc,
            d if d > 0 => acc.add(&table[(d as usize) / 2]),
            d => acc.sub(&table[usize::from(d.unsigned_abs()) / 2]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{BASE_ENCODING, Point, base_point};
    use crate::scalar::Scalar;
    use purrdf_testkit::rng::SplitMix64;

    fn random_scalar(rng: &mut SplitMix64) -> Scalar {
        let mut wide = [0u8; 64];
        for chunk in wide.chunks_mut(8) {
            chunk.copy_from_slice(&rng.next_u64().to_le_bytes());
        }
        Scalar::from_bytes_wide(&wide)
    }

    fn small_scalar(n: u8) -> Scalar {
        let mut b = [0u8; 32];
        b[0] = n;
        Scalar::from_bytes_mod_order(&b)
    }

    /// n·P by repeated addition: the slow, obviously correct reference.
    fn repeated(point: &Point, n: u32) -> Point {
        let unit = point.to_cached();
        let mut acc = Point::IDENTITY;
        for _ in 0..n {
            acc = acc.add(&unit);
        }
        acc
    }

    #[test]
    fn base_point_round_trips_and_has_prime_order() {
        let b = base_point();
        assert_eq!(b.encode(), BASE_ENCODING);
        assert!(!b.is_small_order());
        // L·B is the identity: (L-1)·B + B.
        let mut l_minus_1 = [0u8; 32];
        l_minus_1.copy_from_slice(&[
            0xec, 0xd3, 0xf5, 0x5c, 0x1a, 0x63, 0x12, 0x58, 0xd6, 0x9c, 0xf7, 0xa2, 0xde, 0xf9,
            0xde, 0x14, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x10,
        ]);
        let s = Scalar::from_canonical_bytes(&l_minus_1).expect("L-1 is canonical");
        let almost = Point::mul_base(&s);
        assert!(almost.add(&b.to_cached()).is_identity());
    }

    #[test]
    fn fixed_base_multiplication_matches_repeated_addition() {
        let b = base_point();
        for n in [0u8, 1, 2, 3, 7, 8, 9, 15, 16, 17, 100, 255] {
            assert_eq!(
                Point::mul_base(&small_scalar(n)).encode(),
                repeated(b, u32::from(n)).encode(),
                "{n}·B"
            );
        }
    }

    #[test]
    fn double_scalar_multiplication_matches_the_fixed_base_path() {
        let mut rng = SplitMix64::new(0x5eed_0004);
        let b = base_point();
        for _ in 0..32 {
            let (k, s, a) = (
                random_scalar(&mut rng),
                random_scalar(&mut rng),
                random_scalar(&mut rng),
            );
            let point = Point::mul_base(&a);
            // k·(a·B) + s·B = (k·a + s)·B.
            let expected = Point::mul_base(&k.mul_add(a, s));
            assert_eq!(
                Point::double_scalar_mul_vartime(&k, &point, &s).encode(),
                expected.encode()
            );
        }
        assert_eq!(
            Point::double_scalar_mul_vartime(&Scalar::ZERO, b, &Scalar::ZERO).encode(),
            Point::IDENTITY.encode()
        );
    }

    #[test]
    fn doubling_agrees_with_self_addition() {
        let mut rng = SplitMix64::new(0x5eed_0005);
        for _ in 0..16 {
            let p = Point::mul_base(&random_scalar(&mut rng));
            assert_eq!(p.double().encode(), p.add(&p.to_cached()).encode());
            assert!(p.add(&p.neg().to_cached()).is_identity());
        }
    }
}
