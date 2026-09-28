// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The exact decimal expansion of a binary64 or binary32 value, and of the midpoints
//! between it and its neighbours.
//!
//! Every finite binary floating-point value is a dyadic rational, `±sig · 2^exp`, and
//! every dyadic rational has a finite decimal expansion: `2^−k = 5^k / 10^k`, so the
//! digits are those of `sig · 5^k` read at `k` decimal places. The expansions here are
//! exact and complete — up to 1074 fractional digits for the smallest binary64
//! subnormal — with no rounding anywhere, computed on [`crate::BigInt`].
//!
//! Two consumers need them. A validator comparing a JSON number against a decimal
//! facet (`multipleOf 0.1` over a double that a reader produced) must reason about
//! the value the double *is*, not its shortest round-trip spelling, and
//! [`f64_exact_decimal`] is that value. And a correctly rounded decimal reader is
//! tested at the points where reading is hardest: the midpoint between two adjacent
//! binary values, which a reader can only decide by consuming every digit and which
//! it must break to the even neighbour. [`f64_midpoint_above`] and its three siblings
//! write those midpoints, sign-aware, at either width.
//!
//! # Form
//!
//! An integer value has no decimal point (`1`, `9007199254740993`); a fractional one
//! has exactly as many fractional digits as its expansion needs, with no trailing
//! zeros (`0.5`, `2.5`, `0.1000000000000000055511151231257827021181583404541015625`),
//! since an odd numerator times `5^k` always ends in `5`. A negative value carries a
//! leading `-`; negative zero is `-0`, keeping the sign bit visible.
//!
//! # Midpoints
//!
//! "Above" is toward `+∞` and "below" toward `−∞`, so for a negative value "above"
//! is the midpoint to the neighbour nearer zero. The neighbours are the adjacent
//! values of the format's grid: across a binade boundary the spacing halves, so the
//! midpoint below a power of two is a quarter of the ulp above it; the neighbour
//! above the largest finite value is `2^1024` (`2^128` for binary32), the overflow
//! threshold, so [`f64_midpoint_above`] of `f64::MAX` is the decimal at which a reader's
//! tie goes to infinity; and the two zeros share one value, so their midpoints are
//! `±2^−1075` (`±2^−150`). Only a NaN or an infinity has no expansion or midpoint
//! and answers `None`.
//!
//! ```rust
//! use purrdf_xsd::ieee::exact::{f64_exact_decimal, f64_midpoint_above, f64_midpoint_below};
//!
//! assert_eq!(f64_exact_decimal(0.5).as_deref(), Some("0.5"));
//! assert_eq!(
//!     f64_exact_decimal(0.1).as_deref(),
//!     Some("0.1000000000000000055511151231257827021181583404541015625")
//! );
//! // The tie between 1.0 and its successor reads back as the even neighbour, 1.0.
//! let tie = f64_midpoint_above(1.0).expect("finite");
//! assert_eq!(tie, "1.00000000000000011102230246251565404236316680908203125");
//! assert_eq!(tie.parse::<f64>(), Ok(1.0));
//! // Sign-aware: above −1.0 is the midpoint toward zero.
//! assert_eq!(
//!     f64_midpoint_above(-1.0).as_deref(),
//!     Some("-0.999999999999999944488848768742172978818416595458984375")
//! );
//! assert_eq!(f64_midpoint_below(f64::NAN), None);
//! ```

use crate::bigint::BigInt;

/// A finite value of one binary format: `(−1)^neg · sig · 2^exp`, with `sig` the
/// format's full significand (implicit bit included) and the two facts a
/// neighbour computation needs beyond the value itself.
#[derive(Debug, Clone, Copy)]
struct Dyadic {
    neg: bool,
    sig: u64,
    exp: i32,
    /// The exponent of the format's subnormals (and of its smallest normal): where a
    /// zero's neighbours sit.
    min_exp: i32,
    /// Whether `sig` is the smallest normal significand at an exponent above
    /// `min_exp`, so that the neighbour toward zero lies one binade down, at half
    /// the spacing.
    at_binade_floor: bool,
}

/// `x` decomposed, or `None` when it is not finite.
fn decompose64(x: f64) -> Option<Dyadic> {
    if !x.is_finite() {
        return None;
    }
    let bits = x.to_bits();
    let biased = (bits >> 52) & 0x7ff;
    let fraction = bits & ((1 << 52) - 1);
    let (sig, exp) = if biased == 0 {
        (fraction, -1074)
    } else {
        (
            fraction | (1 << 52),
            i32::try_from(biased).expect("eleven bits") - 1075,
        )
    };
    Some(Dyadic {
        neg: bits >> 63 == 1,
        sig,
        exp,
        min_exp: -1074,
        at_binade_floor: biased > 1 && sig == 1 << 52,
    })
}

/// `x` decomposed, or `None` when it is not finite.
fn decompose32(x: f32) -> Option<Dyadic> {
    if !x.is_finite() {
        return None;
    }
    let bits = x.to_bits();
    let biased = (bits >> 23) & 0xff;
    let fraction = bits & ((1 << 23) - 1);
    let (sig, exp) = if biased == 0 {
        (fraction, -149)
    } else {
        (
            fraction | (1 << 23),
            i32::try_from(biased).expect("eight bits") - 150,
        )
    };
    Some(Dyadic {
        neg: bits >> 31 == 1,
        sig: u64::from(sig),
        exp,
        min_exp: -149,
        at_binade_floor: biased > 1 && sig == 1 << 23,
    })
}

/// The exact decimal expansion of `(−1)^neg · sig · 2^exp`, in the module's form.
fn render(neg: bool, mut sig: u128, mut exp: i32) -> String {
    if sig == 0 {
        return if neg { "-0" } else { "0" }.to_string();
    }
    // Reduce to an odd numerator wherever the value is fractional, so the
    // expansion carries no trailing zeros.
    while exp < 0 && sig & 1 == 0 {
        sig >>= 1;
        exp += 1;
    }
    let mut magnitude = BigInt::from_u128(sig);
    let scale = if exp >= 0 {
        magnitude = magnitude.mul_pow2(exp.unsigned_abs());
        0
    } else {
        magnitude.mul_pow5(exp.unsigned_abs());
        usize::try_from(exp.unsigned_abs()).expect("a small scale")
    };
    let digits = magnitude.to_decimal_string();
    let mut out = String::with_capacity(digits.len() + scale + 3);
    if neg {
        out.push('-');
    }
    if scale == 0 {
        out.push_str(&digits);
    } else if digits.len() > scale {
        let split = digits.len() - scale;
        out.push_str(&digits[..split]);
        out.push('.');
        out.push_str(&digits[split..]);
    } else {
        out.push_str("0.");
        for _ in digits.len()..scale {
            out.push('0');
        }
        out.push_str(&digits);
    }
    out
}

/// The midpoint between `value` and its neighbour toward `+∞` (`above`) or `−∞`.
fn midpoint(value: Dyadic, above: bool) -> String {
    if value.sig == 0 {
        // Both zeros are one value; its neighbours are ± the smallest subnormal.
        return render(!above, 1, value.min_exp - 1);
    }
    let sig = u128::from(value.sig);
    // Toward +∞ on a positive value, or toward −∞ on a negative one, is away from
    // zero: the neighbour of larger magnitude, always at the same spacing.
    let away = above != value.neg;
    let (numerator, exp) = if away {
        (2 * sig + 1, value.exp - 1)
    } else if value.at_binade_floor {
        (4 * sig - 1, value.exp - 2)
    } else {
        (2 * sig - 1, value.exp - 1)
    };
    render(value.neg, numerator, exp)
}

/// The exact decimal expansion of the binary64 value `x`, or `None` for a NaN or an
/// infinity. See the [module documentation](self) for the form.
///
/// # Examples
///
/// ```rust
/// use purrdf_xsd::ieee::exact::f64_exact_decimal;
///
/// assert_eq!(f64_exact_decimal(1.0).as_deref(), Some("1"));
/// assert_eq!(f64_exact_decimal(-2.5).as_deref(), Some("-2.5"));
/// assert_eq!(f64_exact_decimal(-0.0).as_deref(), Some("-0"));
/// assert_eq!(f64_exact_decimal(f64::INFINITY), None);
/// // Exact: the expansion parses back to the very same bits.
/// let x = 6.02214076e23;
/// assert_eq!(f64_exact_decimal(x).unwrap().parse::<f64>(), Ok(x));
/// ```
#[must_use]
pub fn f64_exact_decimal(x: f64) -> Option<String> {
    let value = decompose64(x)?;
    Some(render(value.neg, u128::from(value.sig), value.exp))
}

/// The exact decimal expansion of the binary32 value `x`, or `None` for a NaN or an
/// infinity. See the [module documentation](self) for the form.
///
/// # Examples
///
/// ```rust
/// use purrdf_xsd::ieee::exact::f32_exact_decimal;
///
/// assert_eq!(f32_exact_decimal(0.1).as_deref(), Some("0.100000001490116119384765625"));
/// assert_eq!(
///     f32_exact_decimal(f32::MAX).as_deref(),
///     Some("340282346638528859811704183484516925440")
/// );
/// assert_eq!(f32_exact_decimal(f32::NAN), None);
/// ```
#[must_use]
pub fn f32_exact_decimal(x: f32) -> Option<String> {
    let value = decompose32(x)?;
    Some(render(value.neg, u128::from(value.sig), value.exp))
}

/// The exact decimal expansion of the midpoint between the binary64 value `x` and
/// its neighbour toward `+∞`, or `None` for a NaN or an infinity. For `f64::MAX`
/// the neighbour is `2^1024`, so the result is the overflow threshold a
/// correctly rounded reader ties to infinity at. See the [module
/// documentation](self).
///
/// # Examples
///
/// ```rust
/// use purrdf_xsd::ieee::exact::f64_midpoint_above;
///
/// // 2^53 and 2^53 + 2 straddle the integer 2^53 + 1 exactly.
/// assert_eq!(
///     f64_midpoint_above(9_007_199_254_740_992.0).as_deref(),
///     Some("9007199254740993")
/// );
/// assert_eq!(f64_midpoint_above(f64::INFINITY), None);
/// ```
#[must_use]
pub fn f64_midpoint_above(x: f64) -> Option<String> {
    decompose64(x).map(|value| midpoint(value, true))
}

/// The exact decimal expansion of the midpoint between the binary64 value `x` and
/// its neighbour toward `−∞`, or `None` for a NaN or an infinity. See the [module
/// documentation](self).
///
/// # Examples
///
/// ```rust
/// use purrdf_xsd::ieee::exact::{f64_midpoint_above, f64_midpoint_below};
///
/// // Below 1.0 the grid is twice as fine, so the midpoint is a quarter ulp away.
/// assert_eq!(
///     f64_midpoint_below(1.0).as_deref(),
///     Some("0.999999999999999944488848768742172978818416595458984375")
/// );
/// // The two zeros are one value: below it lies −2^−1075.
/// assert_eq!(
///     f64_midpoint_below(0.0).map(|s| s.len()),
///     f64_midpoint_above(-0.0).map(|s| s.len() + 1)
/// );
/// ```
#[must_use]
pub fn f64_midpoint_below(x: f64) -> Option<String> {
    decompose64(x).map(|value| midpoint(value, false))
}

/// The exact decimal expansion of the midpoint between the binary32 value `x` and
/// its neighbour toward `+∞`, or `None` for a NaN or an infinity. For `f32::MAX` the
/// neighbour is `2^128`. See the [module documentation](self).
///
/// # Examples
///
/// ```rust
/// use purrdf_xsd::ieee::exact::f32_midpoint_above;
///
/// let threshold = f32_midpoint_above(f32::MAX).expect("finite");
/// assert_eq!(threshold, "340282356779733661637539395458142568448");
/// // The tie rounds to even: 2^128, which is infinity in binary32.
/// assert_eq!(threshold.parse::<f32>(), Ok(f32::INFINITY));
/// ```
#[must_use]
pub fn f32_midpoint_above(x: f32) -> Option<String> {
    decompose32(x).map(|value| midpoint(value, true))
}

/// The exact decimal expansion of the midpoint between the binary32 value `x` and
/// its neighbour toward `−∞`, or `None` for a NaN or an infinity. See the [module
/// documentation](self).
///
/// # Examples
///
/// ```rust
/// use purrdf_xsd::ieee::exact::f32_midpoint_below;
///
/// // 1 − 2^−25: the tie between 1.0 and its binary32 predecessor.
/// assert_eq!(
///     f32_midpoint_below(1.0).as_deref(),
///     Some("0.9999999701976776123046875")
/// );
/// assert_eq!(f32_midpoint_below(1.0).unwrap().parse::<f32>(), Ok(1.0));
/// ```
#[must_use]
pub fn f32_midpoint_below(x: f32) -> Option<String> {
    decompose32(x).map(|value| midpoint(value, false))
}

#[cfg(test)]
mod tests {
    use super::{
        f32_exact_decimal, f32_midpoint_above, f32_midpoint_below, f64_exact_decimal,
        f64_midpoint_above, f64_midpoint_below,
    };
    use crate::ieee::reference;

    use purrdf_testkit::rng::splitmix64_next as splitmix64;

    /// The sweep every test walks: the named corners, then deterministic draws over
    /// every binade, subnormals included.
    fn sweep64() -> Vec<f64> {
        let mut xs = vec![
            0.0,
            -0.0,
            5e-324,
            -5e-324,
            f64::from_bits(0x000f_ffff_ffff_ffff),
            f64::MIN_POSITIVE,
            f64::from_bits(0x0010_0000_0000_0001),
            1.0,
            -1.0,
            0.1,
            -0.1,
            2.5,
            -2.5,
            1e23,
            9_007_199_254_740_992.0,
            f64::MAX,
            -f64::MAX,
            f64::MAX / 2.0,
            f64::EPSILON,
            0.5,
            2.0,
            1.0 - f64::EPSILON / 2.0,
        ];
        let mut state = 0x6578_6163_7436_3400_u64;
        for _ in 0..400 {
            let bits = splitmix64(&mut state) & 0x7fef_ffff_ffff_ffff;
            let x = f64::from_bits(bits);
            xs.push(x);
            xs.push(-x);
        }
        xs
    }

    fn sweep32() -> Vec<f32> {
        let mut xs = vec![
            0.0,
            -0.0,
            f32::from_bits(1),
            -f32::from_bits(1),
            f32::from_bits(0x007f_ffff),
            f32::MIN_POSITIVE,
            1.0,
            -1.0,
            0.1,
            2.5,
            -2.5,
            f32::MAX,
            -f32::MAX,
            f32::EPSILON,
            16_777_216.0,
        ];
        let mut state = 0x6578_6163_7433_3200_u64;
        for _ in 0..400 {
            let bits = (splitmix64(&mut state) as u32) & 0x7f7f_ffff;
            let x = f32::from_bits(bits);
            xs.push(x);
            xs.push(-x);
        }
        xs
    }

    /// The next binary64 value above a non-negative `x` below `f64::MAX`.
    fn succ64(x: f64) -> f64 {
        f64::from_bits(x.to_bits() + 1)
    }

    /// The next binary32 value above a non-negative `x` below `f32::MAX`.
    fn succ32(x: f32) -> f32 {
        f32::from_bits(x.to_bits() + 1)
    }

    /// Of two adjacent values, the one with an even significand.
    fn even64(x: f64, next: f64) -> f64 {
        if x.to_bits() & 1 == 0 { x } else { next }
    }

    fn even32(x: f32, next: f32) -> f32 {
        if x.to_bits() & 1 == 0 { x } else { next }
    }

    /// The same decimal, nudged up by one unit far past its last digit.
    fn nudge_up(decimal: &str) -> String {
        let (sign, magnitude) = match decimal.strip_prefix('-') {
            Some(rest) => ("-", rest),
            None => ("", decimal),
        };
        if magnitude.contains('.') {
            format!("{sign}{magnitude}0000000001")
        } else {
            format!("{sign}{magnitude}.0000000001")
        }
    }

    /// Only a fractional decimal (ending in 5) can be nudged down in place.
    fn nudge_down(decimal: &str) -> Option<String> {
        decimal
            .contains('.')
            .then(|| format!("{}4999", &decimal[..decimal.len() - 1]))
    }

    #[test]
    fn the_new_api_equals_the_reference_oracle_on_the_sweep() {
        for x in sweep64() {
            if x == 0.0 {
                continue;
            }
            let magnitude = x.abs();
            let expected = reference::successor_midpoint_decimal(magnitude);
            if x > 0.0 {
                assert_eq!(
                    f64_midpoint_above(x).as_deref(),
                    Some(expected.as_str()),
                    "{x:e}"
                );
            } else {
                assert_eq!(
                    f64_midpoint_below(x),
                    Some(format!("-{expected}")),
                    "{x:e}: below a negative is the successor midpoint of its magnitude"
                );
            }
            // The midpoint toward zero is the reference's successor midpoint of the
            // predecessor, wherever the predecessor is positive.
            let pred = f64::from_bits(magnitude.to_bits() - 1);
            if pred > 0.0 {
                let expected = reference::successor_midpoint_decimal(pred);
                if x > 0.0 {
                    assert_eq!(
                        f64_midpoint_below(x).as_deref(),
                        Some(expected.as_str()),
                        "{x:e}"
                    );
                } else {
                    assert_eq!(f64_midpoint_above(x), Some(format!("-{expected}")), "{x:e}");
                }
            }
        }
    }

    #[test]
    fn exact_decimals_parse_back_to_the_same_bits() {
        for x in sweep64() {
            let decimal = f64_exact_decimal(x).expect("finite");
            assert_eq!(
                decimal.parse::<f64>().map(f64::to_bits),
                Ok(x.to_bits()),
                "{x:e}"
            );
            assert!(
                !decimal.ends_with('0') || !decimal.contains('.'),
                "no trailing zero: {decimal}"
            );
            assert_eq!(decimal.starts_with('-'), x.is_sign_negative(), "{x:e}");
        }
        for x in sweep32() {
            let decimal = f32_exact_decimal(x).expect("finite");
            assert_eq!(
                decimal.parse::<f32>().map(f32::to_bits),
                Ok(x.to_bits()),
                "{x:e}"
            );
            // Every binary32 value is a binary64 value: the two paths agree.
            assert_eq!(decimal, f64_exact_decimal(f64::from(x)).expect("finite"));
        }
    }

    #[test]
    fn pinned_expansions() {
        assert_eq!(f64_exact_decimal(1.0).as_deref(), Some("1"));
        assert_eq!(f64_exact_decimal(-2.5).as_deref(), Some("-2.5"));
        assert_eq!(f64_exact_decimal(0.0).as_deref(), Some("0"));
        assert_eq!(f64_exact_decimal(-0.0).as_deref(), Some("-0"));
        assert_eq!(
            f64_exact_decimal(0.1).as_deref(),
            Some("0.1000000000000000055511151231257827021181583404541015625")
        );
        // f64::MAX = (2^53 − 1)·2^971, a 309-digit integer.
        let max = f64_exact_decimal(f64::MAX).expect("finite");
        assert_eq!(max.len(), 309);
        assert!(max.starts_with("17976931348623157"));
        assert_eq!(
            max,
            crate::BigInt::from_u128((1 << 53) - 1)
                .mul_pow2(971)
                .to_decimal_string()
        );
        // 2^−1074: 751 significant digits after 323 zeros, 1074 places in all.
        let tiny = f64_exact_decimal(5e-324).expect("finite");
        assert_eq!(tiny.len(), 2 + 1074);
        assert!(tiny.starts_with(&format!("0.{}4940656458412465", "0".repeat(323))));
        assert!(tiny.ends_with('5'));
        assert_eq!(
            f32_exact_decimal(0.1).as_deref(),
            Some("0.100000001490116119384765625")
        );
        assert_eq!(f32_exact_decimal(-2.5).as_deref(), Some("-2.5"));
        assert_eq!(
            f32_exact_decimal(f32::MAX).as_deref(),
            Some("340282346638528859811704183484516925440")
        );
        // 2^−149: 105 significant digits after 44 zeros.
        let tiny = f32_exact_decimal(f32::from_bits(1)).expect("finite");
        assert_eq!(tiny.len(), 2 + 149);
        assert!(tiny.starts_with(&format!("0.{}14012984643", "0".repeat(44))));
        assert!(tiny.ends_with('5'));
        for x in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(f64_exact_decimal(x), None);
            assert_eq!(f64_midpoint_above(x), None);
            assert_eq!(f64_midpoint_below(x), None);
        }
        for x in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert_eq!(f32_exact_decimal(x), None);
            assert_eq!(f32_midpoint_above(x), None);
            assert_eq!(f32_midpoint_below(x), None);
        }
    }

    #[test]
    fn pinned_midpoints_and_sign_awareness() {
        let above_one = "1.00000000000000011102230246251565404236316680908203125";
        let below_one = "0.999999999999999944488848768742172978818416595458984375";
        assert_eq!(f64_midpoint_above(1.0).as_deref(), Some(above_one));
        assert_eq!(f64_midpoint_below(1.0).as_deref(), Some(below_one));
        assert_eq!(f64_midpoint_above(-1.0), Some(format!("-{below_one}")));
        assert_eq!(f64_midpoint_below(-1.0), Some(format!("-{above_one}")));
        assert_eq!(
            f64_midpoint_above(-2.5).as_deref(),
            Some("-2.499999999999999777955395074968691915273666381835937500".trim_end_matches('0'))
        );
        assert_eq!(
            f64_midpoint_below(-2.5).as_deref(),
            Some("-2.500000000000000222044604925031308084726333618164062500".trim_end_matches('0'))
        );
        // The two zeros share one value and one pair of midpoints, ±2^−1075.
        let half_subnormal = f64_midpoint_above(0.0).expect("finite");
        assert_eq!(f64_midpoint_above(-0.0), Some(half_subnormal.clone()));
        assert_eq!(f64_midpoint_below(0.0), Some(format!("-{half_subnormal}")));
        assert_eq!(f64_midpoint_below(-0.0), Some(format!("-{half_subnormal}")));
        assert_eq!(half_subnormal.len(), 2 + 1075);
        assert!(half_subnormal.starts_with(&format!("0.{}24703282292062327", "0".repeat(323))));
        // And below the smallest subnormal is that same midpoint.
        assert_eq!(f64_midpoint_below(5e-324), Some(half_subnormal.clone()));
        assert_eq!(
            f64_midpoint_above(-5e-324),
            Some(format!("-{half_subnormal}"))
        );
        // Past the largest finite value: the overflow threshold, (2^54 − 1)·2^970.
        let threshold = f64_midpoint_above(f64::MAX).expect("finite");
        assert_eq!(threshold, reference::successor_midpoint_decimal(f64::MAX));
        assert_eq!(threshold.parse::<f64>(), Ok(f64::INFINITY));
        // (2^54 − 1)·2^970 ends in 2 (…3 × …4), so one less is a digit away.
        assert!(threshold.ends_with('2'));
        let just_under = format!("{}1", &threshold[..threshold.len() - 1]);
        assert_eq!(just_under.parse::<f64>(), Ok(f64::MAX));
        assert_eq!(f64_midpoint_below(-f64::MAX), Some(format!("-{threshold}")));
        // binary32: at MAX, at a binade floor, and at zero.
        let threshold = f32_midpoint_above(f32::MAX).expect("finite");
        assert_eq!(threshold, "340282356779733661637539395458142568448");
        assert_eq!(threshold.parse::<f32>(), Ok(f32::INFINITY));
        assert_eq!(
            f32_midpoint_below(1.0).as_deref(),
            Some("0.9999999701976776123046875")
        );
        assert_eq!(
            f32_midpoint_above(1.0).as_deref(),
            Some("1.00000005960464477539062500".trim_end_matches('0'))
        );
        let half_subnormal = f32_midpoint_above(0.0).expect("finite");
        assert_eq!(f32_midpoint_below(-0.0), Some(format!("-{half_subnormal}")));
        assert_eq!(half_subnormal.len(), 2 + 150);
        assert_eq!(half_subnormal.parse::<f32>(), Ok(0.0));
        assert_eq!(
            nudge_up(&half_subnormal).parse::<f32>(),
            Ok(f32::from_bits(1))
        );
    }

    /// Every midpoint is a tie: on a magnitude `m` the midpoint above reads as the
    /// even neighbour, a digit more reads as the successor, a digit less as `m` —
    /// at both widths, by Rust's correctly rounded parsers as the independent
    /// oracle — and the negative values' midpoints are the magnitudes' mirrored.
    #[test]
    fn every_midpoint_is_an_exact_tie() {
        for x in sweep64() {
            let m = x.abs();
            if m == f64::MAX {
                continue;
            }
            let above = f64_midpoint_above(m).expect("finite");
            let upper = succ64(m);
            assert_eq!(
                above.parse::<f64>(),
                Ok(even64(m, upper)),
                "tie above {m:e}"
            );
            assert_eq!(nudge_up(&above).parse::<f64>(), Ok(upper), "above {m:e}");
            if let Some(lower) = nudge_down(&above) {
                assert_eq!(
                    lower.parse::<f64>(),
                    Ok(m),
                    "just under the tie above {m:e}"
                );
            }
            // Mirrors: below −m is −(above m); above −m is −(below m) for m > 0, and
            // the zeros share their midpoints.
            assert_eq!(f64_midpoint_below(-m), Some(format!("-{above}")), "{m:e}");
            if m > 0.0 {
                let below = f64_midpoint_below(m).expect("finite");
                assert!(!below.starts_with('-'));
                assert_eq!(f64_midpoint_above(-m), Some(format!("-{below}")), "{m:e}");
            } else {
                assert_eq!(f64_midpoint_above(-0.0), Some(above.clone()));
                assert_eq!(f64_midpoint_below(0.0), Some(format!("-{above}")));
            }
        }
        for x in sweep32() {
            let m = x.abs();
            if m == f32::MAX {
                continue;
            }
            let above = f32_midpoint_above(m).expect("finite");
            let upper = succ32(m);
            assert_eq!(
                above.parse::<f32>(),
                Ok(even32(m, upper)),
                "tie above {m:e}"
            );
            assert_eq!(nudge_up(&above).parse::<f32>(), Ok(upper), "above {m:e}");
            if let Some(lower) = nudge_down(&above) {
                assert_eq!(
                    lower.parse::<f32>(),
                    Ok(m),
                    "just under the tie above {m:e}"
                );
            }
            assert_eq!(f32_midpoint_below(-m), Some(format!("-{above}")), "{m:e}");
            if m > 0.0 {
                let below = f32_midpoint_below(m).expect("finite");
                assert_eq!(f32_midpoint_above(-m), Some(format!("-{below}")), "{m:e}");
            } else {
                assert_eq!(f32_midpoint_above(-0.0), Some(above));
            }
        }
    }
}
