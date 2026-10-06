// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Allocation-free controlled arithmetic and outward binary64 enclosures.

use super::MathError;
use crate::ieee::{Binary64, Binary64Scope};

// The scalar and packed paths expand this one original Dekker equation in the
// same operation order. Each multiply and subtract is separately rounded.
macro_rules! split_product {
    ($a:expr, $b:expr, $product:expr, $splitter:expr, $mul:expr, $sub:expr) => {{
        let a = $a;
        let b = $b;
        let product = $product;
        let mul = $mul;
        let sub = $sub;
        let splitter = $splitter;
        let expanded_a = mul(a, splitter);
        let a_high = sub(expanded_a, sub(expanded_a, a));
        let a_low = sub(a, a_high);
        let expanded_b = mul(b, splitter);
        let b_high = sub(expanded_b, sub(expanded_b, b));
        let b_low = sub(b, b_high);
        let high_error = sub(product, mul(a_high, b_high));
        let first_cross = sub(high_error, mul(a_low, b_high));
        let second_cross = sub(first_cross, mul(a_high, b_low));
        (product, sub(mul(a_low, b_low), second_cross))
    }};
}

mod fast;
mod packed;
mod word;
pub use fast::PreparedBinary64;
pub use packed::FloatProductBackend;
pub use word::{Word, WordInterval};

/// A validated thread-bound floating chunk. No callback or suspension may retain
/// this guard; dropping it restores any changed precision-control state.
#[derive(Debug)]
pub struct CheckedBinary64 {
    pub(super) scope: Binary64Scope,
    pub(super) backend: FloatProductBackend,
}

impl CheckedBinary64 {
    /// The admitted product implementation. Availability was checked before the
    /// chunk was created; it changes instructions, never the arithmetic law.
    #[must_use]
    pub const fn product_backend(&self) -> FloatProductBackend {
        self.backend
    }

    /// The controlled operations borrowed from this live validated chunk.
    #[must_use]
    pub fn ops(&self) -> Binary64<'_> {
        self.scope.ops()
    }
}

/// Inclusive finite binary64 endpoints. Each inexact basic operation expands its
/// nearest-even result by one representable value in each needed direction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FloatInterval {
    lower: f64,
    upper: f64,
}

impl FloatInterval {
    /// Validated finite ordered endpoints.
    ///
    /// # Errors
    /// Refuses nonfinite or reversed endpoints.
    pub fn from_bounds(lower: f64, upper: f64) -> Result<Self, MathError> {
        if !lower.is_finite() || !upper.is_finite() {
            return Err(MathError::Binary64Range);
        }
        if lower > upper {
            return Err(MathError::Domain("reversed binary64 interval"));
        }
        Ok(Self { lower, upper })
    }

    /// The exact value of a finite binary64 operand.
    ///
    /// # Errors
    /// Refuses NaN or infinity.
    pub fn point(value: f64) -> Result<Self, MathError> {
        Self::from_bounds(value, value)
    }
    /// Inclusive lower endpoint.
    #[must_use]
    pub const fn lower(self) -> f64 {
        self.lower
    }
    /// Inclusive upper endpoint.
    #[must_use]
    pub const fn upper(self) -> f64 {
        self.upper
    }

    /// Outward sum using the validated chunk.
    ///
    /// # Errors
    /// Refuses a finite enclosure exceeding binary64's range.
    pub fn add(self, rhs: Self, chunk: &CheckedBinary64) -> Result<Self, MathError> {
        let ops = chunk.ops();
        let lower = sum_bounds(self.lower, rhs.lower, ops)?.0;
        let upper = sum_bounds(self.upper, rhs.upper, ops)?.1;
        Self::from_bounds(lower, upper)
    }
    /// Outward difference using the validated chunk.
    ///
    /// # Errors
    /// Refuses a finite enclosure exceeding binary64's range.
    pub fn sub(self, rhs: Self, chunk: &CheckedBinary64) -> Result<Self, MathError> {
        let ops = chunk.ops();
        let lower = sum_bounds(self.lower, -rhs.upper, ops)?.0;
        let upper = sum_bounds(self.upper, -rhs.lower, ops)?.1;
        Self::from_bounds(lower, upper)
    }
    /// Outward product using all endpoint combinations.
    ///
    /// # Errors
    /// Refuses a finite enclosure exceeding binary64's range.
    pub fn mul(self, rhs: Self, chunk: &CheckedBinary64) -> Result<Self, MathError> {
        let ops = chunk.ops();
        // Directed rounding is monotone, so when either factor has a known sign
        // the outward extremes come from two fixed endpoint products. These are
        // the same bounds the four-product fold selects, and the largest
        // magnitude product is among them, so range refusal is unchanged too.
        let selected = match (sign_class(self), sign_class(rhs)) {
            (Sign::Nonnegative, Sign::Nonnegative) => {
                Some(((self.lower, rhs.lower), (self.upper, rhs.upper)))
            }
            (Sign::Nonnegative, Sign::Nonpositive) => {
                Some(((self.upper, rhs.lower), (self.lower, rhs.upper)))
            }
            (Sign::Nonpositive, Sign::Nonnegative) => {
                Some(((self.lower, rhs.upper), (self.upper, rhs.lower)))
            }
            (Sign::Nonpositive, Sign::Nonpositive) => {
                Some(((self.upper, rhs.upper), (self.lower, rhs.lower)))
            }
            (Sign::Nonnegative, Sign::Mixed) => {
                Some(((self.upper, rhs.lower), (self.upper, rhs.upper)))
            }
            (Sign::Nonpositive, Sign::Mixed) => {
                Some(((self.lower, rhs.upper), (self.lower, rhs.lower)))
            }
            (Sign::Mixed, Sign::Nonnegative) => {
                Some(((self.lower, rhs.upper), (self.upper, rhs.upper)))
            }
            (Sign::Mixed, Sign::Nonpositive) => {
                Some(((self.upper, rhs.lower), (self.lower, rhs.lower)))
            }
            (Sign::Mixed, Sign::Mixed) => None,
        };
        if let Some(((a, b), (c, d))) = selected {
            let lower = outward_product(a, b, ops)?.0;
            let upper = outward_product(c, d, ops)?.1;
            return Self::from_bounds(lower, upper);
        }
        let products = packed::product_bounds4(
            [self.lower, self.lower, self.upper, self.upper],
            [rhs.lower, rhs.upper, rhs.lower, rhs.upper],
            chunk.backend,
            ops,
        )?;
        let lower = products
            .iter()
            .map(|value| value.0)
            .fold(f64::INFINITY, f64::min);
        let upper = products
            .iter()
            .map(|value| value.1)
            .fold(f64::NEG_INFINITY, f64::max);
        Self::from_bounds(lower, upper)
    }

    /// Outward quotient using all endpoint combinations.
    ///
    /// # Errors
    /// Refuses exact zero division, unresolved zero crossings, or range overflow.
    pub fn div(self, rhs: Self, chunk: &CheckedBinary64) -> Result<Self, MathError> {
        if rhs.lower <= 0.0 && rhs.upper >= 0.0 {
            return Err(if rhs.lower == 0.0 && rhs.upper == 0.0 {
                MathError::Domain("division by zero")
            } else {
                MathError::PrecisionExhausted
            });
        }
        let ops = chunk.ops();
        // The divisor excludes zero, so with directed rounding monotone the
        // outward extremes are two fixed endpoint quotients for each sign case.
        let ((a, b), (c, d)) = if rhs.lower > 0.0 {
            match sign_class(self) {
                Sign::Nonnegative => ((self.lower, rhs.upper), (self.upper, rhs.lower)),
                Sign::Nonpositive => ((self.lower, rhs.lower), (self.upper, rhs.upper)),
                Sign::Mixed => ((self.lower, rhs.lower), (self.upper, rhs.lower)),
            }
        } else {
            match sign_class(self) {
                Sign::Nonnegative => ((self.upper, rhs.upper), (self.lower, rhs.lower)),
                Sign::Nonpositive => ((self.upper, rhs.lower), (self.lower, rhs.upper)),
                Sign::Mixed => ((self.upper, rhs.upper), (self.lower, rhs.upper)),
            }
        };
        let lower = outward_quotient(a, b, ops)?.0;
        let upper = outward_quotient(c, d, ops)?.1;
        Self::from_bounds(lower, upper)
    }

    /// Outward square that preserves a crossing's exact zero.
    ///
    /// # Errors
    /// Refuses range overflow.
    pub fn square(self, chunk: &CheckedBinary64) -> Result<Self, MathError> {
        let ops = chunk.ops();
        let a = product_bounds(self.lower, self.lower, ops)?;
        let b = product_bounds(self.upper, self.upper, ops)?;
        let lower = if self.lower <= 0.0 && self.upper >= 0.0 {
            0.0
        } else {
            a.0.min(b.0).max(0.0)
        };
        Self::from_bounds(lower, a.1.max(b.1))
    }
    /// Outward nonnegative square root.
    ///
    /// # Errors
    /// Refuses negative inputs, unresolved domain boundaries, or range overflow.
    pub fn sqrt(self, chunk: &CheckedBinary64) -> Result<Self, MathError> {
        if self.lower < 0.0 {
            return Err(if self.upper < 0.0 {
                MathError::Domain("negative square root")
            } else {
                MathError::PrecisionExhausted
            });
        }
        let ops = chunk.ops();
        let lower = outward_root(self.lower, ops);
        if self.lower == self.upper {
            return Self::from_bounds(lower.0.max(0.0), lower.1);
        }
        Self::from_bounds(lower.0.max(0.0), outward_root(self.upper, ops).1)
    }
    /// Exact negation with exchanged endpoints.
    #[must_use]
    pub const fn neg(self) -> Self {
        Self {
            lower: -self.upper,
            upper: -self.lower,
        }
    }
    /// Outward absolute value preserving a crossing's exact zero.
    #[must_use]
    pub fn abs(self) -> Self {
        let a = self.lower.abs();
        let b = self.upper.abs();
        Self {
            lower: if self.lower <= 0.0 && self.upper >= 0.0 {
                0.0
            } else {
                a.min(b)
            },
            upper: a.max(b),
        }
    }
}

/// A two-component approximation with an explicit certified residual enclosure.
/// The represented true value belongs to `hi + lo + residual`. The low component
/// improves approximation; only the residual certificate licenses comparisons.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DoubleDouble {
    hi: f64,
    lo: f64,
    residual: FloatInterval,
}

impl DoubleDouble {
    /// Exactly represent one finite binary64 value.
    ///
    /// # Errors
    /// Refuses NaN or infinity.
    pub fn from_binary64(value: f64) -> Result<Self, MathError> {
        FloatInterval::point(value)?;
        Ok(Self {
            hi: value,
            lo: 0.0,
            residual: FloatInterval {
                lower: 0.0,
                upper: 0.0,
            },
        })
    }
    /// Leading approximation component.
    #[must_use]
    pub const fn hi(self) -> f64 {
        self.hi
    }
    /// Trailing approximation component.
    #[must_use]
    pub const fn lo(self) -> f64 {
        self.lo
    }
    /// Inclusive residual added to both components.
    #[must_use]
    pub const fn residual(self) -> FloatInterval {
        self.residual
    }

    /// A certified binary64 enclosure of the represented true value.
    ///
    /// # Errors
    /// Refuses range overflow.
    pub fn enclosure(self, chunk: &CheckedBinary64) -> Result<FloatInterval, MathError> {
        FloatInterval::point(self.hi)?
            .add(FloatInterval::point(self.lo)?, chunk)?
            .add(self.residual, chunk)
    }

    /// Add expansions by exact two-sum decompositions and retain every discarded
    /// component in the explicit residual enclosure.
    ///
    /// # Errors
    /// Refuses a decomposition or residual that overflows finite binary64.
    pub fn add(self, rhs: Self, chunk: &CheckedBinary64) -> Result<Self, MathError> {
        let ops = chunk.ops();
        let (t, e) = two_sum(self.hi, rhs.hi, ops)?;
        let (u, f) = two_sum(self.lo, rhs.lo, ops)?;
        let (v, g) = two_sum(e, u, ops)?;
        let (hi, lo) = two_sum(t, v, ops)?;
        let residual = FloatInterval::point(f)?
            .add(FloatInterval::point(g)?, chunk)?
            .add(self.residual, chunk)?
            .add(rhs.residual, chunk)?;
        Ok(Self { hi, lo, residual })
    }

    /// Exact sign reversal of the expansion and its certificate.
    #[must_use]
    pub const fn neg(self) -> Self {
        Self {
            hi: -self.hi,
            lo: -self.lo,
            residual: self.residual.neg(),
        }
    }

    /// Multiply with independently rounded split products, retaining an outward
    /// certificate for the whole represented product. No fused multiply is used.
    ///
    /// # Errors
    /// Refuses a decomposition or certificate exceeding finite binary64.
    pub fn mul(self, rhs: Self, chunk: &CheckedBinary64) -> Result<Self, MathError> {
        let ops = chunk.ops();
        let hi = ops.mul(self.hi, rhs.hi);
        if !hi.is_finite() {
            return Err(MathError::Binary64Range);
        }
        let error = two_product(self.hi, rhs.hi, ops)?.map_or(0.0, |(_, error)| error);
        let cross = ops.add(ops.mul(self.hi, rhs.lo), ops.mul(self.lo, rhs.hi));
        let low = ops.add(ops.add(error, cross), ops.mul(self.lo, rhs.lo));
        let (hi, lo) = two_sum(hi, low, ops)?;
        // This bound also handles split subproducts that underflow: no assertion
        // of an error-free product or implicit 106-bit precision is made there.
        let truth = self.enclosure(chunk)?.mul(rhs.enclosure(chunk)?, chunk)?;
        let residual = truth.sub(
            FloatInterval::point(hi)?.add(FloatInterval::point(lo)?, chunk)?,
            chunk,
        )?;
        Ok(Self { hi, lo, residual })
    }
}

#[derive(Clone, Copy)]
enum Sign {
    Nonnegative,
    Nonpositive,
    Mixed,
}

// Zero endpoints are exact, so a factor touching zero still has a known sign.
fn sign_class(value: FloatInterval) -> Sign {
    if value.lower >= 0.0 {
        Sign::Nonnegative
    } else if value.upper <= 0.0 {
        Sign::Nonpositive
    } else {
        Sign::Mixed
    }
}

fn two_sum(a: f64, b: f64, ops: Binary64<'_>) -> Result<(f64, f64), MathError> {
    let sum = ops.add(a, b);
    let virtual_b = ops.sub(sum, a);
    let virtual_a = ops.sub(sum, virtual_b);
    let error = ops.add(ops.sub(a, virtual_a), ops.sub(b, virtual_b));
    if !sum.is_finite() || !error.is_finite() {
        return Err(MathError::Binary64Range);
    }
    Ok((sum, error))
}

// These identities follow the exact addition and split-product equations in
// Dekker, Numerische Mathematik18 (1971), doi:10.1007/BF01397083, sections4--6.
// They are independently implemented here through the controlled binary64 home.
fn sum_bounds(a: f64, b: f64, ops: Binary64<'_>) -> Result<(f64, f64), MathError> {
    let (sum, error) = two_sum(a, b, ops)?;
    Ok(directed_error(sum, error))
}

fn directed_error(value: f64, error: f64) -> (f64, f64) {
    (
        if error < 0.0 {
            value.next_down()
        } else {
            value
        },
        if error > 0.0 { value.next_up() } else { value },
    )
}

/// Exact product decomposition in a range that proves every nonzero split
/// subproduct normal. An operand in[2^-450,2^450] has least significant bit at
/// least2^-502; split subproducts are therefore >=2^-1004. The splitter cannot
/// overflow because its product is <=2^478. Outside this proven range the
/// caller keeps the ordinary adjacent-neighbor certificate.
fn two_product(a: f64, b: f64, ops: Binary64<'_>) -> Result<Option<(f64, f64)>, MathError> {
    let product = ops.mul(a, b);
    if !product.is_finite() {
        return Err(MathError::Binary64Range);
    }
    if a == 0.0 || b == 0.0 {
        return Ok(Some((product, 0.0)));
    }
    let minimum = f64::from_bits((1023 - 450) << 52);
    let maximum = f64::from_bits((1023 + 450) << 52);
    if a.abs() < minimum || b.abs() < minimum || a.abs() > maximum || b.abs() > maximum {
        return Ok(None);
    }
    Ok(Some(split_product!(
        a,
        b,
        product,
        134_217_729.0,
        |x, y| ops.mul(x, y),
        |x, y| ops.sub(x, y)
    )))
}

/// One correctly rounded product widened to its two neighbours. Rounding to
/// nearest moves a finite product by at most half a local gap, including in
/// gradual underflow, so the exact product lies between the neighbours. A zero
/// factor gives an exact zero; a nonfinite product refuses as range overflow.
fn outward_product(a: f64, b: f64, ops: Binary64<'_>) -> Result<(f64, f64), MathError> {
    let product = ops.mul(a, b);
    if !product.is_finite() {
        return Err(MathError::Binary64Range);
    }
    if a == 0.0 || b == 0.0 {
        return Ok((product, product));
    }
    Ok((product.next_down(), product.next_up()))
}

/// One correctly rounded quotient widened to its two neighbours; a zero
/// dividend is exact. A nonfinite quotient refuses as range overflow.
fn outward_quotient(a: f64, b: f64, ops: Binary64<'_>) -> Result<(f64, f64), MathError> {
    let quotient = ops.div(a, b);
    if !quotient.is_finite() {
        return Err(MathError::Binary64Range);
    }
    if a == 0.0 {
        return Ok((quotient, quotient));
    }
    Ok((quotient.next_down(), quotient.next_up()))
}

/// One correctly rounded nonnegative square root widened to its neighbours;
/// zero is exact.
fn outward_root(value: f64, ops: Binary64<'_>) -> (f64, f64) {
    if value == 0.0 {
        return (0.0, 0.0);
    }
    let root = ops.sqrt(value);
    (root.next_down(), root.next_up())
}

/// The unselected outward product of two endpoints, for equivalence tests.
#[cfg(test)]
pub(super) fn reference_product(a: f64, b: f64, ops: Binary64<'_>) -> (f64, f64) {
    product_bounds(a, b, ops).expect("finite test product")
}

fn product_bounds(a: f64, b: f64, ops: Binary64<'_>) -> Result<(f64, f64), MathError> {
    Ok(if let Some((product, error)) = two_product(a, b, ops)? {
        directed_error(product, error)
    } else {
        let product = ops.mul(a, b);
        (product.next_down(), product.next_up())
    })
}
