// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The one failure channel this crate has, and the single site that maps it onto
//! the evaluator's.
//!
//! Every refusal in `purrdf-geo` reduces to one of four kinds, and each kind
//! answers a different question about *whose* mistake it was:
//!
//! * [`GeoError::Config`] — the **caller's wiring** is unusable as written: a
//!   vocabulary with an empty IRI, a registration missing the datatype it is
//!   supposed to recognize. Nothing about the data or the query is wrong yet.
//! * [`GeoError::Literal`] — a **geometry literal** is not the thing its datatype
//!   claims. A malformed WKT token, a GeoJSON object with no `type`, a ring that
//!   does not close. This is bad input, and it is refused rather than repaired,
//!   because a repaired geometry answers a question nobody asked.
//! * [`GeoError::Unsupported`] — the function is **spec-defined and genuinely not
//!   implemented here**. This is the one variant that is a statement about
//!   `purrdf-geo` rather than about its inputs, and it exists so that the gap is
//!   *loud*. A `geof:` call this crate cannot answer must abort the query; it must
//!   never return a plausible-looking wrong geometry, and it must never return
//!   `false` — a topological predicate that answers `false` because it was not
//!   implemented is indistinguishable from one that answers `false` because the
//!   geometries genuinely do not relate, and that is the silent-wrong-answer
//!   channel this crate exists to keep closed.
//! * [`GeoError::Domain`] — the arguments are individually well-formed but the
//!   operation is undefined on them: two geometries in different coordinate
//!   reference systems (this crate reprojects nothing, so it refuses rather than
//!   pretending the coordinates are comparable), a measure asked of an empty
//!   geometry, an exponent past the parser's cap.
//!
//! # Why the mapping onto [`EvalError`] lives here and only here
//!
//! A failure that reaches a caller wearing two different labels depending on
//! which call path produced it is a diagnostic that cannot be relied on. So the
//! `From<GeoError> for EvalError` impl below is *the* mapping site: no other
//! module in this crate constructs an [`EvalError`] directly, and every seam
//! (scalar function, relation, cursor) reduces its failure to a [`GeoError`]
//! first.

use purrdf_sparql_eval::EvalError;

/// Why a `purrdf-geo` operation refused.
///
/// See the module docs for what distinguishes the four kinds; the split is by
/// *whose mistake it was*, not by which function raised it.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum GeoError {
    /// The caller's wiring is unusable as written (an empty IRI, a vocabulary
    /// missing a term a registered function needs).
    Config(String),
    /// A geometry literal is not well-formed for its datatype.
    Literal(String),
    /// A spec-defined operation this crate does not implement. Always loud,
    /// never a default answer.
    Unsupported(String),
    /// Well-formed arguments on which the operation is undefined (mixed
    /// coordinate reference systems, a measure of an empty geometry).
    Domain(String),
}

impl GeoError {
    /// A [`GeoError::Config`] with `what` as its detail.
    pub fn config(what: impl Into<String>) -> Self {
        Self::Config(what.into())
    }

    /// A [`GeoError::Literal`] with `what` as its detail.
    pub fn literal(what: impl Into<String>) -> Self {
        Self::Literal(what.into())
    }

    /// A [`GeoError::Unsupported`] with `what` as its detail.
    pub fn unsupported(what: impl Into<String>) -> Self {
        Self::Unsupported(what.into())
    }

    /// A [`GeoError::Domain`] with `what` as its detail.
    pub fn domain(what: impl Into<String>) -> Self {
        Self::Domain(what.into())
    }

    /// The detail message, without this error's own prefix.
    #[must_use]
    pub fn detail(&self) -> &str {
        match self {
            Self::Config(msg) | Self::Literal(msg) | Self::Unsupported(msg) | Self::Domain(msg) => {
                msg
            }
        }
    }
}

impl core::fmt::Display for GeoError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Config(msg) => write!(f, "invalid geo configuration: {msg}"),
            Self::Literal(msg) => write!(f, "malformed geometry literal: {msg}"),
            Self::Unsupported(msg) => write!(f, "unsupported GeoSPARQL operation: {msg}"),
            Self::Domain(msg) => write!(f, "geometry domain error: {msg}"),
        }
    }
}

impl std::error::Error for GeoError {}

impl From<GeoError> for EvalError {
    /// The single site that decides which evaluator label a geo failure wears.
    ///
    /// * `Config` keeps its name: it is a statement about the host's wiring, which
    ///   is exactly what [`EvalError::config`] means.
    /// * `Literal` becomes [`EvalError::data`]: a geometry literal is *dataset
    ///   content*, and a malformed one is bad data rather than a bad query or a
    ///   bad registration — even when it arrives as a constant written in the
    ///   query text, because the same lexical form would have been equally
    ///   malformed had it come from a triple.
    /// * `Unsupported` and `Domain` become [`EvalError::function`], which is the
    ///   evaluator's label for "the callee could not be invoked as written". The
    ///   message is re-prefixed because `EvalError::function` supplies its own
    ///   framing rather than this type's `Display`.
    fn from(err: GeoError) -> Self {
        match err {
            GeoError::Config(msg) => Self::config(msg),
            GeoError::Literal(msg) => Self::data(format!("malformed geometry literal: {msg}")),
            GeoError::Unsupported(msg) => {
                Self::function(format!("unsupported GeoSPARQL operation: {msg}"))
            }
            GeoError::Domain(msg) => Self::function(format!("geometry domain error: {msg}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::GeoError;
    use purrdf_sparql_eval::EvalError;

    #[test]
    fn each_constructor_builds_its_own_variant() {
        assert!(matches!(GeoError::config("a"), GeoError::Config(_)));
        assert!(matches!(GeoError::literal("b"), GeoError::Literal(_)));
        assert!(matches!(
            GeoError::unsupported("c"),
            GeoError::Unsupported(_)
        ));
        assert!(matches!(GeoError::domain("d"), GeoError::Domain(_)));
    }

    #[test]
    fn display_carries_the_detail_and_its_own_prefix() {
        for error in [
            GeoError::config("the detail"),
            GeoError::literal("the detail"),
            GeoError::unsupported("the detail"),
            GeoError::domain("the detail"),
        ] {
            let rendered = error.to_string();
            assert!(
                rendered.contains("the detail"),
                "the detail must survive rendering: {rendered}"
            );
            assert_eq!(error.detail(), "the detail", "detail() strips the prefix");
            assert!(
                rendered.len() > "the detail".len(),
                "a prefix names the kind: {rendered}"
            );
        }
    }

    #[test]
    fn conversion_maps_config_to_its_namesake_and_a_literal_to_data() {
        assert!(matches!(
            EvalError::from(GeoError::config("x")),
            EvalError::Config(_)
        ));
        assert!(matches!(
            EvalError::from(GeoError::literal("x")),
            EvalError::Data(_)
        ));
    }

    /// An unimplemented operation must reach the caller as a hard failure, never
    /// as a default answer: a `false` from an unimplemented predicate is
    /// indistinguishable from an honest `false`.
    #[test]
    fn conversion_maps_unsupported_and_domain_to_function() {
        assert!(matches!(
            EvalError::from(GeoError::unsupported("geof:transform")),
            EvalError::Function(_)
        ));
        assert!(matches!(
            EvalError::from(GeoError::domain("mixed CRS")),
            EvalError::Function(_)
        ));
    }

    #[test]
    fn conversion_preserves_the_detail() {
        for error in [
            GeoError::config("the detail"),
            GeoError::literal("the detail"),
            GeoError::unsupported("the detail"),
            GeoError::domain("the detail"),
        ] {
            let rendered = EvalError::from(error).to_string();
            assert!(
                rendered.contains("the detail"),
                "the detail must survive the conversion: {rendered}"
            );
        }
    }
}
