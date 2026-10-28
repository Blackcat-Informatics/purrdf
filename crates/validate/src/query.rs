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
/// Exact timestamp spelling and the initial unsigned seed survive every host's
/// number representation. A missing identity is explicit, never inferred from a
/// query string or from the result rows.
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
                "executionIdentity",
                started
                    .execution_identity()
                    .map_or(Value::Null, |identity| Value::from(identity.to_string())),
            )
            .with(
                "snapshotIdentity",
                started
                    .snapshot_identity()
                    .map_or(Value::Null, |identity| Value::from(identity.to_string())),
            )
            .into(),
    }
}

/// The shared JSON transport of an actual native execution receipt.
#[must_use]
pub fn execution_receipt_json(coordinates: &purrdf_sparql_eval::QueryCoordinates) -> String {
    purrdf_lex::json::write_compact(&execution_receipt_value(coordinates))
}

/// The actual nested executions of a rule request, beside its supplied scope.
#[must_use]
pub fn rule_execution_receipt_value(
    coordinates: &purrdf_shapes::RuleCoordinates,
) -> purrdf_lex::json::Value {
    use purrdf_lex::json::{Object, Value};
    Object::new()
        .with(
            "programIdentity",
            coordinates
                .program_identity()
                .map_or(Value::Null, |value| Value::from(value.to_string())),
        )
        .with(
            "snapshotIdentity",
            coordinates
                .snapshot_identity()
                .map_or(Value::Null, |value| Value::from(value.to_string())),
        )
        .with(
            "shapesIdentity",
            coordinates
                .shapes_identity()
                .map_or(Value::Null, |value| Value::from(value.to_string())),
        )
        .with(
            "executionIdentity",
            coordinates
                .execution_identity()
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
        .into()
}

/// Shared JSON transport for successful and refused rule execution receipts.
#[must_use]
pub fn rule_execution_receipt_json(coordinates: &purrdf_shapes::RuleCoordinates) -> String {
    purrdf_lex::json::write_compact(&rule_execution_receipt_value(coordinates))
}

#[cfg(test)]
mod tests {
    use super::{ENGINE_LABEL, provenance};
    use purrdf_sparql_results::{ProvenanceNamespace, ResultProvenance};

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
