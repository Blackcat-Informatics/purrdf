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
//! Arithmetic refusals retain their native operands and allocation owners across
//! clones and evaluator conversion. Their equality is retained owner identity.
//! The v4 interface removes string-only arithmetic constructors and `Hash`;
//! inspection uses the borrowed structured error protocol.
//!
//! At the evaluator boundary a [`TextError`] is converted to a
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

impl purrdf_hash::error::StructuredError for TextDeltaError {
    purrdf_hash::error_visit! {
        SourceObservationUnavailable => "text-delta-source-observation-unavailable" {},
        MissingAddition { ordinal } => "text-delta-missing-addition" {
            "ordinal" => Unsigned(*ordinal as u128),
        },
        SourceMismatch { expected, actual } => "text-delta-source-mismatch" {
            "expected" => Bytes(expected), "actual" => Bytes(actual),
        },
        CoverageMismatch { predicate, before, added, removed, observed } => "text-delta-coverage-mismatch" {
            "predicate" => Term(&**predicate),
            "before" => Unsigned(*before), "added" => Unsigned(*added),
            "removed" => Unsigned(*removed), "observed" => Unsigned(*observed),
        },
        Indexing(cause) => "text-delta-indexing" { "cause" => Cause(cause) },
        AnalyzerMismatch { expected, actual } => "text-delta-analyzer-mismatch" {
            "expected" => Bytes(expected), "actual" => Bytes(actual),
        },
        SourceOverlap { document, identity } => "text-delta-source-overlap" {
            "document" => Unsigned(u128::from(*document)), "identity" => Bytes(identity),
        },
        TermCapacity { observed, limit, cause } => "text-delta-term-capacity" {
            "observed" => Unsigned(*observed), "limit" => Unsigned(u128::from(*limit)),
            "cause" => Cause(cause),
        },
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
///
/// [`purrdf_hash::error::StructuredError`] exposes every retained field without
/// rendering. Native causes and ordered HTML diagnostics remain borrowed;
/// message variants report their existing stored text.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum TextError {
    /// Native query workspace refusal, without allocating a diagnostic.
    Capacity(crate::query_workspace::CapacityFailure),
    /// Immutable native query diagnostic retaining its allocation admission.
    Diagnostic(purrdf_sparql_eval::NativeDiagnostic),
    /// An original refusal retained under its query allocation account.
    RetainedDiagnostic(purrdf_sparql_eval::RetainedNativeFailure),
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
    Config(purrdf_sparql_eval::RetainedNativeFailure),
    /// Strict HTML character-reference parse errors with original source ranges.
    Html(purrdf_sparql_eval::RetainedNativeFailure),

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
    SourceRead(purrdf_sparql_eval::RetainedNativeFailure),

    /// A fixed-point operation overflowed. The arithmetic is exact by
    /// construction, so an intermediate that does not fit is reported rather
    /// than wrapped or saturated: a wrapped score is a wrong ranking presented
    /// as a right one.
    Overflow(purrdf_sparql_eval::RetainedNativeFailure),

    /// A mathematical domain violation — the natural logarithm of a
    /// non-positive value, or a division whose divisor is zero. There is no
    /// value to return, so none is invented.
    Domain(purrdf_sparql_eval::RetainedNativeFailure),
}

purrdf_lex::constructors! {
    impl TextError {
        /// Construct a [`TextError::Data`] from any displayable message.
        pub fn data(what) -> Self::Data;
    }
}

impl TextError {
    /// Retain an explicit caller-provided configuration message.
    pub fn config(what: impl core::fmt::Display) -> Self {
        crate::configuration::ConfigurationRefusal::CallerMessage(what.to_string()).into_error()
    }

    /// Retain ordered HTML-reference diagnostics under resident admission.
    pub fn html(diagnostics: Vec<purrdf_lex::html::Diagnostic>) -> Self {
        Self::html_in(
            diagnostics,
            None,
            &purrdf_sparql_eval::WorkspaceCapability::resident(),
        )
    }

    pub(crate) fn html_in(
        diagnostics: Vec<purrdf_lex::html::Diagnostic>,
        allocation: Option<purrdf_sparql_eval::WorkspaceAllocation>,
        workspace: &purrdf_sparql_eval::WorkspaceCapability,
    ) -> Self {
        let failure = purrdf_sparql_eval::RetainedNativeFailure::error(
            purrdf_sparql_eval::NativeDiagnosticKind::Data,
            HtmlFailure {
                diagnostics,
                _allocation: allocation,
            },
            workspace,
        );
        match failure {
            purrdf_sparql_eval::EvalError::RetainedNativeFailure(failure)
                if workspace.is_bounded() =>
            {
                Self::RetainedDiagnostic(failure)
            }
            purrdf_sparql_eval::EvalError::RetainedNativeFailure(failure) => Self::Html(failure),
            error => Self::Capacity(crate::CapacityFailure::from_workspace(&error)),
        }
    }

    /// Retain an original dataset read failure without rendering or cloning it.
    /// Construction APIs have no query workspace; the resident native owner
    /// preserves any allocation ownership already carried by the provider error.
    pub fn source_read(cause: impl purrdf_hash::error::NativeError + 'static) -> Self {
        crate::query_workspace::source_error(cause)
    }
}

impl core::fmt::Display for TextError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Capacity(failure) => core::fmt::Display::fmt(failure, f),
            Self::Diagnostic(diagnostic) => core::fmt::Display::fmt(diagnostic, f),
            Self::RetainedDiagnostic(failure) => core::fmt::Display::fmt(failure, f),
            Self::Substring(refusal) => core::fmt::Display::fmt(refusal, f),
            Self::Phonetic(refusal) => core::fmt::Display::fmt(refusal, f),
            Self::Html(failure) => core::fmt::Display::fmt(failure.cause(), f),
            Self::Config(failure) => {
                write!(f, "invalid text-index configuration: {}", failure.cause())
            }
            Self::Data(msg) => write!(f, "text-index input error: {msg}"),
            Self::SourceRead(failure) => {
                write!(f, "text-index source read error: {}", failure.cause())
            }
            Self::Overflow(failure) | Self::Domain(failure) => {
                core::fmt::Display::fmt(failure.cause(), f)
            }
        }
    }
}

purrdf_hash::error_sources!(
    TextError,
    Capacity,
    Diagnostic,
    RetainedDiagnostic,
    Html,
    Config,
    SourceRead,
    Substring,
    Phonetic,
    Overflow,
    Domain
);

impl purrdf_hash::error::StructuredError for TextError {
    purrdf_hash::error_visit! {
        Capacity(cause) => "text-capacity" { "cause" => Cause(cause) },
        Diagnostic(cause) => "text-diagnostic" { "cause" => Cause(cause) },
        RetainedDiagnostic(cause) => "text-retained-diagnostic" { "cause" => Cause(cause) },
        Substring(cause) => "text-substring" { "cause" => Cause(cause) },
        Phonetic(cause) => "text-phonetic" { "cause" => Cause(cause) },
        Config(cause) => "text-config" { "cause" => Cause(cause) },
        Html(cause) => "text-html" { "cause" => Cause(cause) },
        Data(message) => "text-data" { "message" => Text(message) },
        SourceRead(cause) => "text-source-read" { "cause" => Cause(cause) },
        Overflow(cause) => "text-overflow" { "cause" => Cause(cause) },
        Domain(cause) => "text-domain" { "cause" => Cause(cause) },
    }
}

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
    ///   retains the native Config classification through evaluator conversion.
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
            TextError::Html(failure) => Self::RetainedNativeFailure(failure),
            TextError::Config(failure) => Self::RetainedNativeFailure(failure),
            TextError::Data(msg) => Self::data(msg),
            TextError::SourceRead(failure) => Self::RetainedNativeFailure(failure),
            TextError::Overflow(failure)
            | TextError::Domain(failure)
            | TextError::RetainedDiagnostic(failure) => Self::RetainedNativeFailure(failure),
        }
    }
}

/// The diagnostic payload dies before the original allocation grant.
#[derive(Debug)]
struct HtmlFailure {
    diagnostics: Vec<purrdf_lex::html::Diagnostic>,
    _allocation: Option<purrdf_sparql_eval::WorkspaceAllocation>,
}

impl core::fmt::Display for HtmlFailure {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(formatter, "HTML reference errors: {:?}", self.diagnostics)
    }
}

impl std::error::Error for HtmlFailure {}

impl purrdf_hash::error::StructuredError for HtmlFailure {
    purrdf_hash::error_visit! {
        @record self { diagnostics, _allocation } => "text-html-diagnostics" {
            "diagnostics" => List(self.diagnostics.iter().map(|value| purrdf_hash::error::ErrorValue::Cause(value))),
        }
    }
}

#[cfg(test)]
mod tests {
    use purrdf_sparql_eval::EvalError;

    use super::TextError;

    #[global_allocator]
    static GLOBAL: purrdf_alloc_probe::CountingAllocator = purrdf_alloc_probe::CountingAllocator;

    #[test]
    fn bounded_html_failure_retains_diagnostics_and_refunds_original_grants() {
        use purrdf_core::{
            SegmentedBuildLimits, SegmentedBuilder, SegmentedReadLimits, SparqlRequest,
        };
        use purrdf_sparql_eval::{
            Arity, ExtensionEnv, NativeSparqlEngine, QueryOptions, UserFunctionRegistry,
        };
        use std::sync::{Arc, Mutex};
        let image =
            SegmentedBuilder::new(SegmentedBuildLimits::new(4096, 500_000, 4096, 512, 4).unwrap())
                .unwrap()
                .seal()
                .unwrap();
        let focus = purrdf_core::RdfDatasetBuilder::new().freeze().unwrap();
        for stop in [false, true] {
            let source = image
                .open_session(SegmentedReadLimits::new(
                    8_000_000, 2, 16_384, 20_000_000, 8,
                ))
                .unwrap();
            let live = || {
                let evidence = source.evidence();
                evidence.live_bytes()
                    - evidence.unpinned_cache_bytes()
                    - evidence.bookkeeping_bytes()
            };
            let baseline = live();
            let slot = Arc::new(Mutex::new(None));
            let saved = Arc::clone(&slot);
            let mut functions = UserFunctionRegistry::default();
            functions.register_expr_admitted(
                "http://example.org/capture-html",
                Arity::Exact(0),
                Arc::new(move |_, capability| {
                    *saved.lock().unwrap() = Some(capability.clone());
                    Ok(None)
                }),
            );
            let env = ExtensionEnv::empty();
            let engine = NativeSparqlEngine::new();
            let functions = engine.bind_functions(functions, env).unwrap();
            let result = engine
                .query_fallible_view(
                    &source,
                    SparqlRequest {
                        query: "SELECT (<http://example.org/capture-html>() AS ?value) WHERE {}",
                        base_iri: None,
                        substitutions: &[],
                    },
                    &QueryOptions::new()
                        .with_env(env)
                        .with_functions(&functions)
                        .with_focus_graph(Some(&focus)),
                )
                .unwrap();
            drop(result);
            drop(functions);
            drop(engine);
            let capability = slot.lock().unwrap().take().unwrap();
            assert!(capability.is_bounded());
            let controls = live();
            if stop {
                let mut diagnostics = Vec::new();
                let mut allocation = None;
                capability
                    .reserve_vec(&mut diagnostics, &mut allocation, 1)
                    .unwrap();
                diagnostics.push(purrdf_lex::html::Diagnostic {
                    kind: purrdf_lex::html::ErrorKind::NullCharacterReference,
                    source: 0..4,
                });
                assert!(live() > controls);
                assert!(matches!(
                    capability.charge(8_000_000),
                    Err(EvalError::WorkspaceStopped)
                ));
                let error = TextError::html_in(diagnostics, allocation, &capability);
                assert!(matches!(
                    error,
                    TextError::Capacity(crate::CapacityFailure::Stopped)
                ));
                assert_eq!(
                    live(),
                    controls,
                    "failed shared admission refunds the diagnostic vector"
                );
                drop(capability);
                assert_eq!(live(), baseline);
                continue;
            }
            let analyzer = crate::Analyzer::with_profile(
                crate::AnalyzerProfile::empty_lexicon().with_input_mode(crate::InputMode::HtmlText),
            )
            .unwrap();
            let input = String::from("é&#0; &nope;");
            assert_eq!(
                &*analyzer
                    .analysis_form_owned("e&#x301;", &capability)
                    .unwrap(),
                "e"
            );
            let window = purrdf_alloc_probe::CurrentThreadWindow::open();
            let error = analyzer.terms_owned(&input, &capability).unwrap_err();
            let retained_bytes = window.close().retained_bytes;
            drop(input);
            drop(analyzer);
            let held = live();
            assert_eq!(retained_bytes, i64::try_from(held - controls).unwrap());
            let TextError::RetainedDiagnostic(original) = &error else {
                panic!("bounded HTML keeps evaluator presentation and original owner");
            };
            let native = std::error::Error::source(original)
                .unwrap()
                .downcast_ref::<super::HtmlFailure>()
                .unwrap();
            assert_eq!(native.diagnostics.len(), 2);
            let pointer = native.diagnostics.as_ptr();
            let expected =
                EvalError::data(format!("HTML reference errors: {:?}", native.diagnostics))
                    .to_string();
            assert_eq!(error.to_string(), expected);
            let buffer = Vec::with_capacity(4);
            let window = purrdf_alloc_probe::CurrentThreadWindow::open();
            let copy = error.clone();
            let converted: EvalError = error.into();
            let EvalError::RetainedNativeFailure(retained) = &converted else {
                panic!("native Data owner");
            };
            let fields =
                purrdf_testkit::error_capture::Capture::read_with_buffer(retained.cause(), buffer);
            assert_eq!(window.close().allocations, 0);
            assert_eq!(fields.code, "text-html-diagnostics");
            let native = std::error::Error::source(retained)
                .unwrap()
                .downcast_ref::<super::HtmlFailure>()
                .unwrap();
            assert_eq!(native.diagnostics.as_ptr(), pointer);
            assert_eq!(
                retained.kind(),
                purrdf_sparql_eval::NativeDiagnosticKind::Data
            );
            drop(fields);
            drop(converted);
            drop(capability);
            assert_eq!(
                live(),
                held,
                "only diagnostic storage and its retained owner remain"
            );
            drop(copy);
            assert_eq!(live(), baseline, "last clone releases the original grant");
        }
    }

    #[test]
    fn native_workspace_layout_refusal_is_inspectable_without_allocation() {
        use purrdf_hash::error::ErrorValue;
        use purrdf_testkit::error_capture::Capture;
        use std::error::Error;

        let workspace = purrdf_sparql_eval::WorkspaceCapability::resident();
        let mut values = Vec::<u64>::new();
        let mut allocation = None;
        let root = Vec::with_capacity(1);
        let fields = Vec::new();
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        let error = crate::query_workspace::admitted(workspace.reserve_vec(
            &mut values,
            &mut allocation,
            usize::MAX,
        ))
        .unwrap_err();
        let root = Capture::read_with_buffer(&error, root);
        let ErrorValue::Cause(cause) = root.field("cause") else {
            panic!("the original workspace refusal remains borrowed");
        };
        let fields = Capture::read_with_buffer(cause, fields);
        let measured = window.close();
        assert_eq!(measured.allocations, 0);
        assert_eq!(measured.requested_bytes, 0);
        assert_eq!(root.code, "text-capacity");
        assert_eq!(fields.code, "text-capacity-layout-overflow");
        assert!(
            error
                .source()
                .unwrap()
                .downcast_ref::<crate::CapacityFailure>()
                .is_some()
        );
        assert_eq!(values, []);
        assert!(allocation.is_none());
        crate::query_workspace::admitted(workspace.reserve_vec(&mut values, &mut allocation, 1))
            .unwrap();
        assert!(values.capacity() >= 1);
    }

    #[test]
    fn dictionary_population_refusal_keeps_widened_count_and_original_cause() {
        use purrdf_hash::error::ErrorValue;
        use purrdf_testkit::error_capture::Capture;
        use std::error::Error;

        let error = super::term_ordinal(u32::MAX as usize).unwrap_err();
        let buffer = Vec::with_capacity(3);
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        let fields = Capture::read_with_buffer(&error, buffer);
        let measured = window.close();
        assert_eq!(measured.allocations, 0);
        assert_eq!(measured.requested_bytes, 0);
        assert_eq!(fields.code, "text-delta-term-capacity");
        assert!(
            matches!(fields.field("observed"), ErrorValue::Unsigned(value) if value == u128::from(u32::MAX) + 1)
        );
        assert!(
            matches!(fields.field("limit"), ErrorValue::Unsigned(value) if value == u128::from(u32::MAX))
        );
        let ErrorValue::Cause(cause) = fields.field("cause") else {
            panic!("the original conversion refusal remains borrowed");
        };
        assert_eq!(Capture::read(cause).code, "integer-conversion-range");
        assert!(
            error
                .source()
                .unwrap()
                .downcast_ref::<std::num::TryFromIntError>()
                .is_some()
        );
        assert_eq!(
            super::term_ordinal(u32::MAX as usize - 1).unwrap(),
            u32::MAX - 1
        );
    }

    #[test]
    fn each_constructor_builds_its_own_variant() {
        let configured = TextError::config("x");
        let captured = purrdf_testkit::error_capture::Capture::read(&configured)
            .child("cause")
            .child("cause");
        assert_eq!(captured.code, "text-config-caller-message");
        assert!(matches!(
            captured.field("message"),
            purrdf_hash::error::ErrorValue::Text("x")
        ));
        assert_eq!(
            TextError::data("x"),
            TextError::Data("x".to_owned()),
            "data() must build Data"
        );
    }

    /// Every rendering names the detail it was given, so a diagnostic never
    /// loses the one piece of information a reader needs.
    #[test]
    fn display_carries_the_detail() {
        for (error, detail) in [
            (
                TextError::config("no predicate IRI supplied"),
                "no predicate IRI supplied",
            ),
            (
                TextError::data("predicate absent from the dataset"),
                "predicate absent from the dataset",
            ),
            (
                TextError::source_read(purrdf_core::SegmentedError::Corrupt(
                    "storage checksum mismatch",
                )),
                "storage checksum mismatch",
            ),
        ] {
            assert!(error.to_string().contains(detail));
        }
    }

    #[test]
    fn conversion_maps_config_and_data_to_their_namesakes() {
        let config: EvalError = TextError::config("no IRI").into();
        assert!(
            matches!(&config, EvalError::RetainedNativeFailure(failure) if failure.kind() == purrdf_sparql_eval::NativeDiagnosticKind::Config),
            "Config must retain its classification, got {config:?}"
        );

        let data: EvalError = TextError::data("absent predicate").into();
        assert!(
            matches!(data, EvalError::Data(_)),
            "Data must map to EvalError::Data, got {data:?}"
        );
    }

    #[test]
    fn source_read_failure_remains_fatal_and_preserves_its_detail() {
        let cause = purrdf_core::PagedQueryError::Provider {
            page: purrdf_core::PageId(7),
            message: "storage checksum mismatch ".repeat(512),
        };
        let error = TextError::source_read(cause.clone());
        assert_eq!(
            error.to_string(),
            format!("text-index source read error: {cause}")
        );
        let copy = error.clone();
        let converted: EvalError = error.into();
        assert!(converted.is_native_storage_failure());
        assert_eq!(converted.code(), Some("native-sparql-source-read"));
        let EvalError::RetainedNativeFailure(retained) = converted else {
            panic!("original native source owner");
        };
        let TextError::SourceRead(original) = copy else {
            unreachable!()
        };
        assert!(core::ptr::eq(original.cause(), retained.cause()));
        let native = std::error::Error::source(&retained)
            .unwrap()
            .downcast_ref::<purrdf_core::PagedQueryError>()
            .unwrap();
        assert_eq!(native, &cause);
        let rendered = retained.to_string();
        assert!(rendered.starts_with("dataset read failed: "));
        assert!(rendered.len() < cause.to_string().len());
    }

    /// The arithmetic failures are the relation's own computation failing
    /// mid-call, which the evaluator classifies as a host-function error — not
    /// as bad data and not as bad configuration.
    #[test]
    fn conversion_maps_arithmetic_failures_to_function() {
        for error in [
            crate::Fixed::from_raw(i128::MAX)
                .checked_add(crate::Fixed::ONE)
                .unwrap_err(),
            crate::Fixed::ZERO.ln().unwrap_err(),
        ] {
            let expected = EvalError::function(error.to_string()).to_string();
            let copy = error.clone();
            let converted: EvalError = error.into();
            let EvalError::RetainedNativeFailure(failure) = &converted else {
                panic!("arithmetic keeps the original native cause");
            };
            assert_eq!(
                failure.kind(),
                purrdf_sparql_eval::NativeDiagnosticKind::Function
            );
            let (TextError::Overflow(original) | TextError::Domain(original)) = copy else {
                panic!("original arithmetic classification");
            };
            assert!(core::ptr::eq(original.cause(), failure.cause()));
            assert_eq!(converted.to_string(), expected);
        }
    }
}
