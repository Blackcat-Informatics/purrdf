// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Shared real-chain inputs for renderer acceptance and the report-only benchmark.

use purrdf_core::{BlankScope, RdfDataset, RdfDatasetBuilder};
use std::sync::Arc;

/// Exactly `depth` inline blanks, with statement metadata in the selected graph.
pub(super) fn chain(depth: usize, named: bool, reverse: bool) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let predicate = builder.intern_iri("http://example.org/p");
    let root = builder.intern_iri("http://example.org/root");
    let end = builder.intern_iri("http://example.org/end");
    let graph = named.then(|| builder.intern_iri("http://example.org/graph"));
    let mut blanks = vec![end; depth];
    for offset in 0..depth {
        let level = if reverse { depth - offset - 1 } else { offset };
        blanks[level] = builder.intern_blank(&format!("node{level}"), BlankScope::DEFAULT);
    }
    for offset in 0..=depth {
        let level = if reverse { depth - offset } else { offset };
        let subject = if level == 0 { root } else { blanks[level - 1] };
        let object = blanks.get(level).copied().unwrap_or(end);
        builder.push_quad(subject, predicate, object, graph);
    }
    let reifier = builder.intern_iri("http://example.org/reifier");
    let annotation = builder.intern_iri("http://example.org/note");
    // A grounded triple keeps all chain blanks inline while exercising the
    // separate RDF 1.2 statement layer on each actual output route.
    let triple = builder.intern_triple(end, predicate, end);
    builder.push_reifier_in_graph(reifier, triple, graph);
    builder.push_annotation_in_graph(reifier, annotation, end, graph);
    builder.freeze().expect("valid chain and statement layer")
}
