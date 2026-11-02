// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Deterministic mixed-carrier inputs shared with comparative measurements.

use purrdf_core::{BlankScope, ContentStore, RdfDataset, RdfDatasetBuilder, RdfLiteral};
use std::sync::Arc;

#[derive(Clone, Copy, Debug)]
pub(super) enum Workload {
    Mixed,
    Reifiers,
    LargeBlobs,
}

pub(super) const ROWS: u32 = 500;

pub(super) fn fixture(workload: Workload) -> (Arc<RdfDataset>, ContentStore) {
    let mut builder = RdfDatasetBuilder::new();
    let predicate = builder.intern_iri("https://example.org/p").unwrap();
    let graph = builder.intern_iri("https://example.org/g").unwrap();
    let empty = builder.intern_iri("https://example.org/empty").unwrap();
    builder.declare_named_graph(empty).unwrap();
    let mut state = 0x0050_4152_5155_4554;
    for row in 0..ROWS {
        let subject = builder
            .intern_iri(&format!("https://example.org/s{row:08}"))
            .unwrap();
        let blank = builder
            .intern_blank(&format!("b{row}"), BlankScope(row % 8))
            .unwrap();
        let draw = purrdf_hash::mix::splitmix64_next(&mut state);
        let literal = builder
            .intern_literal(RdfLiteral::typed(
                format!("{:016}", draw % u64::from(ROWS)),
                "http://www.w3.org/2001/XMLSchema#integer",
            ))
            .unwrap();
        let repeated = builder
            .intern_literal(RdfLiteral::simple(format!("repeated {}", row % 7)))
            .unwrap();
        builder.push_quad(subject, predicate, blank, None).unwrap();
        builder
            .push_quad(blank, predicate, literal, Some(graph))
            .unwrap();
        builder
            .push_quad(
                subject,
                predicate,
                repeated,
                (row % 3 == 0).then_some(graph),
            )
            .unwrap();
        if matches!(workload, Workload::Reifiers) {
            let triple = builder.intern_triple(subject, predicate, literal).unwrap();
            builder
                .push_reifier_in_graph(blank, triple, (row % 2 == 0).then_some(graph))
                .unwrap();
            builder
                .push_annotation_in_graph(blank, predicate, repeated, Some(graph))
                .unwrap();
        }
    }
    let mut blobs = ContentStore::new();
    blobs.insert(Vec::new());
    for row in 0..16 {
        blobs.insert(format!("payload-{row:04}").into_bytes());
    }
    if matches!(workload, Workload::LargeBlobs) {
        blobs.insert(vec![b'x'; 1024 * 1024]);
        blobs.insert(
            (0..1024 * 1024)
                .map(|_| purrdf_hash::mix::splitmix64_next(&mut state) as u8)
                .collect(),
        );
    }
    (
        builder.freeze().expect("benchmark input is valid RDF"),
        blobs,
    )
}
