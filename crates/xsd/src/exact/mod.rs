// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The arbitrary-precision numeric tower: exact `xsd:integer`, `xsd:decimal` and
//! `owl:rational` values of any size, beside the machine-word representations.
//!
//! [`crate::XsdValue`] holds an integer or decimal that fits machine words in
//! its bounded variants and every other one on this tower
//! ([`crate::XsdValue::BigInteger`], [`crate::XsdValue::BigDecimal`]); the
//! types here are also usable on their own for exact reasoned mathematics
//! (combinatorics, partition functions, exact quotients).
//!
//! | Type | Value space | Arithmetic |
//! |---|---|---|
//! | [`Integer`] | `xsd:integer`, unbounded, inline `i128` fast path | `+ − ×` exact; truncating `div_rem`; `pow`; `gcd` |
//! | [`Decimal`] | `xsd:decimal`, big coefficient, unbounded scale | `+ − ×` exact; `÷` under a [`DivisionPolicy`]; rounding in every [`Rounding`] direction |
//! | [`Rational`] | `owl:rational`, unbounded | `+ − × ÷` exact (a field) |
//!
//! ```rust
//! use purrdf_xsd::exact::{Decimal, DivisionPolicy, Integer, Rational, Rounding};
//!
//! // 34! is past i128 (and past 10^38): an exact value, not an overflow.
//! let factorial = (1..=34_i128).fold(Integer::ONE, |acc, k| &acc * &Integer::from(k));
//! assert_eq!(factorial.canonical_lexical(), "295232799039604140847618609643520000000");
//! assert!(factorial.to_i128().is_err());
//!
//! // Any-length lexical forms, exact sums, canonical rendering.
//! let a: Decimal = "0.1000000000000000000000000000000000000001".parse()?;
//! let b: Decimal = "-0.1".parse()?;
//! assert_eq!((&a + &b).canonical_lexical(), "0.0000000000000000000000000000000000000001");
//!
//! // Division under a stated precision policy; the default matches the bounded
//! // machine-word quotient (18 digits, truncated).
//! let third = Decimal::ONE.div(&Decimal::from(3), DivisionPolicy::default())?;
//! assert_eq!(third.canonical_lexical(), "0.333333333333333333");
//! let rounded = Decimal::from(2).div(&Decimal::from(3), DivisionPolicy::scale(5, Rounding::HalfEven))?;
//! assert_eq!(rounded.canonical_lexical(), "0.66667");
//!
//! // The exact quotient, kept exact until it is projected once.
//! let x = Rational::new(Integer::ONE, Integer::from(3_i128))?;
//! let sum = &(&x + &x) + &x;
//! assert_eq!(sum, Rational::ONE);
//! assert!(x.to_decimal(DivisionPolicy::Exact).is_err()); // 1/3 does not terminate
//! # Ok::<(), purrdf_xsd::exact::ExactError>(())
//! ```
//!
//! # Guarantees
//!
//! * **Exact.** Parsing reads lexical forms of any length; canonical rendering is
//!   the XSD 1.1 canonical mapping (byte-identical to the bounded types' wherever
//!   both represent the value); `+ − ×` never round.
//! * **One rounding.** Every conversion to `f64`/`f32` is correctly rounded once,
//!   to nearest with ties to even, subnormals included, by integer arithmetic
//!   alone — no floating-point operation, so the bits are identical on every
//!   target, wasm32 and the x87 included. Every conversion from `f64`/`f32` is
//!   exact.
//! * **Never wraps.** Narrowing to a bounded type ([`Integer::to_i128`],
//!   [`Decimal::to_bounded`], [`Rational::to_bounded`]) returns the value or a
//!   typed [`ExactError::OutOfRange`] carrying the F&O error code; it never
//!   wraps, saturates or rounds.
//! * **Governable.** Every operation has a cost method returning a [`Cost`]
//!   computed in constant time from operand sizes, so a governor can refuse a
//!   hostile operation before it allocates; see [`cost`].
//! * **Deterministic.** No hashing, no floating point, no platform-dependent
//!   width: the same inputs give the same canonical bytes on every target.
//!
//! # Division precision
//!
//! XPath F&O 3.1 §4.2 leaves the precision of an `xs:decimal` quotient
//! implementation-defined. [`DivisionPolicy`] makes it the caller's choice:
//! [`DivisionPolicy::Scale`] rounds to a stated number of fractional digits in a
//! stated [`Rounding`] direction; [`DivisionPolicy::Exact`] returns the exact
//! expansion or refuses a non-terminating quotient with
//! [`ExactError::NonTerminating`]. The default, [`DivisionPolicy::xsd_default`],
//! is eighteen digits truncated toward zero — the quotient the bounded
//! [`crate::numeric::Decimal`] produces wherever its `i128` coefficient holds the
//! result.
//!
//! # Why an exact rational type
//!
//! A reasoned result is often a quotient whose decimal expansion never ends —
//! `1/3`, a probability, a ratio of state counts — and any decimal policy
//! rounds it. Rounding inside a derivation is not reasoning: `(1/3) × 3`
//! computed through an eighteen-digit decimal is `0.999999999999999999`, not
//! `1`, and the error compounds with every step. [`Rational`] keeps the field
//! closed and exact, so a derivation computes on exact values and projects to
//! `xsd:decimal` once, at the end, under a stated policy
//! ([`Rational::to_decimal`]), or is written as an `owl:rational` literal
//! ([`Rational::canonical_lexical`]), the OWL 2 datatype whose value space is
//! exactly this one. The cost is a gcd per operation, which the cost methods
//! charge.
//!

pub mod cost;

mod binary;
mod decimal;
mod error;
mod integer;
mod rational;
mod rounding;

pub use cost::Cost;
pub use decimal::Decimal;
pub use error::{BoundedTarget, ExactError, ExactKind};
pub use integer::Integer;
pub use rational::Rational;
pub use rounding::{DivisionPolicy, Rounding};
