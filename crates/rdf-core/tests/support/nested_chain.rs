// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The chain of nested triple terms the lowering's allocation test and the
//! labelling bench both measure.

use std::sync::Arc;

use purrdf_core::{RdfDataset, RdfDatasetBuilder};

/// The chain of `n` nested triple terms, `t(1) = <<( a a a )>>` and
/// `t(k + 1) = <<( a a t(k) )>>`, with the quad `a a t(k)` for each term.
pub(crate) fn chain(n: usize) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let a = builder.intern_iri("http://example.org/a").expect("interns");
    let mut term = builder.intern_triple(a, a, a).expect("interns");
    builder.push_quad(a, a, term, None).expect("pushes");
    for _ in 1..n {
        term = builder.intern_triple(a, a, term).expect("interns");
        builder.push_quad(a, a, term, None).expect("pushes");
    }
    builder.freeze().expect("freezes")
}
