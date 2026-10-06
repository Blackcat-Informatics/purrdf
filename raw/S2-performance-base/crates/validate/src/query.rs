// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! What a host attaches to a SPARQL query's results beside the answers.
//!
//! The command line, the C ABI and the wasm package each serialize a SELECT or
//! ASK result with the additive `purrdf` provenance extension when their caller
//! supplies a namespace to anchor it under. What that extension says about the
//! query is computed here, once, so the same query text carries the same
//! identity whichever host answered it.

use purrdf_sparql_eval::AggregateRegistry;
use purrdf_sparql_results::{ProvenanceNamespace, ResultProvenance};
use sha2::{Digest as _, Sha256};

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

#[cfg(test)]
mod tests {
    use super::{ENGINE_LABEL, provenance};
    use purrdf_sparql_results::{ProvenanceNamespace, ResultProvenance};

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
