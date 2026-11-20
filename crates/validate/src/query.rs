// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Shared query inputs and the provenance a host attaches beside the answers.
//!
//! The command line, the C ABI and the wasm package each serialize a SELECT or
//! ASK result with the additive `purrdf` provenance extension when their caller
//! supplies a namespace to anchor it under. What that extension says about the
//! query is computed here, once, so the same query text carries the same
//! identity whichever host answered it. [`CoordinateInputs`] decodes supplied
//! time and entropy once, independently of that query-text identity.

use purrdf_sparql_eval::{AggregateRegistry, QueryOptions};
use purrdf_sparql_results::{ProvenanceNamespace, ResultProvenance};
use purrdf_xsd::temporal::{DateTime, parse_datetime};
use sha2::{Digest as _, Sha256};

/// Captures native observations for a host whose ordinary result has no receipt field.
/// The observer owns the first started native receipt rather than re-reading inputs.
/// Later UPDATE WHERE invocations do not replace the operation's initial receipt.
#[derive(Debug)]
pub struct ExecutionReceiptCapture(std::sync::Mutex<purrdf_sparql_eval::QueryCoordinates>);

impl Default for ExecutionReceiptCapture {
    fn default() -> Self {
        Self(std::sync::Mutex::new(
            purrdf_sparql_eval::QueryCoordinates::NotStarted,
        ))
    }
}

impl ExecutionReceiptCapture {
    /// Borrow and retain the actual observation for a failure published before return.
    ///
    /// # Errors
    /// A poisoned observer is a typed execution invariant failure.
    pub fn coordinates(
        &self,
    ) -> Result<purrdf_sparql_eval::QueryCoordinates, purrdf_sparql_eval::EvalError> {
        self.0
            .lock()
            .map(|coordinates| coordinates.clone())
            .map_err(|_| {
                purrdf_sparql_eval::EvalError::Invariant(
                    purrdf_sparql_eval::EvaluatorInvariant::CoordinateObserverPoisoned,
                )
            })
    }

    /// Transfer the observed execution receipt to its public result or failure.
    ///
    /// # Errors
    /// A poisoned capture is an execution invariant failure, never an absent receipt.
    pub fn into_coordinates(
        self,
    ) -> Result<purrdf_sparql_eval::QueryCoordinates, purrdf_sparql_eval::EvalError> {
        self.0.into_inner().map_err(|_| {
            purrdf_sparql_eval::EvalError::Invariant(
                purrdf_sparql_eval::EvaluatorInvariant::CoordinateObserverPoisoned,
            )
        })
    }
}

impl purrdf_sparql_eval::ExecutionCoordinateObserver for ExecutionReceiptCapture {
    fn observe(
        &self,
        coordinates: &purrdf_sparql_eval::QueryCoordinates,
    ) -> Result<(), purrdf_sparql_eval::EvalError> {
        let mut captured = self.0.lock().map_err(|_| {
            purrdf_sparql_eval::EvalError::Invariant(
                purrdf_sparql_eval::EvaluatorInvariant::CoordinateObserverPoisoned,
            )
        })?;
        if matches!(*captured, purrdf_sparql_eval::QueryCoordinates::NotStarted) {
            *captured = coordinates.clone();
        }
        drop(captured);
        Ok(())
    }
}

/// Caller-supplied query or rule time and entropy, decoded once at a host boundary.
///
/// This owns request inputs, not a receipt of execution or sampled defaults.
/// Parsing samples neither the clock nor entropy. Exact fractional seconds and
/// the declared timezone follow the native `xsd:dateTime` contract.
#[derive(Debug, Default)]
pub struct CoordinateInputs {
    now: Option<DateTime>,
    entropy_seed: Option<u64>,
}

impl CoordinateInputs {
    /// Move the exact inputs into a host request that owns its coordinates.
    /// No clock or entropy is sampled and no timestamp coefficient is copied.
    #[must_use]
    pub fn into_parts(self) -> (Option<DateTime>, Option<u64>) {
        (self.now, self.entropy_seed)
    }

    /// Move supplied coordinates into rule options bounded by the request's limits.
    /// The host's knob names and all four limits remain authoritative; omitted
    /// coordinates leave the native clock and entropy defaults intact.
    #[must_use]
    pub fn into_rule_options(
        self,
        limits: &purrdf_shapes::RuleLimits,
        knobs: purrdf_shapes::LimitKnobs,
    ) -> purrdf_shapes::RuleOptions {
        let mut options = limits.rule_options(knobs);
        if let Some(now) = self.now {
            options = options.with_now(now);
        }
        if let Some(seed) = self.entropy_seed {
            options = options.with_entropy_seed(seed);
        }
        options
    }

    /// Decode independently optional time and unsigned decimal seed strings.
    /// A supplied zero is a seed; an omitted field requests the native default.
    ///
    /// # Errors
    /// Returns the corresponding typed native parse error for invalid input.
    pub fn from_parts(
        now: Option<&str>,
        entropy_seed: Option<&str>,
    ) -> Result<Self, CoordinateInputError> {
        let mut inputs = Self::from_time_and_seed(now, None)?;
        inputs.entropy_seed = entropy_seed
            .map(str::parse)
            .transpose()
            .map_err(CoordinateInputError::EntropySeed)?;
        Ok(inputs)
    }

    /// Decode an optional exact time with an already-typed unsigned seed.
    ///
    /// Hosts whose integer boundary has already produced a `u64` use this
    /// constructor without formatting and reparsing the seed. Omitted inputs
    /// request native defaults; a supplied zero remains a seed.
    ///
    /// # Errors
    /// Returns [`CoordinateInputError::Now`] for an invalid native `xsd:dateTime`.
    pub fn from_time_and_seed(
        now: Option<&str>,
        entropy_seed: Option<u64>,
    ) -> Result<Self, CoordinateInputError> {
        Ok(Self {
            now: now
                .map(parse_datetime)
                .transpose()
                .map_err(CoordinateInputError::Now)?,
            entropy_seed,
        })
    }

    /// Replace both coordinate selections, preserving all unrelated options.
    ///
    /// An omitted input clears an earlier selection to the native host default;
    /// this is replacement, not an overlay. The returned options borrow the
    /// exact caller-owned time, which execution admits through its native path.
    #[must_use]
    pub fn apply<'a, 'providers>(
        &'a self,
        mut options: QueryOptions<'a, 'providers>,
    ) -> QueryOptions<'a, 'providers> {
        options.now = self.now.as_ref();
        options.entropy_seed = self.entropy_seed;
        options
    }
}

/// A malformed supplied query coordinate, retaining its native parse cause.
#[derive(Debug)]
#[non_exhaustive]
pub enum CoordinateInputError {
    /// The supplied time is not a native `xsd:dateTime`.
    Now(purrdf_xsd::XsdError),
    /// The supplied seed is not an unsigned 64-bit decimal integer.
    EntropySeed(std::num::ParseIntError),
}

impl std::fmt::Display for CoordinateInputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Now(error) => write!(f, "query time: {error}"),
            Self::EntropySeed(error) => write!(f, "query entropy seed: {error}"),
        }
    }
}

purrdf_hash::error_sources!(CoordinateInputError, Now, EntropySeed);

/// The producer label a populated [`provenance`] carries: the evaluator that
/// answered the query, whichever host called it.
pub const ENGINE_LABEL: &str = "purrdf-sparql-eval";

/// The statistical-aggregate registry a host's aggregate-namespace setting
/// requests, or `None` when the host was given no namespace.
///
/// `AggregateRegistry::register_statistical_aggregates` takes only an IRI
/// namespace string, so every host — the command line, the C ABI, the wasm
/// package and the Python binding — crosses its boundary with one nullable
/// string and builds the registry here, the same way. The namespace is caller
/// configuration, never a PurRDF vocabulary: without one, each of the ten names
/// is an ordinary unregistered custom-aggregate IRI.
#[must_use]
pub fn statistical_aggregates(namespace: Option<&str>) -> Option<AggregateRegistry> {
    let namespace = namespace?;
    let mut registry = AggregateRegistry::default();
    registry.register_statistical_aggregates(namespace);
    Some(registry)
}

/// The [`ResultProvenance`] a SPARQL-results emission of `query` carries.
///
/// Empty when `namespace` is `None`: with nothing to anchor the extension
/// under, the output stays pure W3C and the extension is not emitted at all.
/// With a namespace it carries:
///
/// * `query_hash` — `sha256:` followed by the lowercase hex SHA-256 of the
///   query's UTF-8 text: an opaque, deterministic identity a caller can compare
///   across runs and across hosts, computed from the text the host already
///   holds rather than from anything the evaluator would have to track;
/// * `engine` — [`ENGINE_LABEL`];
/// * no `solutions`: per-solution source provenance is filled by the
///   evaluator's derivation graph, which a host cannot reconstruct from the
///   answers it was handed.
#[must_use]
pub fn provenance(namespace: Option<&ProvenanceNamespace>, query: &str) -> ResultProvenance {
    if namespace.is_none() {
        return ResultProvenance::default();
    }
    let digest = Sha256::digest(query.as_bytes());
    ResultProvenance {
        query_hash: Some(format!("sha256:{}", purrdf_hash::hex::Lower(&digest))),
        engine: Some(ENGINE_LABEL.to_owned()),
        solutions: Vec::new(),
    }
}

/// Render an actual native execution receipt without reconstructing it from inputs.
///
/// The receipt carries the run's coordinates and its run identity, which binds the
/// request, the clock, the seed and the environment. It carries no source content
/// identity: computing one hashes the whole dataset, so a host never pays for it on
/// a caller's behalf. A Rust caller that wants the execution identity supplies the
/// sources' identities to `SuppliedQueryCoordinates::execution_identity`. Exact
/// timestamp spelling and the initial unsigned seed survive every host's number
/// representation. A run without a run identity (external code without a certified
/// implementation identity) renders `null`, never an identity inferred from the
/// query string or the result rows.
#[must_use]
pub fn execution_receipt_value(
    coordinates: &purrdf_sparql_eval::QueryCoordinates,
) -> purrdf_lex::json::Value {
    use purrdf_lex::json::{Object, Value};
    match coordinates {
        purrdf_sparql_eval::QueryCoordinates::NotStarted => {
            Object::new().with("state", "not-started").into()
        }
        purrdf_sparql_eval::QueryCoordinates::Started(started) => Object::new()
            .with("state", "started")
            .with("now", started.effective_now().to_string())
            .with("entropySeed", started.effective_entropy_seed().to_string())
            .with("suppliedNow", started.now().is_some())
            .with("suppliedEntropySeed", started.entropy_seed().is_some())
            .with(
                "runIdentity",
                started
                    .run_identity()
                    .map_or(Value::Null, |identity| Value::from(identity.to_string())),
            )
            .with("readsFocusGraph", started.reads_focus_graph())
            .into(),
    }
}

/// The shared JSON transport of an actual native execution receipt.
#[must_use]
pub fn execution_receipt_json(coordinates: &purrdf_sparql_eval::QueryCoordinates) -> String {
    purrdf_lex::json::write_compact(&execution_receipt_value(coordinates))
}

/// The receipt of `coordinates` bound to the content of the sources its run read:
/// [`execution_receipt_value`] with `contentIdentity` (and `focusGraphIdentity`
/// when supplied) and `executionIdentity` added.
///
/// # Errors
/// [`ReceiptError`](purrdf_sparql_eval::ReceiptError) when the run has no execution
/// identity: it never started, it called extension code without a certified
/// implementation identity, or the focus-graph identity is missing or unused.
pub fn execution_receipt_value_with_sources(
    coordinates: &purrdf_sparql_eval::QueryCoordinates,
    sources: &purrdf_sparql_eval::ExecutionSources,
) -> Result<purrdf_lex::json::Value, purrdf_sparql_eval::ReceiptError> {
    let execution = coordinates.execution_identity(sources)?;
    let mut value = execution_receipt_value(coordinates);
    let Some(object) = value.as_object_mut() else {
        return Err(purrdf_sparql_eval::ReceiptError::NotStarted);
    };
    object.insert("contentIdentity", sources.dataset().to_string());
    if let Some(focus) = sources.focus_graph() {
        object.insert("focusGraphIdentity", focus.to_string());
    }
    object.insert("executionIdentity", execution.to_string());
    Ok(value)
}

/// [`execution_receipt_value_with_sources`] as compact JSON.
///
/// # Errors
/// As [`execution_receipt_value_with_sources`].
pub fn execution_receipt_json_with_sources(
    coordinates: &purrdf_sparql_eval::QueryCoordinates,
    sources: &purrdf_sparql_eval::ExecutionSources,
) -> Result<String, purrdf_sparql_eval::ReceiptError> {
    execution_receipt_value_with_sources(coordinates, sources)
        .map(|value| purrdf_lex::json::write_compact(&value))
}

/// Why a host receipt could not be bound to a content identity.
#[derive(Debug)]
#[non_exhaustive]
pub enum ReceiptBindError {
    /// The receipt text is not JSON.
    Json(purrdf_lex::json::Error),
    /// The JSON is not an execution receipt: this member is missing or ill-typed.
    NotAReceipt {
        /// The receipt member that is missing or ill-typed.
        member: &'static str,
    },
    /// The receipt's run cannot have an execution identity.
    Refused(purrdf_sparql_eval::ReceiptError),
    /// The view refused to produce its content identity; the view's own error is the cause.
    Identity(Box<dyn std::error::Error + Send + Sync>),
}

impl std::fmt::Display for ReceiptBindError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Json(error) => write!(f, "the execution receipt is not JSON: {error}"),
            Self::NotAReceipt { member } => {
                write!(
                    f,
                    "not an execution receipt: `{member}` is missing or ill-typed"
                )
            }
            Self::Refused(error) => error.fmt(f),
            Self::Identity(error) => write!(f, "the view's content identity: {error}"),
        }
    }
}

impl std::error::Error for ReceiptBindError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Json(error) => Some(error),
            Self::Refused(error) => Some(error),
            Self::Identity(error) => Some(&**error),
            Self::NotAReceipt { .. } => None,
        }
    }
}

impl purrdf_hash::error::StructuredError for ReceiptBindError {
    purrdf_hash::error_visit! {
        Json(error) => "receipt-bind-json" { "cause" => Record(error) },
        NotAReceipt { member } => "receipt-bind-not-a-receipt" { "member" => Code(member) },
        Refused(error) => "receipt-bind-refused" { "cause" => Record(error) },
        Identity(_cause) => "receipt-bind-identity" {},
    }
}

/// Bind a host's execution receipt JSON to the content identity of the view
/// its run read: the receipt with `contentIdentity` and `executionIdentity` added,
/// under the same law as [`execution_receipt_value_with_sources`].
///
/// This is the explicit request every host that hands its caller only receipt
/// JSON routes through. `view` must declare [`purrdf_core::ContentIdentified`], so a
/// view that can never certify its content is refused when the call is compiled.
/// The identity is requested only after the receipt is admitted, so a refused
/// binding computes no content identity; a refusal of the identity itself is
/// returned as [`ReceiptBindError::Identity`]. A host receipt carries no
/// focus-graph identity, so a run that read a focus graph is refused.
///
/// A resident dataset is accepted as the view:
///
/// ```
/// let dataset = purrdf_core::RdfDatasetBuilder::new().freeze().unwrap();
/// let receipt = r#"{"state":"not-started"}"#;
/// // The receipt is refused for its run, not for the view.
/// assert!(purrdf_validate::query::bind_execution_receipt(receipt, &dataset).is_err());
/// ```
///
/// A view without the capability does not compile:
///
/// ```compile_fail
/// use purrdf_core::DatasetView;
/// fn bind<V: DatasetView>(view: &V) {
///     let _ = purrdf_validate::query::bind_execution_receipt("{}", view);
/// }
/// ```
///
/// # Errors
/// [`ReceiptBindError`].
pub fn bind_execution_receipt<V>(receipt: &str, view: &V) -> Result<String, ReceiptBindError>
where
    V: purrdf_core::ContentIdentified + ?Sized,
    V::IdentityError: std::error::Error + Send + Sync + 'static,
{
    use purrdf_lex::json::Value;
    use purrdf_sparql_eval::ReceiptError;
    let mut value = purrdf_lex::json::read(receipt).map_err(ReceiptBindError::Json)?;
    match value.get("state").and_then(Value::as_str) {
        Some("started") => {}
        Some("not-started") => return Err(ReceiptBindError::Refused(ReceiptError::NotStarted)),
        _ => return Err(ReceiptBindError::NotAReceipt { member: "state" }),
    }
    let run = match value.get("runIdentity") {
        Some(Value::Null) => {
            return Err(ReceiptBindError::Refused(
                ReceiptError::OpaqueImplementation,
            ));
        }
        Some(Value::String(text)) => {
            purrdf_hash::hex::Digest32::from_hex(text).ok_or(ReceiptBindError::NotAReceipt {
                member: "runIdentity",
            })?
        }
        _ => {
            return Err(ReceiptBindError::NotAReceipt {
                member: "runIdentity",
            });
        }
    };
    let reads_focus_graph = value
        .get("readsFocusGraph")
        .and_then(Value::as_bool)
        .ok_or(ReceiptBindError::NotAReceipt {
            member: "readsFocusGraph",
        })?;
    if reads_focus_graph {
        return Err(ReceiptBindError::Refused(
            ReceiptError::FocusGraphIdentityMissing,
        ));
    }
    let sources = purrdf_sparql_eval::ExecutionSources::new(
        view.content_identity()
            .map_err(|error| ReceiptBindError::Identity(Box::new(error)))?,
    );
    let execution = sources
        .execution_identity_of(&run, false)
        .map_err(ReceiptBindError::Refused)?;
    let Some(object) = value.as_object_mut() else {
        return Err(ReceiptBindError::NotAReceipt { member: "state" });
    };
    object.insert("contentIdentity", sources.dataset().to_string());
    object.insert("executionIdentity", execution.to_string());
    Ok(purrdf_lex::json::write_compact(&value))
}

/// The actual nested executions of a rule request, beside its supplied scope.
///
/// Like [`execution_receipt_value`], it carries no content identity of the data or
/// shapes graph; `RuleCoordinates::execution_identity` takes them from a caller
/// that wants the execution identity.
#[must_use]
pub fn rule_execution_receipt_value(
    coordinates: &purrdf_shapes::RuleCoordinates,
) -> purrdf_lex::json::Value {
    rule_receipt_object(coordinates).into()
}

fn rule_receipt_object(coordinates: &purrdf_shapes::RuleCoordinates) -> purrdf_lex::json::Object {
    use purrdf_lex::json::{Object, Value};
    Object::new()
        .with(
            "programIdentity",
            coordinates
                .program_identity()
                .map_or(Value::Null, |value| Value::from(value.to_string())),
        )
        .with(
            "now",
            coordinates
                .now()
                .map_or(Value::Null, |now| Value::from(now.to_string())),
        )
        .with(
            "entropySeed",
            coordinates
                .entropy_seed()
                .map_or(Value::Null, |seed| Value::from(seed.to_string())),
        )
        .with(
            "executions",
            coordinates
                .executions()
                .iter()
                .map(execution_receipt_value)
                .collect::<Vec<_>>(),
        )
}

/// Shared JSON transport for successful and refused rule execution receipts.
#[must_use]
pub fn rule_execution_receipt_json(coordinates: &purrdf_shapes::RuleCoordinates) -> String {
    purrdf_lex::json::write_compact(&rule_execution_receipt_value(coordinates))
}

/// The rule receipt bound to the data and shapes graphs the inference read:
/// [`rule_execution_receipt_value`] with `dataIdentity`, `shapesIdentity` and
/// `executionIdentity` added.
///
/// # Errors
/// [`RuleReceiptError`](purrdf_shapes::RuleReceiptError) when the inference has no
/// execution identity.
pub fn rule_execution_receipt_value_with_sources(
    coordinates: &purrdf_shapes::RuleCoordinates,
    sources: &purrdf_shapes::RuleSources,
) -> Result<purrdf_lex::json::Value, purrdf_shapes::RuleReceiptError> {
    let execution = coordinates.execution_identity(sources)?;
    Ok(rule_receipt_object(coordinates)
        .with("dataIdentity", sources.data().to_string())
        .with("shapesIdentity", sources.shapes().to_string())
        .with("executionIdentity", execution.to_string())
        .into())
}

#[cfg(test)]
mod tests {
    use super::{ENGINE_LABEL, provenance};
    use purrdf_sparql_results::{ProvenanceNamespace, ResultProvenance};

    /// A resident dataset's view with a scripted identity request: the dataset's
    /// own rows, and an identity that is refused or must never be asked for.
    struct Scripted<'a> {
        dataset: &'a purrdf_core::RdfDataset,
        identity: Identity,
    }

    #[derive(Clone, Copy)]
    enum Identity {
        Refused,
        NeverAsked,
    }

    #[derive(Debug)]
    struct IdentityRefused;

    impl std::fmt::Display for IdentityRefused {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("the scripted view cannot certify its content")
        }
    }

    impl std::error::Error for IdentityRefused {}

    impl purrdf_core::DatasetView for Scripted<'_> {
        type Id = purrdf_core::TermId;
        type ReadError = std::convert::Infallible;
        type TermGuard<'a>
            = purrdf_core::TermRef<'a, Self::Id>
        where
            Self: 'a;
        type ProbePlan = ();
        fn quads(&self) -> impl Iterator<Item = purrdf_core::QuadIds> + '_ {
            self.dataset.quads()
        }
        fn resolve(&self, id: Self::Id) -> Result<Self::TermGuard<'_>, Self::ReadError> {
            purrdf_core::DatasetView::resolve(self.dataset, id)
        }
        fn term_id_by_value(
            &self,
            value: &purrdf_core::TermValue,
        ) -> Result<Option<Self::Id>, Self::ReadError> {
            purrdf_core::DatasetView::term_id_by_value(self.dataset, value)
        }
        fn capabilities(&self) -> purrdf_core::RdfStoreCapabilities {
            purrdf_core::DatasetView::capabilities(self.dataset)
        }
        fn probe_plan(&self, _: bool, _: bool, _: bool, _: purrdf_core::GraphMatch) {}
        fn quads_for_pattern_with_plan(
            &self,
            (): &(),
            s: Option<Self::Id>,
            p: Option<Self::Id>,
            o: Option<Self::Id>,
            g: purrdf_core::GraphMatch,
        ) -> impl Iterator<Item = purrdf_core::QuadIds> + '_ {
            self.dataset.quads_for_pattern(s, p, o, g)
        }
        fn term_count(&self) -> u64 {
            purrdf_core::DatasetView::term_count(self.dataset)
        }
    }

    impl purrdf_core::ContentIdentified for Scripted<'_> {
        type IdentityError = IdentityRefused;
        fn content_identity(&self) -> Result<purrdf_hash::hex::Digest32, IdentityRefused> {
            match self.identity {
                Identity::Refused => Err(IdentityRefused),
                Identity::NeverAsked => panic!("a refusal hashes nothing"),
            }
        }
    }

    /// The receipt of a started run over `dataset`, with its native coordinates.
    fn started_receipt(
        dataset: &purrdf_core::RdfDataset,
    ) -> (String, purrdf_sparql_eval::QueryCoordinates) {
        let engine = purrdf_sparql_eval::NativeSparqlEngine::new();
        let capture = super::ExecutionReceiptCapture::default();
        engine
            .query_with_options_view(
                dataset,
                purrdf_core::SparqlRequest {
                    query: "SELECT (1 AS ?one) WHERE {}",
                    base_iri: None,
                    substitutions: &[],
                },
                &purrdf_sparql_eval::QueryOptions::new().with_coordinate_observer(&capture),
            )
            .unwrap();
        let coordinates = capture.into_coordinates().unwrap();
        (super::execution_receipt_json(&coordinates), coordinates)
    }

    /// A host binding a receipt's JSON reaches exactly the execution identity the
    /// native coordinates compute for the same content; every receipt the binding
    /// refuses asks its view for no content identity, and its neighbour binds.
    #[test]
    fn a_host_receipt_binds_to_the_native_execution_identity_and_refusals_hash_nothing() {
        use purrdf_core::ContentIdentified;
        use purrdf_sparql_eval::{ExecutionSources, ReceiptError};
        let dataset = purrdf_core::RdfDatasetBuilder::new().freeze().unwrap();
        let (receipt, coordinates) = started_receipt(&dataset);
        let native = super::execution_receipt_json_with_sources(
            &coordinates,
            &ExecutionSources::new(dataset.content_identity().unwrap()),
        )
        .unwrap();
        assert!(!receipt.contains("contentIdentity"));
        assert_eq!(
            super::bind_execution_receipt(&receipt, &dataset).unwrap(),
            native
        );

        let unhashed = Scripted {
            dataset: &dataset,
            identity: Identity::NeverAsked,
        };
        let refused = |text: &str| super::bind_execution_receipt(text, &unhashed).unwrap_err();
        assert!(matches!(
            refused(r#"{"state":"not-started"}"#),
            super::ReceiptBindError::Refused(ReceiptError::NotStarted)
        ));
        let opaque = receipt.replacen("\"runIdentity\":\"", "\"runIdentity\":null,\"x\":\"", 1);
        assert!(matches!(
            refused(&opaque),
            super::ReceiptBindError::Refused(ReceiptError::OpaqueImplementation)
        ));
        let focused = receipt.replace("\"readsFocusGraph\":false", "\"readsFocusGraph\":true");
        assert!(matches!(
            refused(&focused),
            super::ReceiptBindError::Refused(ReceiptError::FocusGraphIdentityMissing)
        ));
        assert!(matches!(refused("{"), super::ReceiptBindError::Json(_)));
        assert!(matches!(
            refused(r#"{"state":"started"}"#),
            super::ReceiptBindError::NotAReceipt {
                member: "runIdentity"
            }
        ));
    }

    /// A receipt requested from a view whose content identity is refused is the
    /// typed `Identity` refusal carrying the view's own error and no output, while
    /// the same receipt bound to a resident dataset succeeds.
    #[test]
    fn a_receipt_from_a_view_that_cannot_certify_its_content_is_a_typed_refusal() {
        use std::error::Error as _;
        let dataset = purrdf_core::RdfDatasetBuilder::new().freeze().unwrap();
        let (receipt, _) = started_receipt(&dataset);

        let view = Scripted {
            dataset: &dataset,
            identity: Identity::Refused,
        };
        let refused = super::bind_execution_receipt(&receipt, &view).unwrap_err();
        assert!(matches!(refused, super::ReceiptBindError::Identity(_)));
        assert!(
            refused
                .source()
                .is_some_and(<dyn std::error::Error>::is::<IdentityRefused>)
        );

        let bound = super::bind_execution_receipt(&receipt, &dataset).unwrap();
        assert!(bound.contains("\"contentIdentity\""));
        assert!(bound.contains("\"executionIdentity\""));
    }

    /// Replacing a prior request must not retain its time or seed by accident.
    #[test]
    fn coordinate_inputs_replace_prior_selections_without_resetting_other_options() {
        let time = "2020-01-02T03:04:05.12345678901234567890123456789+05:30";
        let prior =
            super::CoordinateInputs::from_parts(Some(time), Some("7")).expect("valid coordinates");
        let replacement = super::CoordinateInputs::from_parts(None, Some("0")).expect("zero seed");
        let division = purrdf_sparql_eval::DivisionPolicy::Exact;
        let options = replacement
            .apply(prior.apply(purrdf_sparql_eval::QueryOptions::new().with_division(division)));
        assert!(
            options.now.is_none(),
            "absent replacement clears prior time"
        );
        assert_eq!(options.entropy_seed, Some(0));
        assert_eq!(options.division, division);
        let time_only =
            super::CoordinateInputs::from_parts(Some(time), None).expect("time-only request");
        let options = time_only.apply(options);
        assert!(
            options.entropy_seed.is_none(),
            "absent replacement clears prior seed"
        );
        assert_eq!(options.now.expect("supplied time").to_string(), time);
        let empty = super::CoordinateInputs::default();
        let options = empty.apply(options);
        assert!(options.now.is_none());
        assert!(options.entropy_seed.is_none());
        assert_eq!(options.division, division);
        for (now, seed) in [
            (Some("invalid"), None),
            (None, Some("-1")),
            (Some("invalid"), Some("-1")),
        ] {
            let error = super::CoordinateInputs::from_parts(now, seed).expect_err("invalid");
            assert!(std::error::Error::source(&error).is_some());
            assert!(matches!(
                (&error, now),
                (super::CoordinateInputError::Now(_), Some(_))
                    | (super::CoordinateInputError::EntropySeed(_), None)
            ));
        }
    }

    fn namespace() -> ProvenanceNamespace {
        ProvenanceNamespace::new("prov", "https://example.org/ns/prov#").expect("a valid namespace")
    }

    #[test]
    fn no_namespace_leaves_the_output_pure_w3c() {
        let empty = provenance(None, "SELECT * WHERE { ?s ?p ?o }");
        assert_eq!(empty, ResultProvenance::default());
        assert!(empty.is_empty());
    }

    #[test]
    fn a_namespace_carries_the_query_hash_and_the_engine_label() {
        let carried = provenance(Some(&namespace()), "SELECT * WHERE { ?s ?p ?o }");
        assert_eq!(
            carried,
            ResultProvenance {
                query_hash: Some(
                    "sha256:dab3891081bcd39898fa154675c0d147665f9b3a5aae251f3dfa296b697ef999"
                        .to_owned()
                ),
                engine: Some(ENGINE_LABEL.to_owned()),
                solutions: Vec::new(),
            }
        );
    }

    #[test]
    fn the_empty_query_hashes_to_the_empty_digest() {
        let carried = provenance(Some(&namespace()), "");
        assert_eq!(
            carried.query_hash.as_deref(),
            Some("sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855")
        );
    }

    #[test]
    fn the_hash_is_of_the_exact_text_and_not_of_the_namespace() {
        let other = ProvenanceNamespace::new("p2", "https://example.org/other#").expect("valid");
        let query = "ASK { }";
        assert_eq!(
            provenance(Some(&namespace()), query),
            provenance(Some(&other), query),
            "the namespace anchors the extension; it is not part of the identity"
        );
        assert_ne!(
            provenance(Some(&namespace()), query).query_hash,
            provenance(Some(&namespace()), "ASK {}").query_hash,
            "whitespace is part of the text"
        );
    }
}
