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
    /// Native validation retains its original typed values and cause through
    /// this shared diagnostic; caller-supplied messages remain available.
    Config(purrdf_sparql_eval::NativeDiagnostic),
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
    Data(purrdf_sparql_eval::NativeDiagnostic),
    /// The input source refused a term or iterator read. No partial index is published.
    SourceRead(String),

    /// A fixed-point operation overflowed. The arithmetic is exact by
    /// construction, so an intermediate that does not fit is reported rather
    /// than wrapped or saturated: a wrapped score is a wrong ranking presented
    /// as a right one.
    Overflow(purrdf_sparql_eval::NativeDiagnostic),

    /// A mathematical domain violation — the natural logarithm of a
    /// non-positive value, or a division whose divisor is zero. There is no
    /// value to return, so none is invented.
    Domain(purrdf_sparql_eval::NativeDiagnostic),
}

purrdf_lex::constructors! {
    impl TextError {
        /// Construct a fatal operational source refusal.
        pub fn source_read(what) -> Self::SourceRead;

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
            Self::Config(diagnostic) => write!(
                f,
                "invalid text-index configuration: {}",
                diagnostic.message()
            ),
            Self::Data(diagnostic) => write!(f, "text-index input error: {}", diagnostic.message()),
            Self::SourceRead(msg) => write!(f, "text-index source read error: {msg}"),
            Self::Overflow(diagnostic) | Self::Domain(diagnostic) => {
                f.write_str(diagnostic.message())
            }
        }
    }
}

impl std::error::Error for TextError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Capacity(error) => Some(error),
            Self::Diagnostic(error)
            | Self::Config(error)
            | Self::Data(error)
            | Self::Overflow(error)
            | Self::Domain(error) => Some(error),
            Self::Substring(error) => Some(error),
            Self::Phonetic(error) => Some(error),
            _ => None,
        }
    }
}

impl purrdf_hash::error::StructuredError for TextError {
    purrdf_hash::error_visit! {
        Capacity(error) => "text-capacity" { "cause" => Cause(error) },
        Diagnostic(error) => "text-diagnostic" { "cause" => Cause(error) },
        Substring(error) => "text-substring" { "cause" => Cause(error) },
        Phonetic(error) => "text-phonetic" { "cause" => Cause(error) },
        Config(reason) => "text-config" { "reason" => Cause(reason) },
        Html(diagnostics) => "text-html" { "diagnostics" => List(diagnostics.iter().map(|diagnostic| purrdf_hash::error::ErrorValue::Record(diagnostic))) },
        Data(reason) => "text-data" { "reason" => Cause(reason) },
        SourceRead(message) => "text-source-read" { "message" => Message(message) },
        Overflow(reason) => "text-overflow" { "reason" => Cause(reason) },
        Domain(reason) => "text-domain" { "reason" => Cause(reason) },
    }
}

purrdf_lex::variant_from!(TextError {
    Substring(crate::surface::SubstringRefusal),
    Phonetic(crate::phonetic::PhoneticRefusal),
});

impl TextError {
    /// Preserve a caller-supplied configuration message. Native validation
    /// retains its original values through the same diagnostic owner.
    pub fn config(what: impl core::fmt::Display) -> Self {
        crate::query_workspace::query_error(
            &purrdf_sparql_eval::WorkspaceCapability::resident(),
            purrdf_sparql_eval::NativeDiagnosticKind::Config,
            what,
        )
        .unwrap_or_else(|failure| failure)
    }

    /// Preserve a caller-supplied input-data message.
    pub fn data(what: impl core::fmt::Display) -> Self {
        crate::query_workspace::query_error(
            &purrdf_sparql_eval::WorkspaceCapability::resident(),
            purrdf_sparql_eval::NativeDiagnosticKind::Data,
            what,
        )
        .unwrap_or_else(|failure| failure)
    }

    /// Preserve a caller-supplied overflow message. Native arithmetic producers
    /// retain their own typed refusal instead of calling this constructor.
    pub fn overflow(what: impl core::fmt::Display) -> Self {
        Self::caller_arithmetic(what, false)
    }

    /// Preserve a caller-supplied domain message.
    pub fn domain(what: impl core::fmt::Display) -> Self {
        Self::caller_arithmetic(what, true)
    }

    fn caller_arithmetic(what: impl core::fmt::Display, domain: bool) -> Self {
        let label = if domain {
            "fixed-point domain error"
        } else {
            "fixed-point overflow"
        };
        let diagnostic =
            crate::query_workspace::admitted(purrdf_sparql_eval::NativeDiagnostic::render(
                purrdf_sparql_eval::NativeDiagnosticKind::Function,
                format_args!("{label}: {what}"),
                &purrdf_sparql_eval::WorkspaceCapability::resident(),
            ));
        match diagnostic {
            Ok(diagnostic) => {
                if domain {
                    Self::Domain(diagnostic)
                } else {
                    Self::Overflow(diagnostic)
                }
            }
            Err(error) => error,
        }
    }

    /// Reduce a text-index failure to the evaluator's own channel.
    ///
    /// The mapping is decided here and nowhere else:
    ///
    /// * [`TextError::Config`] retains its original diagnostic with
    ///   [`NativeDiagnosticKind::Config`](purrdf_sparql_eval::NativeDiagnosticKind::Config)
    ///   classification;
    /// * [`TextError::Data`] retains its original diagnostic with
    ///   [`NativeDiagnosticKind::Data`](purrdf_sparql_eval::NativeDiagnosticKind::Data)
    ///   classification;
    /// * [`TextError::Overflow`] and [`TextError::Domain`] are both this
    ///   relation's own computation failing inside a property-function call,
    ///   and the evaluator classifies a callee that could not be invoked as
    ///   written — a relation's own returned `Err` included — as
    ///   [`NativeDiagnosticKind::Function`](purrdf_sparql_eval::NativeDiagnosticKind::Function)
    ///   while retaining the original text error as its native cause.
    ///   Neither is a statement about the caller's configuration or about the
    ///   dataset, so neither borrows those labels.
    ///
    /// Fresh cause and presentation allocations use the supplied query account.
    #[must_use]
    pub fn into_eval_in(
        self,
        workspace: &purrdf_sparql_eval::WorkspaceCapability,
    ) -> purrdf_sparql_eval::EvalError {
        use purrdf_sparql_eval::{EvalError, NativeDiagnostic, NativeDiagnosticKind};
        let kind = match self {
            Self::Capacity(failure) => return failure.into_eval(),
            Self::Diagnostic(diagnostic)
            | Self::Config(diagnostic)
            | Self::Data(diagnostic)
            | Self::Overflow(diagnostic)
            | Self::Domain(diagnostic) => return EvalError::NativeDiagnostic(diagnostic),
            Self::SourceRead(message) => return EvalError::SourceRead(message),
            Self::Substring(_) | Self::Phonetic(_) | Self::Html(_) => NativeDiagnosticKind::Data,
        };
        match NativeDiagnostic::render_cause(kind, self, workspace) {
            Ok(diagnostic) => EvalError::NativeDiagnostic(diagnostic),
            Err(error) => error,
        }
    }
}

impl From<TextError> for purrdf_sparql_eval::EvalError {
    fn from(error: TextError) -> Self {
        error.into_eval_in(&purrdf_sparql_eval::WorkspaceCapability::resident())
    }
}

#[cfg(test)]
mod tests {
    use purrdf_sparql_eval::EvalError;

    use super::TextError;

    #[test]
    fn each_constructor_builds_its_own_variant() {
        assert!(
            matches!(TextError::config("x"), TextError::Config(diagnostic) if diagnostic.message() == "x")
        );
        assert!(
            matches!(TextError::data("x"), TextError::Data(diagnostic) if diagnostic.message() == "x")
        );
        assert!(
            matches!(TextError::overflow("x"), TextError::Overflow(diagnostic) if diagnostic.message() == "fixed-point overflow: x")
        );
        assert!(
            matches!(TextError::domain("x"), TextError::Domain(diagnostic) if diagnostic.message() == "fixed-point domain error: x")
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
                TextError::SourceRead(m) => m.as_str(),
                TextError::Config(diagnostic)
                | TextError::Data(diagnostic)
                | TextError::Overflow(diagnostic)
                | TextError::Domain(diagnostic) => diagnostic.message(),
            };
            assert!(
                rendered.contains(detail),
                "rendering `{rendered}` dropped the detail `{detail}`"
            );
        }
    }

    #[test]
    fn conversion_maps_config_and_data_to_their_namesakes() {
        let config: EvalError = TextError::config("no IRI").into();
        assert!(
            matches!(&config, EvalError::NativeDiagnostic(diagnostic) if diagnostic.kind() == purrdf_sparql_eval::NativeDiagnosticKind::Config),
            "Config must retain its native configuration diagnostic, got {config:?}"
        );

        let data: EvalError = TextError::data("absent predicate").into();
        assert!(
            matches!(&data, EvalError::NativeDiagnostic(diagnostic) if diagnostic.kind() == purrdf_sparql_eval::NativeDiagnosticKind::Data),
            "Data must retain its native data diagnostic, got {data:?}"
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
                matches!(&converted, EvalError::NativeDiagnostic(diagnostic) if diagnostic.kind() == purrdf_sparql_eval::NativeDiagnosticKind::Function),
                "{err:?} must retain the function classification, got {converted:?}"
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
