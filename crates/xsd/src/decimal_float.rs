// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The correctly rounded `xsd:decimal → xsd:double` / `xsd:float` conversion
//! (XPath F&O §19.1.2 casting, and SPARQL's `decimal ⊂ float ⊂ double`
//! promotion): the exact rational `mantissa / 10^scale` rounded ONCE, to
//! nearest with ties to even, straight to the target width.
//!
//! # Algorithm
//!
//! * **Exact fast paths.** A zero mantissa is `+0.0`. At scale 0 the value is the
//!   integer itself, and Rust's `i128 → f64`/`f32` casts are correctly rounded.
//!   When `|mantissa| < 2^53` and `scale ≤ 22` (`2^24` and `10` for `f32`), both
//!   `mantissa` and `10^scale` are exact in the target type, so one IEEE division
//!   is the one correctly rounded operation (Clinger's fast path). That covers
//!   every decimal of at most 15 significant digits.
//! * **The exact path**, for everything else. With `u = |mantissa|` and
//!   `d = 10^scale` (a `u128` for `scale ≤ 38`), exact integer division yields the
//!   top 64 significant bits of `u / d` — the integer quotient, then 64-bit
//!   chunks of the binary fraction while `d < 2^64` (one `u128` division each;
//!   `scale ≤ 19`, which includes every scale this crate constructs), bit by bit
//!   beyond — with a sticky bit OR-ed into the lowest bit for any nonzero
//!   remainder. That bit lies at least 11 places below any rounding position, so
//!   it only breaks ties, never moves one. [`round_top64`] then rounds those 64
//!   bits once to the target's precision, subnormal precision included.
//! * **Scales above 38** have no `u128` power of ten. No construction in this
//!   crate produces one (the scale is capped at 18), so the value is handed to
//!   Rust's decimal-to-float parser, which is itself correctly rounded; this
//!   keeps the function total without a panic path.
//!
//! The rounding never goes through `f64` on the way to `f32`: narrowing a
//! rounded `f64` rounds twice, and differs from the correctly rounded `f32`
//! whenever the first rounding lands exactly on an `f32` halfway point.

/// `10^k` for `k ≤ 22`, every one exact in `f64` (`5^22 < 2^53`).
const POW10_F64: [f64; 23] = [
    1e0, 1e1, 1e2, 1e3, 1e4, 1e5, 1e6, 1e7, 1e8, 1e9, 1e10, 1e11, 1e12, 1e13, 1e14, 1e15, 1e16,
    1e17, 1e18, 1e19, 1e20, 1e21, 1e22,
];

/// `10^k` for `k ≤ 10`, every one exact in `f32` (`5^10 < 2^24`).
const POW10_F32: [f32; 11] = [1e0, 1e1, 1e2, 1e3, 1e4, 1e5, 1e6, 1e7, 1e8, 1e9, 1e10];

/// An IEEE binary interchange format, as far as rounding needs it.
#[derive(Clone, Copy)]
struct Format {
    /// Stored fraction bits (52 for binary64, 23 for binary32).
    fraction_bits: u32,
    /// The exponent bias (1023 / 127).
    bias: i32,
    /// The all-ones biased exponent (2047 / 255): infinity.
    max_biased: i32,
}

const BINARY64: Format = Format {
    fraction_bits: 52,
    bias: 1023,
    max_biased: 2047,
};

const BINARY32: Format = Format {
    fraction_bits: 23,
    bias: 127,
    max_biased: 255,
};

/// The correctly rounded `f64` of `mantissa / 10^scale`.
pub(crate) fn decimal_to_f64(mantissa: i128, scale: u8) -> f64 {
    if mantissa == 0 {
        return 0.0;
    }
    if scale == 0 {
        return mantissa as f64;
    }
    let magnitude = mantissa.unsigned_abs();
    if magnitude < 1 << 53 && usize::from(scale) < POW10_F64.len() {
        // Both operands exact (`magnitude < 2^53`), so the division rounds once --
        // on every target, the x87 included (`crate::ieee`).
        return crate::ieee::f64_div(mantissa as f64, POW10_F64[usize::from(scale)]);
    }
    let Some(bits) = exact_bits(magnitude, scale, BINARY64) else {
        return parse_fallback::<f64>(mantissa, scale);
    };
    let value = f64::from_bits(bits);
    if mantissa < 0 { -value } else { value }
}

/// The correctly rounded `f32` of `mantissa / 10^scale` — rounded once, never
/// through `f64`.
pub(crate) fn decimal_to_f32(mantissa: i128, scale: u8) -> f32 {
    if mantissa == 0 {
        return 0.0;
    }
    if scale == 0 {
        return mantissa as f32;
    }
    let magnitude = mantissa.unsigned_abs();
    if magnitude < 1 << 24 && usize::from(scale) < POW10_F32.len() {
        // Both operands exact (`magnitude < 2^24`), so the division rounds once.
        return crate::ieee::f32_div(mantissa as f32, POW10_F32[usize::from(scale)]);
    }
    let Some(bits) = exact_bits(magnitude, scale, BINARY32) else {
        return parse_fallback::<f32>(mantissa, scale);
    };
    // `exact_bits` for binary32 fits the low 32 bits by construction.
    let value = f32::from_bits(u32::try_from(bits).unwrap_or(0x7F80_0000));
    if mantissa < 0 { -value } else { value }
}

/// The correctly rounded conversion by Rust's decimal-to-float parser, for a
/// scale past `u128`'s powers of ten (see the module docs: unreachable from any
/// decimal this crate constructs).
fn parse_fallback<F: std::str::FromStr + Default>(mantissa: i128, scale: u8) -> F {
    format!("{mantissa}e-{scale}")
        .parse::<F>()
        .unwrap_or_default()
}

/// The magnitude bits of `magnitude / 10^scale` (nonzero `magnitude`,
/// `scale ≥ 1`) in `format`, or `None` when `10^scale` exceeds `u128`.
fn exact_bits(magnitude: u128, scale: u8, format: Format) -> Option<u64> {
    let divisor = 10u128.checked_pow(u32::from(scale))?;
    let (top, exponent) = top64_of_quotient(magnitude, divisor);
    Some(round_top64(top, exponent, format))
}

/// The top 64 significant bits of `numerator / divisor` (both nonzero), as
/// `(top, exponent)` with `numerator / divisor ≈ top × 2^exponent`: `top` has
/// bit 63 set, and its bit 0 is OR-ed with "the exact quotient has a nonzero
/// bit below these 64".
fn top64_of_quotient(numerator: u128, divisor: u128) -> (u64, i32) {
    let quotient = numerator / divisor;
    let remainder = numerator % divisor;
    if quotient != 0 {
        let length = quotient.bit_width();
        if length >= 64 {
            let drop = length - 64;
            // `quotient >> drop` has exactly 64 bits.
            let top = (quotient >> drop) as u64;
            let sticky = (drop > 0 && quotient & ((1u128 << drop) - 1) != 0) || remainder != 0;
            // `drop ≤ 64`, so the cast is exact.
            return (top | u64::from(sticky), drop as i32);
        }
        let needed = 64 - length;
        let (fraction, rest) = fraction_bits(remainder, divisor, needed);
        // `quotient < 2^length`, so the shift keeps it inside 64 bits.
        let top = ((quotient as u64) << needed) | fraction;
        return (top | u64::from(rest != 0), -(needed as i32));
    }
    // `numerator < divisor`: skip the leading zero fraction bits first. With
    // `t = bitlen(divisor) − bitlen(numerator)`, `numerator × 2^t` has the
    // divisor's bit length, so the leading one bit of the quotient sits at
    // fraction position `t` (if `numerator × 2^t ≥ divisor`) or `t + 1`.
    // `divisor ≤ 10^38 < 2^127`, so `numerator × 2^(t+1) < 2^128`.
    let t = numerator.leading_zeros() - divisor.leading_zeros();
    let mut zeros = t;
    let mut scaled = numerator << t;
    if scaled < divisor {
        scaled <<= 1;
        zeros += 1;
    }
    // Now `1 ≤ scaled / divisor < 2`: the leading one, then 63 more bits.
    let (fraction, rest) = fraction_bits(scaled - divisor, divisor, 63);
    let top = (1u64 << 63) | fraction;
    // `zeros ≤ 127`, so the cast is exact.
    (top | u64::from(rest != 0), -(zeros as i32) - 63)
}

/// The next `count` (≤ 64) binary fraction bits of `remainder / divisor`
/// (`remainder < divisor`), and the remainder after them.
fn fraction_bits(remainder: u128, divisor: u128, count: u32) -> (u64, u128) {
    debug_assert!(count <= 64 && remainder < divisor);
    if count == 0 {
        return (0, remainder);
    }
    if divisor <= 1 << 64 {
        // `remainder < 2^64`, so `remainder << count < 2^128`: one exact division.
        let shifted = remainder << count;
        // The quotient is `< 2^count ≤ 2^64`.
        return ((shifted / divisor) as u64, shifted % divisor);
    }
    // `divisor < 2^127`, so doubling a remainder below it stays inside `u128`.
    let mut bits = 0u64;
    let mut rest = remainder;
    for _ in 0..count {
        rest <<= 1;
        bits <<= 1;
        if rest >= divisor {
            rest -= divisor;
            bits |= 1;
        }
    }
    (bits, rest)
}

/// Round `top × 2^exponent` (`top`'s bit 63 set, its bit 0 carrying the sticky
/// bit) once, to nearest with ties to even, into `format`'s magnitude bits —
/// normal, subnormal, `+0` on underflow and `+∞` on overflow.
fn round_top64(top: u64, exponent: i32, format: Format) -> u64 {
    // `2^unbiased ≤ value < 2^(unbiased + 1)`.
    let unbiased = exponent + 63;
    let min_normal = 1 - format.bias;
    // Significand bits available: `fraction_bits + 1` when normal, fewer below.
    let precision = if unbiased >= min_normal {
        format.fraction_bits as i32 + 1
    } else {
        format.fraction_bits as i32 + 1 - (min_normal - unbiased)
    };
    if precision < 0 {
        // Below half the smallest subnormal.
        return 0;
    }
    // `precision ≤ 53`, so `drop ≥ 11`: the sticky bit 0 is never the half bit.
    let drop = (64 - precision) as u32;
    let (kept, rest) = if drop >= 64 {
        (0, top)
    } else {
        (top >> drop, top & ((1u64 << drop) - 1))
    };
    let half = 1u64 << (drop - 1);
    let rounded = if rest > half || (rest == half && kept & 1 == 1) {
        kept + 1
    } else {
        kept
    };
    if unbiased < min_normal {
        // Subnormal: the bits are the significand itself; a carry to
        // `2^fraction_bits` is exactly the smallest normal's encoding.
        return rounded;
    }
    // Normal: `rounded` includes the hidden bit, so adding it onto
    // `biased − 1` in the exponent field carries a rounding overflow
    // (`rounded = 2^(fraction_bits+1)`) into the exponent by itself.
    let biased = unbiased + format.bias;
    if biased >= format.max_biased {
        return (format.max_biased as u64) << format.fraction_bits;
    }
    (((biased - 1) as u64) << format.fraction_bits) + rounded
}

#[cfg(test)]
mod tests {
    use super::{decimal_to_f32, decimal_to_f64};

    /// A deterministic SplitMix64 stream (this zero-dependency crate carries no RNG).
    fn splitmix64(state: &mut u64) -> u64 {
        *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = *state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// The independent oracle: Rust's decimal-to-float parser is correctly
    /// rounded, and parsing the exact decimal shares no code with the
    /// conversion under test.
    fn oracle_f64(mantissa: i128, scale: u8) -> f64 {
        format!("{mantissa}e-{scale}")
            .parse::<f64>()
            .expect("decimal parses")
    }

    fn oracle_f32(mantissa: i128, scale: u8) -> f32 {
        format!("{mantissa}e-{scale}")
            .parse::<f32>()
            .expect("decimal parses")
    }

    fn assert_matches_oracle(mantissa: i128, scale: u8) {
        assert_eq!(
            decimal_to_f64(mantissa, scale).to_bits(),
            oracle_f64(mantissa, scale).to_bits(),
            "f64 of {mantissa}e-{scale}"
        );
        assert_eq!(
            decimal_to_f32(mantissa, scale).to_bits(),
            oracle_f32(mantissa, scale).to_bits(),
            "f32 of {mantissa}e-{scale}"
        );
    }

    /// A random `i128` of exactly `bits` significant bits (`1 ≤ bits ≤ 127`),
    /// random sign.
    fn random_mantissa(state: &mut u64, bits: u32) -> i128 {
        let raw = (u128::from(splitmix64(state)) << 64) | u128::from(splitmix64(state));
        let mut magnitude = if bits >= 128 {
            raw
        } else {
            raw & ((1u128 << bits) - 1)
        };
        magnitude |= 1u128 << (bits - 1);
        // Sometimes clear the low bits so sparse mantissas are drawn too.
        if splitmix64(state).is_multiple_of(8) && bits > 8 {
            magnitude &= !((1u128 << (bits - 8)) - 1);
        }
        let value = i128::try_from(magnitude).expect("bits ≤ 127");
        if splitmix64(state).is_multiple_of(2) {
            -value
        } else {
            value
        }
    }

    /// The conversion this crate shipped before: two roundings (the cast, then
    /// the division by a possibly inexact `10^scale`).
    fn old_to_f64(mantissa: i128, scale: u8) -> f64 {
        mantissa as f64 / 10f64.powi(i32::from(scale))
    }

    #[test]
    fn random_decimals_match_the_oracle_across_mantissa_sizes_and_scales() {
        let mut state = 0x0DEC_1F64_u64;
        for bits in 1..=127_u32 {
            for scale in 0..=38_u8 {
                for _ in 0..4 {
                    assert_matches_oracle(random_mantissa(&mut state, bits), scale);
                }
            }
        }
        for scale in 0..=38_u8 {
            for mantissa in [i128::MAX, i128::MIN, i128::MIN + 1, 1, -1, 0] {
                assert_matches_oracle(mantissa, scale);
            }
        }
    }

    #[test]
    fn zero_is_positive_zero_at_every_scale() {
        for scale in 0..=38_u8 {
            assert_eq!(decimal_to_f64(0, scale).to_bits(), 0);
            assert_eq!(decimal_to_f32(0, scale).to_bits(), 0);
        }
    }

    /// Decimals whose exact value is a binary tie at 53 (or 24) bits, and the
    /// decimals one mantissa unit either side: the tie `t = (2^53 + 2j + 1) × 2^k`
    /// is written at scale `s` as the mantissa `t × 10^s`, so the ties are
    /// exercised through the scaled (division) path as well as at scale 0.
    #[test]
    fn exact_ties_and_their_neighbours_round_to_even() {
        for significand in [(1u128 << 53) + 1, (1u128 << 53) + 3, (1u128 << 54) - 1] {
            for shift in 0..=60_u32 {
                let tie = significand << shift;
                for scale in 0..=10_u8 {
                    // `tie × 10^scale / 10^scale` is exactly the tie.
                    let Some(mantissa) = i128::try_from(tie)
                        .ok()
                        .and_then(|t| t.checked_mul(10i128.pow(u32::from(scale))))
                    else {
                        continue;
                    };
                    for offset in [-1_i128, 0, 1] {
                        assert_matches_oracle(mantissa + offset, scale);
                        assert_matches_oracle(-(mantissa + offset), scale);
                    }
                }
            }
        }
        for significand in [(1u128 << 24) + 1, (1u128 << 24) + 3, (1u128 << 25) - 1] {
            for shift in 0..=90_u32 {
                let tie = significand << shift;
                for scale in 0..=10_u8 {
                    let Some(mantissa) = i128::try_from(tie)
                        .ok()
                        .and_then(|t| t.checked_mul(10i128.pow(u32::from(scale))))
                    else {
                        continue;
                    };
                    for offset in [-1_i128, 0, 1] {
                        assert_matches_oracle(mantissa + offset, scale);
                    }
                }
            }
        }
        // A dyadic tie below one, through the fraction-bit path:
        // `(2^24 + 1) / 2^25 = 5^25 (2^24 + 1) / 10^25`.
        let mantissa = 5i128.pow(25) * ((1 << 24) + 1);
        for offset in [-1_i128, 0, 1] {
            assert_matches_oracle(mantissa + offset, 25);
        }
        assert_eq!(
            decimal_to_f32(mantissa, 25).to_bits(),
            0.5f32.to_bits(),
            "the tie rounds to the even 2^-1"
        );
    }

    /// `f32` subnormals: scale 38 reaches below `f32::MIN_POSITIVE ≈ 1.18e-38`.
    #[test]
    fn f32_subnormals_round_once() {
        for mantissa in 1..=2_000_i128 {
            assert_matches_oracle(mantissa, 38);
            assert_matches_oracle(-mantissa, 38);
        }
        let mut state = 0x05AB_0F32_u64;
        for bits in 1..=40_u32 {
            for _ in 0..64 {
                assert_matches_oracle(random_mantissa(&mut state, bits), 38);
                assert_matches_oracle(random_mantissa(&mut state, bits), 37);
            }
        }
    }

    /// The refused-old / neighbour-new pair: search for a decimal where the old
    /// two-rounding conversion misses the oracle, prove one exists, and check
    /// the new conversion agrees with the oracle there and on every neighbour
    /// the search inspected.
    #[test]
    fn the_old_two_rounding_conversion_is_refused_and_the_new_one_matches() {
        let mut state = 0x000D_DDEC_u64;
        let mut witness = None;
        'search: for bits in 54..=127_u32 {
            for scale in 1..=18_u8 {
                for _ in 0..64 {
                    let mantissa = random_mantissa(&mut state, bits);
                    let oracle = oracle_f64(mantissa, scale);
                    assert_eq!(decimal_to_f64(mantissa, scale).to_bits(), oracle.to_bits());
                    if old_to_f64(mantissa, scale).to_bits() != oracle.to_bits() {
                        witness = Some((mantissa, scale));
                        break 'search;
                    }
                }
            }
        }
        let (mantissa, scale) = witness.expect("the old conversion misrounds some decimal");
        assert_ne!(
            old_to_f64(mantissa, scale).to_bits(),
            oracle_f64(mantissa, scale).to_bits()
        );
        assert_eq!(
            decimal_to_f64(mantissa, scale).to_bits(),
            oracle_f64(mantissa, scale).to_bits()
        );
    }

    /// `72922151633738826.80`: the old conversion rounded the 63-bit mantissa to
    /// `f64`, then divided, landing on `7.292215163373882e16`; the correctly
    /// rounded value is `7.292215163373883e16`. Pinned so the defect stays
    /// visible even if the random stream above ever changes.
    #[test]
    fn pinned_two_rounding_witness_rounds_correctly() {
        let (mantissa, scale) = (7_292_215_163_373_882_680_i128, 2_u8);
        assert_eq!(old_to_f64(mantissa, scale), 7.292_215_163_373_882e16);
        assert_eq!(oracle_f64(mantissa, scale), 7.292_215_163_373_883e16);
        assert_eq!(
            decimal_to_f64(mantissa, scale).to_bits(),
            7.292_215_163_373_883e16_f64.to_bits()
        );
    }

    /// A pinned f32 double-rounding witness: `(2^24 + 1) × 2^40 + 1` at scale 0
    /// sits just above an `f32` halfway point; through `f64` the `+ 1` is lost and
    /// the remaining exact tie rounds down to even. At scale 1 the same value
    /// times ten keeps the property.
    #[test]
    fn f32_rounds_once_where_narrowing_f64_rounds_twice() {
        let value = (((1i128 << 24) + 1) << 40) + 1;
        let correct = f32::from_bits(((127 + 64) << 23) | 1);
        for (mantissa, scale) in [(value, 0_u8), (value * 10, 1), (value * 1_000_000, 6)] {
            assert_eq!(decimal_to_f32(mantissa, scale).to_bits(), correct.to_bits());
            assert_eq!(oracle_f32(mantissa, scale).to_bits(), correct.to_bits());
            assert_ne!(
                (decimal_to_f64(mantissa, scale) as f32).to_bits(),
                correct.to_bits(),
                "narrowing the f64 rounds twice"
            );
        }
    }
}
