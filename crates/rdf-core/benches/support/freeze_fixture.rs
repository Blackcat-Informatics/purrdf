// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Shared deterministic fixture for freeze timing and allocation measurements.

use purrdf_core::{RdfDatasetBuilder, RdfLiteral};

pub(super) fn row_count() -> usize {
    let rows = match std::env::var("PURRDF_FREEZE_BENCH_ROWS") {
        Ok(value) => value
            .parse()
            .expect("benchmark row count must be an integer"),
        Err(std::env::VarError::NotPresent) => 100_000,
        Err(error) => panic!("cannot read benchmark row count: {error}"),
    };
    assert!(rows >= 10_000, "benchmark needs at least 10,000 rows");
    rows
}

pub(super) fn fixture(rows: usize, graph_count: usize) -> RdfDatasetBuilder {
    const PREDICATES: usize = 10;
    let subjects = rows.div_ceil(PREDICATES);
    let terms = rows
        .checked_mul(187)
        .expect("fixture term count fits usize")
        / 1170;
    let objects = terms
        .checked_sub(subjects + PREDICATES + graph_count + 1)
        .filter(|&count| count != 0)
        .expect("fixture needs distinct object terms");
    let mut builder = RdfDatasetBuilder::new();
    let predicates: Vec<_> = (0..PREDICATES)
        .map(|index| {
            builder
                .intern_iri(&format!("https://example.org/freeze/p/{index}"))
                .expect("fixture predicate is valid")
        })
        .collect();
    let graphs: Vec<_> = (0..graph_count)
        .map(|index| {
            builder
                .intern_iri(&format!("https://example.org/freeze/g/{index}"))
                .expect("fixture graph is valid")
        })
        .collect();
    let subjects: Vec<_> = (0..subjects)
        .map(|index| {
            builder
                .intern_iri(&format!("https://example.org/freeze/s/{index}"))
                .expect("fixture subject is valid")
        })
        .collect();
    let objects: Vec<_> = (0..objects)
        .map(|index| {
            if index % 2 == 0 {
                builder
                    .intern_literal(RdfLiteral::simple(format!("source value {index}")))
                    .expect("fixture literal is valid")
            } else {
                builder
                    .intern_iri(&format!("https://example.org/freeze/o/{index}"))
                    .expect("fixture object is valid")
            }
        })
        .collect();
    for index in 0..rows {
        let graph = if graphs.is_empty() {
            None
        } else {
            Some(graphs[index % graphs.len()])
        };
        // Every subject/predicate pair is unique, so validation cannot collapse
        // the requested population through statement deduplication.
        builder
            .push_quad(
                subjects[index / PREDICATES],
                predicates[index % PREDICATES],
                objects[index % objects.len()],
                graph,
            )
            .expect("fixture statement is valid");
    }
    builder
}
