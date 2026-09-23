// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A query embedding's lexical form: the one spelling that reaches a producer.
//!
//! [`RequestTerm::Vector`](crate::RequestTerm::Vector) carries its components as
//! `f32`. A producer receives them through an argument position, and an argument
//! position carries one RDF term — so the components have to be written as one
//! literal's lexical form. [`encode_embedding`] writes that form and
//! [`decode_embedding`] reads it back.
//!
//! # The datatype is the producer's; the lexical form is this layer's
//!
//! PurRDF mints no vocabulary, so the datatype IRI the literal carries is the
//! one the producer's own
//! [`TermPlacement`](purrdf_sparql_eval::TermPlacement) declares — required, and
//! never defaulted, exactly as a geometry's datatype is required. What the
//! producer does **not** supply is the lexical form: the caller handed this layer
//! `Vec<f32>` rather than text, so the layer must choose one spelling and write
//! it. [`decode_embedding`] is therefore part of the seam rather than a
//! convenience: it is how the producer that declared the datatype reads back
//! exactly what was written under it.
//!
//! # Why hex of the bit pattern, and not a decimal
//!
//! The form is the components' IEEE-754 binary32 bit patterns, each as exactly
//! eight upper-case hexadecimal digits, separated by one ASCII space:
//!
//! ```text
//! [0.25, -1.5, 3.0]  ->  "3E800000 BFC00000 40400000"
//! ```
//!
//! Three properties decide it, and each one alone would.
//!
//! * **It is bit-exact.** [`f32::to_bits`] and [`f32::from_bits`] are total
//!   inverses, so the decoded vector is the encoded one — component for
//!   component, bit for bit.
//! * **It is the identity the plan already uses.** A plan's canonical bytes
//!   write each component through the same `to_bits`
//!   ([`Plan::canonical_bytes`](crate::Plan::canonical_bytes)), and
//!   `RequestTerm`'s hand-written `PartialEq` compares by it. So two request
//!   terms are equal exactly when their encodings are equal exactly when their
//!   emitted lexicals are equal. A decimal form would break that in both
//!   directions at once: `0.0` and `-0.0` are distinct bit patterns and one
//!   spelling, and two distinct `NaN` payloads are distinct bit patterns and one
//!   spelling — so two plans with different identities would compile to the same
//!   query.
//! * **It involves no float arithmetic.** Formatting a `u32` as hex is integer
//!   formatting: no rounding mode, no locale, no shortest-round-trip search.
//!   The crate denies `clippy::float_arithmetic` and this form needs none of it.
//!
//! The eight digits are fixed-width, so the separator is a convenience for a
//! reader rather than load-bearing, and no component can be confused with a
//! prefix of the next. Nothing in the form needs escaping in a SPARQL string
//! literal, so the emitted constant is the form itself.
//!
//! # One spelling, and only one
//!
//! [`decode_embedding`] accepts upper-case digits and a single space, because
//! that is what [`encode_embedding`] writes. A lower-case spelling, a padded
//! one, or a comma-separated one is refused rather than read: this codec's two
//! halves are a private conversation between the layer that writes the argument
//! and the producer that reads it, and accepting a spelling the writer never
//! emits would admit text that came from somewhere else.

use core::fmt::Write as _;

/// The number of hexadecimal digits one component is written as: a binary32 bit
/// pattern is 32 bits, which is exactly eight.
const COMPONENT_DIGITS: usize = 8;

/// The byte that separates two components.
const SEPARATOR: char = ' ';

/// Text that is not one embedding in this layer's lexical form.
///
/// Both variants are refusals to guess. A producer that reaches either has been
/// handed a lexical this layer did not write, and reading it as an embedding
/// anyway would hand a nearest-neighbour search a query point nobody asked for.
///
/// The enum is closed rather than `#[non_exhaustive]`, for the reason
/// [`RequestTerm`](crate::RequestTerm) is: a caller routing these refusals wants
/// the compile error when a third one appears, not a wildcard arm that swallows
/// it.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum EmbeddingError {
    /// The text carries no component at all.
    ///
    /// An embedding with no components names no point in any space, and a
    /// request carrying one is refused when it is planned, so this text cannot
    /// have been written by [`encode_embedding`].
    #[error("an embedding lexical carries no components")]
    Empty,

    /// A component is not exactly eight upper-case hexadecimal digits.
    #[error(
        "component {index} of an embedding lexical is {token:?}, which is not \
         eight upper-case hexadecimal digits"
    )]
    MalformedComponent {
        /// The zero-based index of the offending component.
        index: usize,
        /// The text found there, verbatim.
        token: String,
    },
}

/// Write `embedding` as the lexical form this module documents.
///
/// A pure function of the components' bit patterns: the same vector always
/// yields the same string, on every target and in every process.
#[must_use]
pub fn encode_embedding(embedding: &[f32]) -> String {
    // Exact: `COMPONENT_DIGITS` digits per component plus one separator between
    // each adjacent pair, so the string never reallocates.
    let mut out = String::with_capacity(embedding.len() * (COMPONENT_DIGITS + 1));
    for (index, component) in embedding.iter().enumerate() {
        if index > 0 {
            out.push(SEPARATOR);
        }
        // Reading the bit pattern is not arithmetic on the float, and formatting
        // an integer as hex is not arithmetic at all.
        write!(out, "{:0COMPONENT_DIGITS$X}", component.to_bits())
            .expect("writing to a String cannot fail");
    }
    out
}

/// Read a lexical written by [`encode_embedding`] back into its components.
///
/// The components come back bit-for-bit: `decode_embedding(&encode_embedding(v))`
/// is `v`, including a `-0.0` component and a `NaN` payload, because both
/// directions go through [`f32::to_bits`]/[`f32::from_bits`] rather than through
/// a decimal.
///
/// # Errors
///
/// [`EmbeddingError`] when the text is not one embedding in that form; see that
/// type.
pub fn decode_embedding(lexical: &str) -> Result<Vec<f32>, EmbeddingError> {
    if lexical.is_empty() {
        return Err(EmbeddingError::Empty);
    }
    let tokens = lexical.split(SEPARATOR);
    let mut embedding = Vec::with_capacity(lexical.len() / (COMPONENT_DIGITS + 1) + 1);
    for (index, token) in tokens.enumerate() {
        let malformed = || EmbeddingError::MalformedComponent {
            index,
            token: token.to_owned(),
        };
        if token.len() != COMPONENT_DIGITS || !token.bytes().all(is_canonical_hex_digit) {
            return Err(malformed());
        }
        let bits = u32::from_str_radix(token, 16).map_err(|_| malformed())?;
        embedding.push(f32::from_bits(bits));
    }
    Ok(embedding)
}

/// Whether `byte` is one of the sixteen digits this form writes.
///
/// Upper case only: `from_str_radix` would accept `a` as well, and accepting it
/// here would make two lexicals name one embedding.
const fn is_canonical_hex_digit(byte: u8) -> bool {
    byte.is_ascii_digit() || matches!(byte, b'A'..=b'F')
}

#[cfg(test)]
mod tests {
    use super::{EmbeddingError, decode_embedding, encode_embedding};

    /// Every component's bit pattern, which is the identity this form is
    /// exact under.
    fn bits(embedding: &[f32]) -> Vec<u32> {
        embedding.iter().copied().map(f32::to_bits).collect()
    }

    #[test]
    fn the_documented_example_is_the_documented_text() {
        assert_eq!(
            encode_embedding(&[0.25, -1.5, 3.0]),
            "3E800000 BFC00000 40400000"
        );
    }

    /// The property the emitted constant stands on: every component comes back
    /// with the bit pattern it went in with, including the two values a decimal
    /// form collapses.
    #[test]
    fn every_component_round_trips_bit_for_bit() {
        for embedding in [
            vec![0.25_f32],
            vec![0.25, -1.5, 3.0],
            vec![0.0, -0.0],
            vec![f32::MIN_POSITIVE, f32::MAX, f32::MIN],
            // Two distinct `NaN` payloads and one decimal spelling, built from
            // their bits rather than negated: this crate does no float
            // arithmetic, not even to write a test fixture.
            vec![f32::from_bits(0x7FC0_0001), f32::from_bits(0xFFC0_0000)],
            vec![f32::INFINITY, f32::NEG_INFINITY],
        ] {
            let lexical = encode_embedding(&embedding);
            let decoded = decode_embedding(&lexical).expect("the layer's own form decodes");
            assert_eq!(bits(&decoded), bits(&embedding), "round trip of {lexical}");
        }
    }

    /// `0.0` and `-0.0` are one decimal spelling and two bit patterns, so a
    /// decimal form would make two distinct request terms compile to one query.
    #[test]
    fn signed_zeroes_are_two_lexicals() {
        assert_ne!(encode_embedding(&[0.0]), encode_embedding(&[-0.0]));
    }

    #[test]
    fn a_lower_case_spelling_is_refused_but_the_upper_case_one_decodes() {
        let error = decode_embedding("3e800000").expect_err("lower case is not the written form");
        assert_eq!(
            error,
            EmbeddingError::MalformedComponent {
                index: 0,
                token: "3e800000".to_owned(),
            }
        );
        // The neighbouring valid case, one letter apart.
        assert_eq!(decode_embedding("3E800000"), Ok(vec![0.25]));
    }

    #[test]
    fn text_that_is_not_this_form_is_refused_but_a_long_vector_is_not() {
        for text in [
            "3E80000",            // seven digits
            "3E8000000",          // nine digits
            "3E800000  BFC00000", // a doubled separator leaves an empty component
            "3E800000,BFC00000",  // the wrong separator
            "3E800000 ",          // a trailing separator
            "3G800000",           // not a hexadecimal digit
        ] {
            assert!(
                decode_embedding(text).is_err(),
                "{text:?} is not one embedding lexical"
            );
        }
        assert_eq!(decode_embedding(""), Err(EmbeddingError::Empty));
        // The neighbouring valid case: length is not what is being refused.
        let long: Vec<f32> = (0..512_i16).map(f32::from).collect();
        assert_eq!(decode_embedding(&encode_embedding(&long)), Ok(long));
    }
}
