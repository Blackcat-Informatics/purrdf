// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Shared RDF 1.2 claim populations and their statement-query controls.

use purrdf_core::{RdfDataset, RdfDatasetBuilder};
use purrdf_sparql_eval::{ChargePoint, QueryExplanation};
use std::sync::Arc;

/// Every `bgp-candidate-quad` unit the ledger attributes to any node of `explanation`.
pub fn candidate_quads(explanation: &QueryExplanation) -> u64 {
    explanation
        .ledger()
        .iter()
        .map(|node| node.fuel_at(ChargePoint::BgpCandidateQuad))
        .sum()
}

/// One population serves the small controls and the actual large execution.
/// Matching keys stay fixed while unrelated claims and six annotations grow.
pub fn dataset(population: usize) -> Arc<RdfDataset> {
    use std::fmt::Write as _;
    let mut builder = RdfDatasetBuilder::new();
    let mut intern = |name: &str| {
        builder
            .intern_iri(&format!("http://example.org/{name}"))
            .unwrap()
    };
    let focus = intern("focus");
    let other = intern("other");
    let predicate = intern("p");
    let alternative = intern("q");
    let producer = intern("producer");
    let selected = intern("selected");
    let remainder = intern("remainder");
    let value = intern("value");
    let annotations = ["observed", "valid", "confidence", "operation", "standpoint"].map(intern);
    let mut spelling = String::new();
    for index in 0..population {
        spelling.clear();
        write!(spelling, "http://example.org/claim/{index}").unwrap();
        let reifier = builder.intern_iri(&spelling).unwrap();
        spelling.clear();
        write!(spelling, "http://example.org/object/{index}").unwrap();
        let object = builder.intern_iri(&spelling).unwrap();
        let target = builder
            .intern_triple(
                if index < 4 { focus } else { other },
                if index % 2 == 0 {
                    predicate
                } else {
                    alternative
                },
                object,
            )
            .unwrap();
        builder.push_reifier(reifier, target).unwrap();
        builder
            .push_annotation(
                reifier,
                producer,
                if index < 3 { selected } else { remainder },
            )
            .unwrap();
        for annotation in annotations {
            builder.push_annotation(reifier, annotation, value).unwrap();
        }
    }
    builder.freeze().unwrap()
}

pub const QUERIES: [(&str, u64); 2] = [
    (
        "SELECT ?r ?o WHERE { ?r <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> <<( <http://example.org/focus> <http://example.org/p> ?o )>> }",
        2,
    ),
    (
        "SELECT ?r WHERE { ?r <http://example.org/producer> <http://example.org/selected> }",
        3,
    ),
];
