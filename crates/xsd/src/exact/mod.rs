// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The arbitrary-precision numeric tower: exact `xsd:integer`, `xsd:decimal` and
//! `owl:rational` values of any size, beside the bounded 3.x representations.
//!
//! The crate's default numeric value space is bounded — [`crate::XsdValue::Integer`]
//! holds an `i128` and [`crate::numeric::Decimal`] an `i128` coefficient with at
//! most eighteen fractional digits — and a value outside those bounds is a typed
//! error. This module is the unbounded tower that removes the bounds, available
//! now as standalone types so reasoned mathematics (exact combinatorics,
//! partition functions, exact quotients) can compute on it today, and so the
//! switch of the default representation is a small, separate step.
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
//! // 3.x quotient (18 digits, truncated).
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
//! is eighteen digits truncated toward zero — the quotient the bounded 3.x
//! decimal produces — so switching representations changes no quotient that is
//! representable today.
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
//! # The v4.0 switch
//!
//! The default representation changes in one step that replaces types rather
//! than behaviour, because this module already carries the semantics:
//!
//! | 3.x (bounded, today) | v4.0 |
//! |---|---|
//! | `XsdValue::Integer { value: i128, .. }` | `XsdValue::Integer { value: exact::Integer, .. }` |
//! | `XsdValue::Decimal(numeric::Decimal)` | `XsdValue::Decimal(exact::Decimal)` |
//! | `numeric::Decimal` (`i128`, scale ≤ 18) | removed; `exact::Decimal` takes the name |
//! | `rational::Rational` (`i128` pair) | removed; `exact::Rational` takes the name |
//! | out-of-range parse or result → `XsdError::OutOfRange` | an exact value |
//! | decimal quotient: 18 digits, truncated | [`DivisionPolicy`] on the evaluator, default unchanged |
//! | `BigInt` accumulator for `SUM`/`AVG` | the integer itself |
//!
//! What a v4.0 caller sees: integer and decimal literals of any length parse;
//! arithmetic that overflowed with `err:FOAR0002` returns the exact value;
//! `numeric::Decimal::mantissa()`/`scale()` become [`Decimal::unscaled`] and
//! [`Decimal::scale`]; `i128` integer values are read with
//! [`Integer::to_i128`] (or [`Integer::as_i128`]), which is fallible. Each
//! arithmetic and cast site of the SPARQL evaluator charges the operation's
//! [`Cost`] to its governor first (see [`cost`]). Code that adopts these types
//! in 3.x — through [`Integer::from_xsd`], [`Decimal::from_xsd`],
//! [`Decimal::to_xsd`] and the bounded conversions — needs no change at the
//! switch beyond the type names.

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
