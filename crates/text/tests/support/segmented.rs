// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The existing native bounded-dataset fixture shared by text query owners.

use purrdf_core::RdfDataset;

pub(crate) fn segmented(dataset: &RdfDataset) -> purrdf_core::SegmentedImage {
    let mut builder = purrdf_core::SegmentedBuilder::new(
        purrdf_core::SegmentedBuildLimits::new(4096, 500_000, 4096, 512, 4).unwrap(),
    )
    .unwrap();
    for quad in dataset.quads() {
        let mut values = vec![
            dataset.term_value(quad.s),
            dataset.term_value(quad.p),
            dataset.term_value(quad.o),
        ];
        if let Some(graph) = quad.g {
            values.push(dataset.term_value(graph));
        }
        let mut ids = Vec::new();
        builder.intern_batch(&values, |_, id| ids.push(id)).unwrap();
        builder
            .push_quad(purrdf_core::QuadIds {
                s: ids[0],
                p: ids[1],
                o: ids[2],
                g: quad.g.map(|_| ids[3]),
            })
            .unwrap();
    }
    builder.seal().unwrap()
}
