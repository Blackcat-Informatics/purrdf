// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Shared explicit canonical-search policy and exact host execution evidence.

use purrdf_core::{
    CanonError, CanonOutput, CanonPhase, CanonPresentation, CanonSelection, SearchConfiguration,
    SearchEvent, SearchPolicy, SearchReceipt, SearchRefusal,
};
use purrdf_lex::json::Value;

/// One host request's original outcome and only its own finalized run receipts.
#[derive(Debug)]
pub struct CanonicalExecution<T, E> {
    /// The unchanged native value or original typed failure.
    pub result: Result<T, E>,
    /// Every constituent run, with independent per-run ceilings.
    pub receipts: Vec<SearchReceipt>,
}

/// Execute one host request with an explicit policy and optional caller signal.
/// A fresh owned control prevents stale receipts from another request appearing
/// in this result, including when the operation fails.
pub fn execute<T, E>(
    policy: SearchPolicy,
    stop: Option<&dyn purrdf_core::StopSignal>,
    operation: impl FnOnce(&mut purrdf_core::CanonicalizationControl<'_>) -> Result<T, E>,
) -> CanonicalExecution<T, E> {
    let mut control = purrdf_core::CanonicalizationControl::new(policy, stop);
    let result = operation(&mut control);
    CanonicalExecution {
        result,
        receipts: control.into_receipts(),
    }
}

/// Find the original resident canonicalization cause in a native source chain.
/// This inspects actual error types and never classifies rendered messages.
#[must_use]
pub fn native_canonical_cause<'a>(
    mut error: &'a (dyn std::error::Error + 'static),
) -> Option<&'a CanonError> {
    loop {
        if let Some(cause) = error.downcast_ref::<CanonError>() {
            return Some(cause);
        }
        error = error.source()?;
    }
}

/// A host service failure retaining the original native canonicalization cause.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CanonicalServiceError {
    /// An existing service diagnostic, unrelated to canonicalization.
    Text(String),
    /// The actual native failure, with the operation that requested identity.
    Canonical {
        /// The operation's caller-facing context.
        context: String,
        /// The original native cause, never reconstructed from its rendering.
        cause: CanonError,
    },
}

impl CanonicalServiceError {
    pub(crate) fn entail(context: &str, error: purrdf_entail::EntailError) -> Self {
        match error {
            purrdf_entail::EntailError::Canonicalization(cause) => Self::Canonical {
                context: context.to_owned(),
                cause,
            },
            error => Self::Text(format!("{context}: {error}")),
        }
    }
}

impl std::fmt::Display for CanonicalServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Text(message) => f.write_str(message),
            Self::Canonical { context, cause } => {
                write!(f, "{context} was refused canonicalization: {cause}")
            }
        }
    }
}

impl std::error::Error for CanonicalServiceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Text(_) => None,
            Self::Canonical { cause, .. } => Some(cause),
        }
    }
}

impl purrdf_hash::error::StructuredError for CanonicalServiceError {
    purrdf_hash::error_visit! {
        Text(message) => "canonical-service-message" { "message" => Message(message) },
        Canonical { context, cause } => "canonical-service-refused" {
            "context" => Text(context), "cause" => Cause(cause),
        },
    }
}

purrdf_lex::variant_from!(CanonicalServiceError { Text(String) });

/// Decode one required policy: `unlimited`, or a canonical unsigned decimal limit.
/// Every host uses this exact grammar; omission never selects a finite ceiling.
///
/// # Errors
/// Refuses malformed text, a sign, leading zeros and values outside `u64`.
pub fn parse_policy(text: &str) -> Result<SearchPolicy, CanonicalServiceError> {
    if text == "unlimited" {
        return Ok(SearchPolicy::Unlimited);
    }
    if !text.is_empty()
        && (text == "0" || !text.starts_with('0'))
        && text.bytes().all(|byte| byte.is_ascii_digit())
        && let Ok(limit) = text.parse()
    {
        return Ok(SearchPolicy::Finite(limit));
    }
    Err(CanonicalServiceError::Text(
        "canonical search policy must be unlimited or an unsigned decimal u64 without leading zeros".into(),
    ))
}

/// Exact host receipt snapshots. Integer coordinates and counters are decimal
/// strings so a JavaScript JSON reader cannot round values above `2^53`.
/// Receipt arrays describe this operation's supplied slice, never an aggregate ceiling.
#[must_use]
pub fn receipts_json(receipts: &[SearchReceipt]) -> Value {
    Value::Array(receipts.iter().map(receipt_json).collect())
}

fn receipt_json(receipt: &SearchReceipt) -> Value {
    let configuration = match &receipt.configuration {
        SearchConfiguration::Rdfc {
            profile_version,
            hash,
            presentation,
            selection,
            output,
        } => {
            let (presentation_id, presentation_version) = match presentation {
                CanonPresentation::Overlay => (
                    purrdf_core::CANON_PRESENTATION_OVERLAY_ID,
                    purrdf_core::CANON_PRESENTATION_OVERLAY_VERSION,
                ),
                CanonPresentation::FlatAssertion => (
                    purrdf_core::CANON_PRESENTATION_FLAT_ASSERTION_ID,
                    purrdf_core::CANON_PRESENTATION_FLAT_ASSERTION_VERSION,
                ),
            };
            Value::object([
                ("algorithm", Value::from(purrdf_core::CANON_PROFILE_ID)),
                ("version", Value::from(profile_version.to_string())),
                (
                    "label_hash",
                    Value::from(match hash {
                        purrdf_core::CanonHash::Sha256 => "sha256",
                        purrdf_core::CanonHash::Sha384 => "sha384",
                    }),
                ),
                ("presentation", Value::from(presentation_id)),
                (
                    "presentation_version",
                    Value::from(presentation_version.to_string()),
                ),
                (
                    "selection",
                    match selection {
                        CanonSelection::Dataset => {
                            Value::object([("kind", Value::from("dataset"))])
                        }
                        CanonSelection::NamedGraph(iri) => Value::object([
                            ("kind", Value::from("named-graph")),
                            ("iri", Value::from(iri.clone())),
                        ]),
                    },
                ),
                (
                    "output",
                    Value::from(match output {
                        CanonOutput::Document => "document",
                        CanonOutput::RelabeledDataset => "relabeled-dataset",
                        CanonOutput::RelabeledMapping => "relabeled-mapping",
                        CanonOutput::Sha256Digest => "sha256-digest",
                        CanonOutput::Blake3Digest => "blake3-digest",
                        CanonOutput::DocumentSha256Digest => "document-sha256-digest",
                    }),
                ),
            ])
        }
        SearchConfiguration::DatasetStateV1 => {
            Value::object([("algorithm", Value::from("dataset-state-v1"))])
        }
        SearchConfiguration::ComparisonPreparation => Value::object([(
            "algorithm",
            Value::from("structural-comparison-preparation"),
        )]),
    };
    Value::object([
        ("configuration", configuration),
        (
            "policy",
            match receipt.policy {
                SearchPolicy::Unlimited => Value::from("unlimited"),
                SearchPolicy::Finite(limit) => {
                    Value::object([("finite", Value::from(limit.to_string()))])
                }
            },
        ),
        (
            "charging_version",
            Value::from(receipt.charging_version.to_string()),
        ),
        (
            "charged_work",
            Value::from(receipt.work.total().to_string()),
        ),
        (
            "events",
            Value::object([
                (
                    "rdfc_frame",
                    Value::from(receipt.work.count(SearchEvent::RdfcFrame).to_string()),
                ),
                (
                    "rdfc_permutation",
                    Value::from(receipt.work.count(SearchEvent::RdfcPermutation).to_string()),
                ),
                (
                    "state_node",
                    Value::from(receipt.work.count(SearchEvent::StateNode).to_string()),
                ),
                (
                    "state_candidate",
                    Value::from(receipt.work.count(SearchEvent::StateCandidate).to_string()),
                ),
                (
                    "state_automorphism",
                    Value::from(
                        receipt
                            .work
                            .count(SearchEvent::StateAutomorphism)
                            .to_string(),
                    ),
                ),
            ]),
        ),
        (
            "phase",
            Value::from(match receipt.phase {
                CanonPhase::Preparation => "preparation",
                CanonPhase::Search => "search",
                CanonPhase::Output => "output",
            }),
        ),
        ("completed", Value::from(receipt.completed)),
        (
            "observed_refusal",
            match receipt.observed_refusal {
                None => Value::Null,
                Some(refusal) => Value::from(match refusal {
                    SearchRefusal::Limit => "finite-limit",
                    SearchRefusal::Stopped => "caller-stop",
                    SearchRefusal::CounterCapacity => "counter-capacity",
                }),
            },
        ),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_policy_requires_an_explicit_exact_u64_and_receipts_preserve_it() {
        for text in [
            "",
            " 1",
            "1 ",
            "+1",
            "-1",
            "00",
            "01",
            "18446744073709551616",
        ] {
            assert!(parse_policy(text).is_err(), "accepted {text:?}");
        }
        assert_eq!(parse_policy("unlimited").unwrap(), SearchPolicy::Unlimited);
        let dataset = purrdf_core::RdfDatasetBuilder::new().freeze().unwrap();
        for limit in [0, 9_007_199_254_740_993, u64::MAX] {
            let policy = parse_policy(&limit.to_string()).unwrap();
            assert_eq!(policy, SearchPolicy::Finite(limit));
            let request = execute(policy, None, |control| {
                purrdf_core::try_canonicalize(&dataset, control)
            });
            assert!(request.result.unwrap().nquads.is_empty());
            let encoded = receipts_json(&request.receipts).to_string();
            assert!(
                encoded.contains(&format!("\"finite\":\"{limit}\"")),
                "{encoded}"
            );
            assert!(encoded.contains("\"charged_work\":\"0\""), "{encoded}");
        }
    }
}
