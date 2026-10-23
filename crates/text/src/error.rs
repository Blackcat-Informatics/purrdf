// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

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

/// A refused index delta, retaining the original operands independently of its
/// human-readable presentation.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum TextDeltaError {
    /// The provider has no independent content metadata or source transition.
    SourceObservationUnavailable,
    /// The effective additions do not occur in the supplied next source.
    MissingAddition {
        /// Canonical selected addition ordinal.
        ordinal: usize,
    },
    /// Reversing the delta does not recover the indexed source generation.
    SourceMismatch {
        /// Existing selected-source identity.
        expected: [u8; 32],
        /// Identity reconstructed from the supplied source and delta.
        actual: [u8; 32],
    },
    /// Statement populations implied by the delta disagree with the source,
    /// including nonliteral rows that do not participate in the text digest.
    CoverageMismatch {
        /// The configured predicate whose population disagrees.
        predicate: Box<purrdf_core::TermValue>,
        /// Population in the indexed source.
        before: u128,
        /// Population in the supplied additions.
        added: u128,
        /// Population in the supplied removals.
        removed: u128,
        /// Population independently observed in the next source.
        observed: u128,
    },
    /// The original source, analysis, capacity or ranking refusal.
    Indexing(TextError),
    /// The delta was analyzed under a different resolved law.
    AnalyzerMismatch {
        /// The existing index's analyzer identity.
        expected: [u8; 32],
        /// The supplied delta's analyzer identity.
        actual: [u8; 32],
    },
    /// A supposedly replaced document still occurs in both source tables.
    SourceOverlap {
        /// The overlapping document identifier.
        document: u32,
        /// The original source-literal identity.
        identity: [u8; 32],
    },
    /// The resulting projection dictionary exceeds its address space.
    TermCapacity {
        /// The exact requested number of distinct terms.
        observed: u128,
        /// The largest supported dictionary population.
        limit: u32,
        /// The original integer conversion refusal.
        cause: std::num::TryFromIntError,
    },
}

impl core::fmt::Display for TextDeltaError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::SourceObservationUnavailable => {
                f.write_str("text delta requires independently maintained source content evidence")
            }
            Self::MissingAddition { ordinal } => write!(
                f,
                "text delta addition {ordinal} is absent from the next source"
            ),
            Self::SourceMismatch { expected, actual } => write!(
                f,
                "text delta source mismatch: expected {}, reconstructed {}",
                purrdf_hash::hex::Lower(expected),
                purrdf_hash::hex::Lower(actual)
            ),
            Self::CoverageMismatch {
                predicate,
                before,
                added,
                removed,
                observed,
            } => write!(
                f,
                "text delta coverage mismatch for {predicate:?}: {before} previous, {added} added, {removed} removed, {observed} observed"
            ),
            Self::Indexing(cause) => cause.fmt(f),
            Self::AnalyzerMismatch { expected, actual } => write!(
                f,
                "text delta analyzer mismatch: expected {}, received {}",
                purrdf_hash::hex::Lower(expected),
                purrdf_hash::hex::Lower(actual)
            ),
            Self::SourceOverlap { document, identity } => write!(
                f,
                "replaced document {document} retained source {}",
                purrdf_hash::hex::Lower(identity)
            ),
            Self::TermCapacity {
                observed, limit, ..
            } => write!(
                f,
                "text delta needs {observed} distinct terms; limit is {limit}"
            ),
        }
    }
}

impl std::error::Error for TextDeltaError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Indexing(cause) => Some(cause),
            Self::TermCapacity { cause, .. } => Some(cause),
            Self::AnalyzerMismatch { .. }
            | Self::SourceObservationUnavailable
            | Self::SourceOverlap { .. }
            | Self::MissingAddition { .. }
            | Self::CoverageMismatch { .. }
            | Self::SourceMismatch { .. } => None,
        }
    }
}

purrdf_lex::variant_from!(TextDeltaError {
    Indexing(TextError),
});

/// Admit a dictionary population, widening before adding even at usize::MAX.
pub(crate) fn term_ordinal(current_count: usize) -> Result<u32, TextDeltaError> {
    let observed = current_count as u128 + 1;
    u32::try_from(observed)
        .map(|count| count - 1)
        .map_err(|cause| TextDeltaError::TermCapacity {
            observed,
            limit: u32::MAX,
            cause,
        })
}

/// A failure raised by the text index.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TextError {
    /// Native query workspace refusal, without allocating a diagnostic.
    Capacity(crate::query_workspace::CapacityFailure),
    /// Immutable native query diagnostic retaining its allocation admission.
    Diagnostic(purrdf_sparql_eval::NativeDiagnostic),
    /// Exact substring admission or resource refusal, with generation and counters.
    Substring(crate::surface::SubstringRefusal),
    /// Explicit phonetic-domain or pronunciation refusal.
    Phonetic(crate::phonetic::PhoneticRefusal),
    /// The caller's configuration is not usable as written — an absent
    /// property-function IRI (PurRDF mints none, so there is nothing to fall
    /// back to), an empty set of indexed predicates, or a predicate that is not
    /// an IRI.
    ///
    /// Field parameters and predicate routing are validated by the immutable
    /// [`crate::RankingProfile`], whose identity carries every scoring choice.
    Config(String),
    /// Strict HTML character-reference parse errors with original source ranges.
    Html(Vec<purrdf_lex::html::Diagnostic>),

    /// The input data cannot be indexed or queried as given — a literal whose
    /// datatype does not resolve to an IRI, a corpus larger than the `u32`
    /// document-id space, or a query naming a
    /// document the index does not hold. Distinct from [`TextError::Config`]: the
    /// caller asked a well-formed question of data that does not answer it.
    ///
    /// Data the configuration describes and the dataset does not hold is **not**
    /// this error. A configured predicate the dataset has not interned, and a
    /// named graph it has not interned, each contribute no rows: an index over an
    /// empty corpus is an ordinary operating state, and its emptiness travels as
    /// the answer to a query rather than as a refusal to build. See
    /// [`TextIndex::from_dataset`](crate::TextIndex::from_dataset).
    Data(String),
    /// The input source refused a term or iterator read. No partial index is published.
    SourceRead(String),

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

purrdf_lex::constructors! {
    impl TextError {
        /// Construct a [`TextError::Config`] from any displayable message.
        pub fn config(what) -> Self::Config;

        /// Construct a [`TextError::Data`] from any displayable message.
        pub fn data(what) -> Self::Data;

        /// Construct a fatal operational source refusal.
        pub fn source_read(what) -> Self::SourceRead;

        /// Construct a [`TextError::Overflow`] from any displayable message.
        pub fn overflow(what) -> Self::Overflow;

        /// Construct a [`TextError::Domain`] from any displayable message.
        pub fn domain(what) -> Self::Domain;
    }
}

impl core::fmt::Display for TextError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Capacity(failure) => core::fmt::Display::fmt(failure, f),
            Self::Diagnostic(diagnostic) => core::fmt::Display::fmt(diagnostic, f),
            Self::Substring(refusal) => core::fmt::Display::fmt(refusal, f),
            Self::Phonetic(refusal) => core::fmt::Display::fmt(refusal, f),
            Self::Html(diagnostics) => write!(f, "HTML reference errors: {diagnostics:?}"),
            Self::Config(msg) => write!(f, "invalid text-index configuration: {msg}"),
            Self::Data(msg) => write!(f, "text-index input error: {msg}"),
            Self::SourceRead(msg) => write!(f, "text-index source read error: {msg}"),
            Self::Overflow(msg) => write!(f, "fixed-point overflow: {msg}"),
            Self::Domain(msg) => write!(f, "fixed-point domain error: {msg}"),
        }
    }
}

impl std::error::Error for TextError {}

purrdf_lex::variant_from!(TextError {
    Substring(crate::surface::SubstringRefusal),
    Phonetic(crate::phonetic::PhoneticRefusal),
});

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
            TextError::Capacity(failure) => failure.into_eval(),
            TextError::Diagnostic(diagnostic) => Self::NativeDiagnostic(diagnostic),
            TextError::Substring(refusal) => Self::data(refusal.to_string()),
            TextError::Phonetic(refusal) => Self::data(refusal.to_string()),
            TextError::Html(diagnostics) => {
                Self::data(format!("HTML reference errors: {diagnostics:?}"))
            }
            TextError::Config(msg) => Self::config(msg),
            TextError::Data(msg) => Self::data(msg),
            TextError::SourceRead(msg) => Self::SourceRead(msg),
            TextError::Overflow(msg) => Self::function(format!("fixed-point overflow: {msg}")),
            TextError::Domain(msg) => Self::function(format!("fixed-point domain error: {msg}")),
        }
    }
}

#[cfg(test)]
mod tests {
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
            TextError::source_read("storage checksum mismatch"),
            TextError::overflow("product exceeds i128"),
            TextError::domain("ln of a non-positive value"),
        ] {
            let rendered = err.to_string();
            let detail = match &err {
                TextError::Html(_)
                | TextError::Substring(_)
                | TextError::Phonetic(_)
                | TextError::Capacity(_)
                | TextError::Diagnostic(_) => {
                    unreachable!("test uses string diagnostic variants")
                }
                TextError::Config(m)
                | TextError::SourceRead(m)
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

    #[test]
    fn source_read_failure_remains_fatal_and_preserves_its_detail() {
        let converted: EvalError = TextError::source_read("storage checksum mismatch").into();
        assert!(
            matches!(&converted, EvalError::SourceRead(message) if message == "storage checksum mismatch")
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
