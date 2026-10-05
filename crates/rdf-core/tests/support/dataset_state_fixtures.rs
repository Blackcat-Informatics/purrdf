// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Fixed canonicalizer scale and symmetry witnesses shared by native tests and benches.

use std::sync::Arc;

use super::{BlankScope, RdfDataset, RdfDatasetBuilder};

/// Distinct IRI anchors make every blank structurally unique without branching.
pub(super) fn anchored_blanks(count: usize, reverse: bool) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let predicate = builder.intern_iri("http://example.org/p");
    for ordinal in 0..count {
        let index = if reverse {
            count - ordinal - 1
        } else {
            ordinal
        };
        let label = if reverse { count - index } else { index };
        let blank = builder.intern_blank(
            &format!("b{label}"),
            if reverse {
                BlankScope(19)
            } else {
                BlankScope::DEFAULT
            },
        );
        let anchor = builder.intern_iri(&format!("http://example.org/anchor/{index:08}"));
        builder.push_quad(blank, predicate, anchor, None);
    }
    builder.freeze().unwrap()
}

/// Disjoint directed triangles have component permutations without blank transpositions.
pub(super) fn triangle_components(count: usize) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let predicate = builder.intern_iri("http://example.org/p");
    for component in 0..count {
        let blanks = [0, 1, 2].map(|index| {
            builder.intern_blank(&format!("c{component}b{index}"), BlankScope::DEFAULT)
        });
        for index in 0..3 {
            builder.push_quad(blanks[index], predicate, blanks[(index + 1) % 3], None);
        }
    }
    builder.freeze().unwrap()
}
