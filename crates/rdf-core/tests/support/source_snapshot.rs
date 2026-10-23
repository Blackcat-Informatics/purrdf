// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Frozen three-stream fixture whose dictionary can be independently rekeyed.

use purrdf_core::{RdfDataset, RdfDatasetBuilder, RdfLiteral};
use std::sync::Arc;

pub(crate) fn dataset(count: usize, changed: bool, reverse: bool) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let p = builder.intern_iri("urn:source:predicate").unwrap();
    let annotation = builder.intern_iri("urn:source:annotation").unwrap();
    let graph = builder.intern_iri("urn:source:graph").unwrap();
    for ordinal in 0..count {
        let index = if reverse {
            count - ordinal - 1
        } else {
            ordinal
        };
        let subject = builder.intern_iri(&format!("urn:source:{index}")).unwrap();
        let reifier = builder
            .intern_iri(&format!("urn:source:reifier:{index}"))
            .unwrap();
        let revision = usize::from(changed && index == count / 2);
        let object = builder
            .intern_literal(RdfLiteral::simple(format!(
                "row {index} revision {revision}"
            )))
            .unwrap();
        let note = builder
            .intern_literal(RdfLiteral::simple(format!(
                "note {index} revision {revision}"
            )))
            .unwrap();
        let g = index.is_multiple_of(2).then_some(graph);
        builder.push_quad(subject, p, object, g).unwrap();
        let triple = builder.intern_triple(subject, p, object).unwrap();
        builder.push_reifier_in_graph(reifier, triple, g).unwrap();
        builder
            .push_annotation_in_graph(reifier, annotation, note, g)
            .unwrap();
    }
    builder.freeze().unwrap()
}
