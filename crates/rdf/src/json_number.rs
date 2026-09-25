// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Every JSON number this crate reads becomes the binary64 nearest its decimal value.
//!
//! A JSON number read by `serde_json` reaches this crate's output as an `f64`: JSON-LD
//! writes a non-integral number as the canonical `xsd:double` lexical of that value and an
//! `@json` value as its serialization, CSVW and OKF write it as a literal's lexical form, and
//! the rdf:JSON canonical checks compare its serialization with the literal's bytes. So the
//! `f64` must be the one the decimal spells, on every target.
//!
//! Two things make it so. The workspace enables `serde_json`'s `float_roundtrip`, whose
//! reader is correctly rounded. (The default one scales a `u64` significand by a binary64
//! power of ten, rounding at each step and dropping every digit past the nineteenth, and
//! lands on a neighbour of the right value for many decimals of seventeen digits or more:
//! `122.416294033786585` reads as `122.4162940337866`.) And that reader's exact fast path
//! -- a significand and a power of ten that are both binary64 values, multiplied or
//! divided once -- is exact only if the operation rounds once: on the x87 it rounds to the
//! register's 64 bits and again to 53, and `8.64759627780072e32` reads as its successor.
//! [`read_json`] runs the read inside a [`Binary64Scope`], which sets the x87's precision
//! control to 53 bits for its duration; everywhere else the scope is empty and the read
//! is the bare call.
//!
//! The scope covers the whole `serde_json` call because the number is converted inside
//! the tokenizer, where no caller can stand; entering it is one control-word load per
//! document, not per number.

use purrdf_xsd::ieee::Binary64Scope;

/// Run a `serde_json` read whose numbers reach a literal, serialized bytes or an identity,
/// with every binary64 operation inside it rounded once.
#[inline]
pub(crate) fn read_json<T>(read: impl FnOnce() -> T) -> T {
    let _binary64 = Binary64Scope::enter();
    read()
}
