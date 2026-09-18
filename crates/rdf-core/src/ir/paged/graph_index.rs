// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! [`GraphPageIndex`] — the dataset-level "which pages carry graph G" index, derived
//! from every [`PageSlot`]'s [`PageSummary`](super::summary::PageSummary) without
//! materializing any page.
//!
//! A [`PagedDataset`](super::PagedDataset) composes many frozen pages; answering
//! "what named graphs exist" or "which pages could possibly hold graph G's rows"
//! from the trait surface alone would require scanning every page's quads. Each
//! page's [`PageSummary`] already carries that page's own declared-graph list and
//! exact per-graph row counts (in the page's LOCAL [`TermId`](crate::ir::TermId)
//! space); [`GraphPageIndex::derive`] folds those per-page answers, translated to
//! the shared [`GlobalTermId`] space, into ONE dataset-level index. The index is
//! derived — never persisted — so it stays correct across
//! [`with_pages`](super::PagedDataset::with_pages),
//! [`drop_page`](super::PagedDataset::drop_page), and
//! [`compact`](super::PagedDataset::compact) by simply being rebuilt from the
//! resulting pages, rather than needing its own remap pass.

#![allow(
    dead_code,
    reason = "the narrow read accessors (`keys`, `pages_for_named`, `pages_for_default`) are \
              this index's consumer-facing API; their production callers are the page-admission \
              predicate and the composed `named_graphs()` surface in `mod.rs` and `query.rs`, \
              and this attribute is removed once those call sites exist"
)]

use std::collections::BTreeMap;

use super::PageSlot;
use super::provider::PageId;
use super::summary::PageStream;
use crate::ir::GlobalTermId;

/// One stream's postings in compressed-sparse-row form: `offsets` has one entry
/// past the last key (`keys.len() + 1`), and `postings[offsets[i]..offsets[i + 1]]`
/// is the ascending [`PageId`] list for key `i`.
#[derive(Debug, Clone, PartialEq, Eq)]
struct StreamCsr {
    /// Row offsets into `postings`, length `keys.len() + 1`.
    offsets: Box<[u32]>,
    /// The concatenated per-key `PageId` postings, each key's slice ascending.
    postings: Box<[PageId]>,
}

impl StreamCsr {
    /// The ascending `PageId` postings for the key at `index` (an index into the
    /// owning [`GraphPageIndex::keys`], already known to be in range).
    #[inline]
    fn pages(&self, index: usize) -> &[PageId] {
        let start = usize::try_from(self.offsets[index]).expect("offset fits usize");
        let end = usize::try_from(self.offsets[index + 1]).expect("offset fits usize");
        &self.postings[start..end]
    }
}

/// Per-graph, per-stream page lists accumulated during [`GraphPageIndex::derive`],
/// before being flattened to CSR. Each list is built by iterating pages in
/// ascending [`PageId`] order, so it is already ascending and needs no sort.
#[derive(Debug, Clone, Default)]
struct GraphPostings {
    /// Pages carrying at least one base-quad row in this graph.
    base: Vec<PageId>,
    /// Pages carrying at least one reifier row in this graph.
    reifier: Vec<PageId>,
    /// Pages carrying at least one annotation row in this graph.
    annotation: Vec<PageId>,
}

/// The dataset-level "which pages carry graph G" index: every named graph any page
/// declares, and, per [`PageStream`], the ascending page ids that carry at least one
/// row of that stream in a given graph (named or default) — all without
/// materializing a page.
///
/// See the [module docs](self) for why this is derived rather than persisted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GraphPageIndex {
    /// Every named graph any page knows about, ascending by [`GlobalTermId`] and
    /// deduplicated (a declared-empty graph is included). This is the composed
    /// `named_graphs()` answer.
    keys: Box<[GlobalTermId]>,
    /// Per-key base-quad page postings, parallel to `keys`.
    base: StreamCsr,
    /// Per-key reifier-row page postings, parallel to `keys`.
    reifier: StreamCsr,
    /// Per-key annotation-row page postings, parallel to `keys`.
    annotation: StreamCsr,
    /// Ascending pages carrying at least one default-graph base-quad row.
    default_base: Box<[PageId]>,
    /// Ascending pages carrying at least one default-graph reifier row.
    default_reifier: Box<[PageId]>,
    /// Ascending pages carrying at least one default-graph annotation row.
    default_annotation: Box<[PageId]>,
}

/// Flatten `named`'s per-graph postings (already ascending `PageId` per key, since
/// `named`'s values were built by scanning pages in ascending order) into one
/// stream's CSR arrays, in `named`'s ascending key order.
fn build_csr(
    named: &BTreeMap<GlobalTermId, GraphPostings>,
    select: impl Fn(&GraphPostings) -> &[PageId],
) -> StreamCsr {
    let mut offsets: Vec<u32> = Vec::with_capacity(named.len() + 1);
    let mut postings: Vec<PageId> = Vec::new();
    offsets.push(0);
    for value in named.values() {
        postings.extend_from_slice(select(value));
        offsets.push(u32::try_from(postings.len()).expect("posting count fits u32"));
    }
    StreamCsr {
        offsets: offsets.into_boxed_slice(),
        postings: postings.into_boxed_slice(),
    }
}

impl GraphPageIndex {
    /// Every named graph any page knows about, ascending by [`GlobalTermId`] and
    /// deduplicated (declared-empty graphs included) — the composed
    /// `named_graphs()` answer.
    #[inline]
    pub(crate) fn keys(&self) -> &[GlobalTermId] {
        &self.keys
    }

    /// The ascending pages carrying at least one `stream` row in named graph `g`;
    /// an empty slice if no page does (including when `g` is not a known graph at
    /// all).
    #[inline]
    pub(crate) fn pages_for_named(&self, g: GlobalTermId, stream: PageStream) -> &[PageId] {
        let Ok(index) = self.keys.binary_search(&g) else {
            return &[];
        };
        match stream {
            PageStream::Base => self.base.pages(index),
            PageStream::Reifier => self.reifier.pages(index),
            PageStream::Annotation => self.annotation.pages(index),
        }
    }

    /// The ascending pages carrying at least one `stream` row in the default graph;
    /// an empty slice if none.
    #[inline]
    pub(crate) fn pages_for_default(&self, stream: PageStream) -> &[PageId] {
        match stream {
            PageStream::Base => &self.default_base,
            PageStream::Reifier => &self.default_reifier,
            PageStream::Annotation => &self.default_annotation,
        }
    }

    /// Derive a fresh index from `pages`, the ONE producer.
    ///
    /// For each [`PageSlot`] in ascending index order, reads
    /// `slot.translation.summary()` and, for each local graph in
    /// [`declared_graphs`](super::summary::PageSummary::declared_graphs), maps it to
    /// global via [`PageTranslation::to_global`](super::translation::PageTranslation::to_global)
    /// and records the page under that global key for every stream with a nonzero
    /// row count. A key is added even when every stream's count is zero, so a
    /// declared-empty graph is still enumerable via [`keys`](Self::keys). Default-graph
    /// postings are recorded the same way, per stream, from
    /// [`default_rows`](super::summary::PageSummary::default_rows).
    ///
    /// Reads no page's quads (only the already-sealed per-page summary), so this
    /// costs `O(Σ per-page graphs)` and materializes nothing — callers reconstituting
    /// a dataset via [`from_parts`](super::PagedDataset::from_parts) get a correct
    /// index without paying for a page.
    pub(crate) fn derive(pages: &[PageSlot]) -> Self {
        let mut named: BTreeMap<GlobalTermId, GraphPostings> = BTreeMap::new();
        let mut default_base: Vec<PageId> = Vec::new();
        let mut default_reifier: Vec<PageId> = Vec::new();
        let mut default_annotation: Vec<PageId> = Vec::new();

        for slot in pages {
            let summary = slot.translation.summary();
            for local_graph in summary.declared_graphs() {
                let global_graph = slot.translation.to_global(local_graph);
                let postings = named.entry(global_graph).or_default();
                if summary.graph_rows(local_graph, PageStream::Base) > 0 {
                    postings.base.push(slot.id);
                }
                if summary.graph_rows(local_graph, PageStream::Reifier) > 0 {
                    postings.reifier.push(slot.id);
                }
                if summary.graph_rows(local_graph, PageStream::Annotation) > 0 {
                    postings.annotation.push(slot.id);
                }
            }
            if summary.default_rows(PageStream::Base) > 0 {
                default_base.push(slot.id);
            }
            if summary.default_rows(PageStream::Reifier) > 0 {
                default_reifier.push(slot.id);
            }
            if summary.default_rows(PageStream::Annotation) > 0 {
                default_annotation.push(slot.id);
            }
        }

        let keys: Box<[GlobalTermId]> = named.keys().copied().collect();
        let base = build_csr(&named, |p| &p.base);
        let reifier = build_csr(&named, |p| &p.reifier);
        let annotation = build_csr(&named, |p| &p.annotation);

        Self {
            keys,
            base,
            reifier,
            annotation,
            default_base: default_base.into_boxed_slice(),
            default_reifier: default_reifier.into_boxed_slice(),
            default_annotation: default_annotation.into_boxed_slice(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::sync::Arc;

    use super::super::provider::{CountingDemandProvider, InMemoryPageProvider, PageProvider};
    use super::super::summary::PageStream;
    use super::PageId;
    use crate::dataset_view::DatasetView;
    use crate::ir::paged::PagedDataset;
    use crate::ir::{RdfDatasetBuilder, TermValue};

    /// An `example.org` IRI value.
    fn iri(name: &str) -> TermValue {
        TermValue::iri(format!("http://example.org/{name}"))
    }

    #[test]
    fn derive_indexes_every_stream_and_declared_empty_graphs() {
        // Page 0: a base row in graph A.
        let page0 = {
            let mut b = RdfDatasetBuilder::new();
            let s = b.intern_iri("http://example.org/s0");
            let p = b.intern_iri("http://example.org/p");
            let o = b.intern_iri("http://example.org/o0");
            let ga = b.intern_iri("http://example.org/gA");
            b.push_quad(s, p, o, Some(ga));
            b.freeze().expect("page0 freeze")
        };
        // Page 1: a base row in graph B, and a reifier row in graph A.
        let page1 = {
            let mut b = RdfDatasetBuilder::new();
            let s = b.intern_iri("http://example.org/s1");
            let p = b.intern_iri("http://example.org/p");
            let o = b.intern_iri("http://example.org/o1");
            let gb = b.intern_iri("http://example.org/gB");
            b.push_quad(s, p, o, Some(gb));
            let a = b.intern_iri("http://example.org/a");
            let bb = b.intern_iri("http://example.org/b");
            let c = b.intern_iri("http://example.org/c");
            let triple = b.intern_triple(a, bb, c);
            let r = b.intern_iri("http://example.org/r");
            let ga = b.intern_iri("http://example.org/gA");
            b.push_reifier_in_graph(r, triple, Some(ga));
            b.freeze().expect("page1 freeze")
        };
        // Page 2: graph C declared but empty, plus a default-graph base row.
        let page2 = {
            let mut b = RdfDatasetBuilder::new();
            let gc = b.intern_iri("http://example.org/gC");
            b.declare_named_graph(gc);
            let s = b.intern_iri("http://example.org/s2");
            let p = b.intern_iri("http://example.org/p");
            let o = b.intern_iri("http://example.org/o2");
            b.push_quad(s, p, o, None);
            b.freeze().expect("page2 freeze")
        };

        let provider = Arc::new(InMemoryPageProvider::new(vec![page0, page1, page2]));
        let paged = PagedDataset::from_provider(provider).expect("seal pages");

        let ga = paged.term_id_by_value(&iri("gA")).expect("gA interned");
        let gb = paged.term_id_by_value(&iri("gB")).expect("gB interned");
        let gc = paged.term_id_by_value(&iri("gC")).expect("gC interned");

        let index = paged.graph_index();
        let mut expected_keys = [ga, gb, gc];
        expected_keys.sort_unstable();
        assert_eq!(
            index.keys(),
            &expected_keys[..],
            "keys() is every declared graph, ascending, including the declared-empty one"
        );

        assert_eq!(
            index.pages_for_named(ga, PageStream::Base),
            &[PageId(0)],
            "only page 0 carries a base row in graph A"
        );
        assert_eq!(
            index.pages_for_named(ga, PageStream::Reifier),
            &[PageId(1)],
            "only page 1 carries a reifier row in graph A"
        );
        assert!(
            index.pages_for_named(gc, PageStream::Base).is_empty(),
            "graph C is declared but carries no base rows on any page"
        );
        assert_eq!(
            index.pages_for_default(PageStream::Base),
            &[PageId(2)],
            "only page 2 carries a default-graph base row"
        );
    }

    #[test]
    fn derive_survives_compaction_and_page_subsetting() {
        let page0 = {
            let mut b = RdfDatasetBuilder::new();
            let s = b.intern_iri("http://example.org/s0");
            let p = b.intern_iri("http://example.org/p");
            let o = b.intern_iri("http://example.org/o0");
            let ga = b.intern_iri("http://example.org/gA");
            b.push_quad(s, p, o, Some(ga));
            b.freeze().expect("page0 freeze")
        };
        let page1 = {
            let mut b = RdfDatasetBuilder::new();
            let s = b.intern_iri("http://example.org/s1");
            let p = b.intern_iri("http://example.org/p");
            let o = b.intern_iri("http://example.org/o1");
            let gb = b.intern_iri("http://example.org/gB");
            b.push_quad(s, p, o, Some(gb));
            let a = b.intern_iri("http://example.org/a");
            let bb = b.intern_iri("http://example.org/b");
            let c = b.intern_iri("http://example.org/c");
            let triple = b.intern_triple(a, bb, c);
            let r = b.intern_iri("http://example.org/r");
            let ga = b.intern_iri("http://example.org/gA");
            b.push_reifier_in_graph(r, triple, Some(ga));
            b.freeze().expect("page1 freeze")
        };

        let provider = Arc::new(InMemoryPageProvider::new(vec![page0, page1]));
        let paged = PagedDataset::from_provider(provider).expect("seal pages");

        let original_values: BTreeSet<String> = paged
            .graph_index()
            .keys()
            .iter()
            .map(|&id| format!("{:?}", paged.dictionary().term_value(id)))
            .collect();
        assert_eq!(
            original_values,
            BTreeSet::from([format!("{:?}", iri("gA")), format!("{:?}", iri("gB"))]),
            "the index enumerates graphs A and B before compaction"
        );

        // Compaction renumbers global ids; the index must be rebuilt and resolve to
        // the SAME graph VALUES even though the ids themselves changed.
        let compacted = paged.compact();
        let compacted_values: BTreeSet<String> = compacted
            .graph_index()
            .keys()
            .iter()
            .map(|&id| format!("{:?}", compacted.dictionary().term_value(id)))
            .collect();
        assert_eq!(
            compacted_values, original_values,
            "compaction preserves the graph VALUE set even though ids are renumbered"
        );

        // Page subsetting: keep only the original page 1 (base row in graph B,
        // reifier row in graph A). It is renumbered to PageId(0), and the index's
        // postings must name the NEW id, not the old one.
        let subset = paged.with_pages(&[PageId(1)]);
        assert_eq!(subset.page_count(), 1);
        let ga = subset.term_id_by_value(&iri("gA")).expect("gA interned");
        let gb = subset.term_id_by_value(&iri("gB")).expect("gB interned");
        assert_eq!(
            subset
                .graph_index()
                .pages_for_named(ga, PageStream::Reifier),
            &[PageId(0)],
            "the surviving page is renumbered to PageId(0) in the rebuilt index"
        );
        assert_eq!(
            subset.graph_index().pages_for_named(gb, PageStream::Base),
            &[PageId(0)],
        );
    }

    #[test]
    fn derive_materializes_no_page() {
        let page = {
            let mut b = RdfDatasetBuilder::new();
            let s = b.intern_iri("http://example.org/s");
            let p = b.intern_iri("http://example.org/p");
            let o = b.intern_iri("http://example.org/o");
            let g = b.intern_iri("http://example.org/g");
            b.push_quad(s, p, o, Some(g));
            b.freeze().expect("page freeze")
        };
        let eager =
            PagedDataset::from_provider(Arc::new(InMemoryPageProvider::new(vec![page.clone()])))
                .expect("seal page");
        let (dictionary, generation, parts) = eager.to_parts();

        let counting = Arc::new(CountingDemandProvider::new(vec![Box::new(move || {
            Arc::clone(&page)
        })]));
        let warm = PagedDataset::from_parts(
            dictionary,
            counting.clone() as Arc<dyn PageProvider>,
            generation,
            parts,
        )
        .expect("matching warm snapshot");

        assert_eq!(
            counting.hits(),
            0,
            "deriving the graph index from PagePart summaries must not materialize a page"
        );
        assert_eq!(warm.page_count(), 1);
    }
}
