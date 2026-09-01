// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The text index's typed error channel.
//!
//! Per the repository's hard-fail doctrine, every condition that is not a valid
//! in-scope result is a typed error: there is no lenient mode, no silently
//! dropped document, and no score computed from a value the arithmetic could not
//! actually represent. A ranked answer that quietly omitted a row it could not
//! score would be indistinguishable from a ranked answer that legitimately did
//! not match it, so the omission is refused instead.
//!
//! At the evaluator boundary a [`TextError`] is reduced to a
//! [`purrdf_sparql_eval::EvalError`] by the [`From`] impl at the bottom of this
//! module — that conversion is the single place the mapping is decided, so the
//! same failure never reaches a caller wearing two different labels.

/// A failure raised by the text index.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TextError {
    /// The caller's configuration is not usable as written — an absent
    /// property-function IRI (PurRDF mints none, so there is nothing to fall
    /// back to), an empty set of indexed predicates, or a BM25 parameter
    /// outside its defined range.
    Config(String),

    /// The input data cannot be indexed or queried as given — a predicate the
    /// configuration names that the dataset does not carry, or a term the index
    /// cannot encode (a triple term nested past the encoder's depth bound).
    /// Distinct from [`TextError::Config`]: the caller asked a well-formed
    /// question of data that does not answer it.
    Data(String),

    /// A fixed-point operation overflowed. The arithmetic is exact by
    /// construction, so an intermediate that does not fit is reported rather
    /// than wrapped or saturated: a wrapped score is a wrong ranking presented
    /// as a right one.
    Overflow(String),

    /// A mathematical domain violation — the natural logarithm of a
    /// non-positive value, or a division whose divisor is zero. There is no
    /// value to return, so none is invented.
    Domain(String),
}

impl TextError {
    /// Construct a [`TextError::Config`] from any displayable message.
    pub fn config(what: impl Into<String>) -> Self {
        Self::Config(what.into())
    }

    /// Construct a [`TextError::Data`] from any displayable message.
    pub fn data(what: impl Into<String>) -> Self {
        Self::Data(what.into())
    }

    /// Construct a [`TextError::Overflow`] from any displayable message.
    pub fn overflow(what: impl Into<String>) -> Self {
        Self::Overflow(what.into())
    }

    /// Construct a [`TextError::Domain`] from any displayable message.
    pub fn domain(what: impl Into<String>) -> Self {
        Self::Domain(what.into())
    }
}

impl core::fmt::Display for TextError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Config(msg) => write!(f, "invalid text-index configuration: {msg}"),
            Self::Data(msg) => write!(f, "text-index input error: {msg}"),
            Self::Overflow(msg) => write!(f, "fixed-point overflow: {msg}"),
            Self::Domain(msg) => write!(f, "fixed-point domain error: {msg}"),
        }
    }
}

impl std::error::Error for TextError {}

impl From<TextError> for purrdf_sparql_eval::EvalError {
    /// Reduce a text-index failure to the evaluator's own channel.
    ///
    /// The mapping is decided here and nowhere else:
    ///
    /// * [`TextError::Config`] is the caller's evaluation configuration, which
    ///   is exactly what [`EvalError::Config`](purrdf_sparql_eval::EvalError::Config)
    ///   names;
    /// * [`TextError::Data`] is the dataset failing to answer a well-formed
    ///   question, which is
    ///   [`EvalError::Data`](purrdf_sparql_eval::EvalError::Data);
    /// * [`TextError::Overflow`] and [`TextError::Domain`] are both this
    ///   relation's own computation failing inside a property-function call,
    ///   and the evaluator classifies a callee that could not be invoked as
    ///   written — a relation's own returned `Err` included — as
    ///   [`EvalError::Function`](purrdf_sparql_eval::EvalError::Function).
    ///   Neither is a statement about the caller's configuration or about the
    ///   dataset, so neither borrows those labels.
    fn from(err: TextError) -> Self {
        match err {
            TextError::Config(msg) => Self::config(msg),
            TextError::Data(msg) => Self::data(msg),
            TextError::Overflow(msg) => Self::function(format!("fixed-point overflow: {msg}")),
            TextError::Domain(msg) => Self::function(format!("fixed-point domain error: {msg}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use purrdf_sparql_eval::EvalError;

    use super::TextError;

    #[test]
    fn each_constructor_builds_its_own_variant() {
        assert_eq!(
            TextError::config("x"),
            TextError::Config("x".to_owned()),
            "config() must build Config"
        );
        assert_eq!(
            TextError::data("x"),
            TextError::Data("x".to_owned()),
            "data() must build Data"
        );
        assert_eq!(
            TextError::overflow("x"),
            TextError::Overflow("x".to_owned()),
            "overflow() must build Overflow"
        );
        assert_eq!(
            TextError::domain("x"),
            TextError::Domain("x".to_owned()),
            "domain() must build Domain"
        );
    }

    /// Every rendering names the detail it was given, so a diagnostic never
    /// loses the one piece of information a reader needs.
    #[test]
    fn display_carries_the_detail() {
        for err in [
            TextError::config("no predicate IRI supplied"),
            TextError::data("predicate absent from the dataset"),
            TextError::overflow("product exceeds i128"),
            TextError::domain("ln of a non-positive value"),
        ] {
            let rendered = err.to_string();
            let detail = match &err {
                TextError::Config(m)
                | TextError::Data(m)
                | TextError::Overflow(m)
                | TextError::Domain(m) => m.clone(),
            };
            assert!(
                rendered.contains(&detail),
                "rendering `{rendered}` dropped the detail `{detail}`"
            );
        }
    }

    #[test]
    fn conversion_maps_config_and_data_to_their_namesakes() {
        let config: EvalError = TextError::config("no IRI").into();
        assert!(
            matches!(config, EvalError::Config(_)),
            "Config must map to EvalError::Config, got {config:?}"
        );

        let data: EvalError = TextError::data("absent predicate").into();
        assert!(
            matches!(data, EvalError::Data(_)),
            "Data must map to EvalError::Data, got {data:?}"
        );
    }

    /// The arithmetic failures are the relation's own computation failing
    /// mid-call, which the evaluator classifies as a host-function error — not
    /// as bad data and not as bad configuration.
    #[test]
    fn conversion_maps_arithmetic_failures_to_function() {
        for err in [
            TextError::overflow("product exceeds i128"),
            TextError::domain("ln of a non-positive value"),
        ] {
            let converted: EvalError = err.clone().into();
            assert!(
                matches!(converted, EvalError::Function(_)),
                "{err:?} must map to EvalError::Function, got {converted:?}"
            );
        }
    }

    /// The detail survives the conversion: a caller reading the evaluator's
    /// message still learns which arithmetic failure occurred.
    #[test]
    fn conversion_preserves_the_detail() {
        let converted: EvalError = TextError::overflow("product exceeds i128").into();
        assert!(
            converted.to_string().contains("product exceeds i128"),
            "conversion dropped the detail: {converted}"
        );
    }
}
