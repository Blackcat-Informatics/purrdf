// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Shared public-API inputs and carrier comparisons for the layered consumer.

use purrdf_core::{
    InMemoryPageProvider, PackBuilder, PageId, PagedDataset, PagedQueryLimits, QuadValues,
    RdfDataset, RdfDatasetBuilder, TermFactory, TermValue,
};
use std::sync::Arc;

pub(crate) fn eager(
    rows: impl IntoIterator<Item = QuadValues>,
    graphs: &[TermValue],
) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    for row in rows {
        let s = builder.intern_value(&row.s).unwrap();
        let p = builder.intern_value(&row.p).unwrap();
        let o = builder.intern_value(&row.o).unwrap();
        let g = row
            .g
            .as_ref()
            .map(|value| builder.intern_value(value).unwrap());
        builder.push_quad(s, p, o, g).unwrap();
    }
    for graph in graphs {
        let id = builder.intern_value(graph).unwrap();
        builder.declare_named_graph(id).unwrap();
    }
    builder.freeze().expect("valid eager consumer input")
}

pub(crate) fn seal(page: Arc<RdfDataset>) -> Arc<PagedDataset> {
    Arc::new(
        PagedDataset::from_provider(Arc::new(InMemoryPageProvider::new(vec![page])))
            .expect("seal independent generation"),
    )
}

pub(crate) fn carrier_pages(dataset: &PagedDataset) -> Vec<Vec<u8>> {
    let mut ordered = Vec::new();
    for ordinal in 0..dataset.page_count() {
        let selected = dataset.with_pages(&[PageId(ordinal)]);
        let bytes = PackBuilder::build_view_bytes(
            &selected.query_view(PagedQueryLimits::UNBOUNDED),
            &mut purrdf_core::CanonicalizationControl::new(
                purrdf_core::RDFC_V2_CORPUS_POLICY,
                None,
            ),
        )
        .expect("complete canonical page carrier");
        ordered.push(bytes);
    }
    ordered
}
