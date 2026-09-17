// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The serde-friendly value types a plan is built from.
//!
//! A plan is pure data that a caller serializes, edits and hands back, so every
//! value it carries must round-trip through a document format and compare for
//! equality. The kernel's `Iri` is a zero-dependency parsed value with neither a
//! `Hash` impl nor a serde impl, and `purrdf-text`'s `Fixed` has no serde impl
//! either; rather than widen those ring-fenced leaves, this module wraps them
//! with the exact behaviour a plan needs:
//!
//! * [`Iri`] validates through the kernel parser, hashes and orders by its
//!   textual content, and serializes as the IRI string — a deserialized IRI is
//!   re-parsed, so a forged document cannot smuggle an unvalidated IRI in.
//! * [`Weight`] wraps `Fixed` and serializes as its raw `i128`, preserving the
//!   exact value across a round trip.
//! * [`Term`] carries a caller's canonical term lexical; the layer mints no
//!   vocabulary and parses no RDF.

use core::fmt;
use core::hash::{Hash, Hasher};

use purrdf_text::Fixed;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::PlanError;

/// A validated IRI in caller-supplied retrieval configuration.
///
/// This is the kernel's parsed IRI, held so the layer can hash it, order it,
/// and serialize it as a string. Construction is fallible and validates through
/// `purrdf_core::parse_iri`; equality, ordering and hashing are all over the
/// IRI's textual content, which is what a plan's identity needs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Iri(purrdf_core::Iri);

impl Iri {
    /// Parse and validate an IRI (RFC 3987).
    ///
    /// # Errors
    ///
    /// [`PlanError::InvalidIri`] when the text is not a valid IRI. Nothing is
    /// guessed or repaired: an invalid IRI in configuration is refused where it
    /// is supplied.
    pub fn parse(text: &str) -> Result<Self, PlanError> {
        purrdf_core::parse_iri(text)
            .map(Self)
            .map_err(|source| PlanError::InvalidIri {
                text: text.to_owned(),
                source,
            })
    }

    /// The IRI's full text, verbatim.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// The underlying kernel IRI.
    #[must_use]
    pub const fn as_kernel_iri(&self) -> &purrdf_core::Iri {
        &self.0
    }
}

impl From<purrdf_core::Iri> for Iri {
    fn from(iri: purrdf_core::Iri) -> Self {
        Self(iri)
    }
}

impl fmt::Display for Iri {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0.as_str())
    }
}

impl Hash for Iri {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.as_str().hash(state);
    }
}

impl PartialOrd for Iri {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Iri {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.0.as_str().cmp(other.0.as_str())
    }
}

impl Serialize for Iri {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.0.as_str())
    }
}

impl<'de> Deserialize<'de> for Iri {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        Self::parse(&text).map_err(serde::de::Error::custom)
    }
}

/// An exact fixed-point stratum weight.
///
/// `Fixed` has no serde impl of its own, and serializing a weight as a decimal
/// string would reintroduce a parsing convention the layer does not need. This
/// newtype serializes the raw `i128` instead, so the value is preserved exactly.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Weight(Fixed);

impl Weight {
    /// Wrap an exact fixed-point value.
    #[must_use]
    pub const fn new(value: Fixed) -> Self {
        Self(value)
    }

    /// Build a weight from its raw scaled integer.
    #[must_use]
    pub const fn from_raw(raw: i128) -> Self {
        Self(Fixed::from_raw(raw))
    }

    /// The raw scaled integer.
    #[must_use]
    pub const fn into_raw(self) -> i128 {
        self.0.into_raw()
    }

    /// The wrapped fixed-point value.
    #[must_use]
    pub const fn fixed(self) -> Fixed {
        self.0
    }

    /// Whether this weight is strictly positive.
    #[must_use]
    pub fn is_positive(self) -> bool {
        self.0 > Fixed::ZERO
    }
}

impl From<Fixed> for Weight {
    fn from(value: Fixed) -> Self {
        Self(value)
    }
}

impl Serialize for Weight {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_i128(self.0.into_raw())
    }
}

impl<'de> Deserialize<'de> for Weight {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = i128::deserialize(deserializer)?;
        Ok(Self::from_raw(raw))
    }
}

/// A caller-supplied RDF term, carried in its canonical lexical form.
///
/// The composition layer neither mints terms nor parses RDF: the caller supplies
/// the term in the canonical form its own term codec produces, and the layer
/// carries it verbatim. It never names a dataset-local `TermId`, so a plan is
/// valid against any dataset.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Term(String);

impl Term {
    /// Carry `text` as a term.
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self(text.into())
    }

    /// The term's canonical text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Term {
    /// The term's canonical text, verbatim — exactly [`Term::as_str`].
    ///
    /// A fused row is printed the moment anybody looks at one, and `{:?}`
    /// renders the newtype (`Term("<http://example.org/a>")`) rather than the
    /// canonical lexical the field carries. Nothing is quoted, escaped or
    /// abbreviated here: the caller supplied the canonical form and this hands
    /// it back unchanged.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<String> for Term {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for Term {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

/// Serde bridge for a field typed `Option<Fixed>`.
///
/// `Fixed` serializes as its raw `i128`, preserving the value exactly; the
/// presence discriminant is serde's own `Option` handling.
pub(crate) mod fixed_option {
    use purrdf_text::Fixed;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    /// Serialize an optional exact fixed-point value as its raw integer.
    ///
    /// The `&Option<Fixed>` signature is fixed by serde's `with` contract; the
    /// idiomatic `Option<&Fixed>` the lint prefers cannot be expressed here.
    #[allow(clippy::ref_option)]
    pub(crate) fn serialize<S: Serializer>(
        value: &Option<Fixed>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        value.map(Fixed::into_raw).serialize(serializer)
    }

    /// Deserialize an optional exact fixed-point value from its raw integer.
    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<Fixed>, D::Error> {
        Ok(Option::<i128>::deserialize(deserializer)?.map(Fixed::from_raw))
    }
}

#[cfg(test)]
mod tests {
    use super::{Iri, Term};

    /// Both printable value types render their text and nothing else, so a row
    /// printed in a host's log carries the canonical form the caller supplied
    /// rather than the newtype wrapped around it.
    #[test]
    fn display_is_exactly_the_underlying_text() {
        let term = Term::new("<http://example.org/doc-alpha>");
        assert_eq!(term.to_string(), term.as_str());
        assert_eq!(term.to_string(), "<http://example.org/doc-alpha>");
        assert_ne!(
            format!("{term:?}"),
            term.to_string(),
            "`Debug` is the newtype spelling, which is what makes `Display` worth having"
        );

        let iri = Iri::parse("http://example.org/stratum/text").expect("a valid IRI");
        assert_eq!(iri.to_string(), iri.as_str());
        assert_eq!(iri.to_string(), "http://example.org/stratum/text");
    }

    /// A term that is not an IRI at all prints unchanged: this layer parses no
    /// RDF and quotes, escapes and abbreviates nothing.
    #[test]
    fn display_neither_quotes_nor_escapes() {
        for text in ["doc-alpha", "\"a literal\"@en", "_:b0", "a b\tc"] {
            assert_eq!(Term::new(text).to_string(), text);
        }
    }
}
