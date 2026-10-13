// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Public layered paging, chronology, RDF 1.2 classification, and fold laws.

use std::collections::BTreeSet;
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use purrdf_core::{
    BlankScope, CanonicalPagedError, DatasetView, FallibleDatasetView, GlobalTermId, GraphMatch,
    InMemoryPageProvider, PackBuilder, PageFault, PageGeneration, PageId, PageMaterialization,
    PageProvider, PagedDataset, PagedQueryError, PagedQueryLimits, PagedStack, PagedStackError,
    PagedStackEvidence, RdfDataset, RdfDatasetBuilder, RdfTextDirection, StackPageOrigin, TermBox,
    TermFactory, TermValue, ViewOperationStatus, canonical_paged_seal,
};

fn iri(name: &str) -> TermValue {
    TermValue::iri(format!("https://example.org/{name}"))
}
fn row(s: &str, o: &str) -> purrdf_core::QuadValues {
    purrdf_core::QuadValues::triple(iri(s), iri("p"), iri(o))
}
fn page(rows: &[purrdf_core::QuadValues]) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    for row in rows {
        let s = builder.intern_value(&row.s);
        let p = builder.intern_value(&row.p);
        let o = builder.intern_value(&row.o);
        let g = row.g.as_ref().map(|g| builder.intern_value(g));
        builder.push_quad(s, p, o, g);
    }
    builder.freeze().unwrap()
}
fn sealed(pages: Vec<Arc<RdfDataset>>) -> Arc<PagedDataset> {
    Arc::new(PagedDataset::from_provider(Arc::new(InMemoryPageProvider::new(pages))).unwrap())
}
fn bound(n: usize) -> NonZeroUsize {
    NonZeroUsize::new(n).unwrap()
}
fn evidence(
    view: &impl FallibleDatasetView<Error = PagedQueryError, Evidence = PagedStackEvidence>,
) -> PagedStackEvidence {
    match view.operation_status() {
        ViewOperationStatus::Ready { evidence } => evidence,
        ViewOperationStatus::Failed { error, .. } => panic!("incomplete: {error}"),
    }
}
fn surface<D: DatasetView>(view: &D) -> BTreeSet<purrdf_core::QuadValues> {
    view.quads()
        .chain(view.reifier_quads())
        .chain(view.annotation_quads())
        .map(|q| purrdf_core::QuadValues {
            s: view.term_value(q.s).unwrap(),
            p: view.term_value(q.p).unwrap(),
            o: view.term_value(q.o).unwrap(),
            g: q.g.map(|g| view.term_value(g).unwrap()),
        })
        .collect()
}
fn page_bytes(dataset: &PagedDataset) -> Vec<Vec<u8>> {
    (0..dataset.page_count())
        .map(|i| {
            let part = dataset.with_pages(&[PageId(i)]);
            PackBuilder::build_view_bytes(&part.query_view(PagedQueryLimits::UNBOUNDED)).unwrap()
        })
        .collect()
}
fn dictionary_values(dataset: &PagedDataset) -> Vec<TermValue> {
    (0..dataset.dictionary().len())
        .map(|i| {
            dataset
                .dictionary()
                .term_value(GlobalTermId::from_index(i as u64))
        })
        .collect()
}

struct ProbeProvider {
    pages: Vec<Arc<RdfDataset>>,
    alternate: Option<Arc<RdfDataset>>,
    reads: AtomicUsize,
    generation: AtomicU64,
    count: AtomicU64,
    mode: AtomicUsize,
    generation_reads: AtomicUsize,
    count_reads: AtomicUsize,
}
impl ProbeProvider {
    fn new(pages: Vec<Arc<RdfDataset>>, alternate: Option<Arc<RdfDataset>>) -> Self {
        Self {
            count: AtomicU64::new(pages.len() as u64),
            pages,
            alternate,
            reads: AtomicUsize::new(0),
            generation: AtomicU64::new(7),
            mode: AtomicUsize::new(0),
            generation_reads: AtomicUsize::new(0),
            count_reads: AtomicUsize::new(0),
        }
    }
}
impl PageProvider for ProbeProvider {
    fn page_count(&self) -> u64 {
        self.count_reads.fetch_add(1, Ordering::Relaxed);
        self.count.load(Ordering::Relaxed)
    }
    fn generation(&self) -> PageGeneration {
        self.generation_reads.fetch_add(1, Ordering::Relaxed);
        PageGeneration(self.generation.load(Ordering::Relaxed))
    }
    fn materialize(&self, id: PageId) -> Result<PageMaterialization, PageFault> {
        self.reads.fetch_add(1, Ordering::Relaxed);
        let mode = self.mode.load(Ordering::Relaxed);
        match mode {
            1 => return Err(PageFault::provider(id, "provider refusal")),
            2 => return Err(PageFault::cancelled(id, "cancelled")),
            3 => return Err(PageFault::deadline_exceeded(id, "deadline")),
            _ => {}
        }
        let dataset = if mode == 5 {
            self.alternate.as_ref().unwrap()
        } else {
            &self.pages[id.0 as usize]
        };
        if mode == 6 {
            self.generation.fetch_add(1, Ordering::Relaxed);
        }
        Ok(PageMaterialization::new(
            Arc::clone(dataset),
            PageGeneration(7),
            if mode == 4 { 11 } else { 10 },
        ))
    }
}

#[test]
fn construction_append_and_snapshot_do_not_read_lower_pages() {
    let a = Arc::new(ProbeProvider::new(vec![page(&[row("a", "b")])], None));
    let b = Arc::new(ProbeProvider::new(vec![page(&[row("b", "c")])], None));
    let first = Arc::new(PagedDataset::from_provider(a.clone()).unwrap());
    let second = Arc::new(PagedDataset::from_provider(b.clone()).unwrap());
    assert_eq!(
        first.dictionary().term_value(GlobalTermId::from_index(0)),
        iri("a")
    );
    assert_eq!(
        second.dictionary().term_value(GlobalTermId::from_index(0)),
        iri("b")
    );
    a.reads.store(0, Ordering::Relaxed);
    b.reads.store(0, Ordering::Relaxed);
    let mut stack = PagedStack::new(vec![first]).unwrap();
    stack.append(second, vec![]).unwrap();
    let snapshot = stack.snapshot().unwrap();
    assert_eq!(a.reads.load(Ordering::Relaxed), 0);
    assert_eq!(b.reads.load(Ordering::Relaxed), 0);
    let view = snapshot.query_view(PagedQueryLimits::new(2, 20));
    assert_eq!(
        surface(&view),
        BTreeSet::from([row("a", "b"), row("b", "c")])
    );
    let receipt = evidence(&view);
    assert_eq!(receipt.sources.len(), 2);
    assert_eq!(
        receipt.requested_origins,
        vec![
            StackPageOrigin::Sealed {
                layer: 1,
                page: PageId(0)
            },
            StackPageOrigin::Sealed {
                layer: 0,
                page: PageId(0)
            }
        ]
    );
    assert_eq!(
        (receipt.pages.consumed_pages, receipt.pages.consumed_bytes),
        (2, 20)
    );
    assert_eq!(surface(&view).len(), 2);
    assert_eq!(evidence(&view), receipt);
}

#[test]
fn chronological_removal_reinsertion_same_head_and_snapshot_isolation() {
    let q = row("a", "b");
    let base = sealed(vec![page(std::slice::from_ref(&q))]);
    let mut stack = PagedStack::new(vec![Arc::clone(&base), base]).unwrap();
    let original = stack.snapshot().unwrap();
    assert!(!stack.insert(q.clone()).unwrap());
    assert!(stack.remove(&q).unwrap());
    assert!(!stack.remove(&q).unwrap());
    stack.seal_head(bound(2)).unwrap();
    let removed = stack.snapshot().unwrap();
    assert_eq!(removed.sealed_depth(), 3);
    assert!(surface(&removed.query_view(PagedQueryLimits::UNBOUNDED)).is_empty());
    assert!(stack.insert(q.clone()).unwrap());
    assert!(stack.remove(&q).unwrap());
    assert!(stack.insert(q.clone()).unwrap());
    let reinserted = stack.snapshot().unwrap();
    stack.seal_head(bound(1)).unwrap();
    assert!(stack.remove(&q).unwrap());
    for snapshot in [original, reinserted] {
        let view = snapshot.query_view(PagedQueryLimits::UNBOUNDED);
        assert_eq!(surface(&view), BTreeSet::from([q.clone()]));
        assert_eq!(view.quads().count(), 1);
        evidence(&view);
    }
    assert!(!stack.contains(&q).unwrap());
    let view = removed.query_view(PagedQueryLimits::UNBOUNDED);
    assert_eq!(
        evidence(&view).removals[2].as_ref(),
        std::slice::from_ref(&q)
    );
}

#[test]
fn head_charges_once_under_global_limits_and_folds_have_no_partial_success() {
    let a = Arc::new(ProbeProvider::new(vec![page(&[row("a", "b")])], None));
    let base = Arc::new(PagedDataset::from_provider(a).unwrap());
    let mut stack = PagedStack::new(vec![base]).unwrap();
    stack.insert(row("new", "term")).unwrap();
    let snapshot = stack.snapshot().unwrap();
    let unbounded = snapshot.query_view(PagedQueryLimits::UNBOUNDED);
    assert_eq!(surface(&unbounded).len(), 2);
    let receipt = evidence(&unbounded);
    assert!(receipt.head_bytes > 0);
    assert_eq!(
        receipt.requested_origins.first(),
        Some(&StackPageOrigin::Head)
    );
    assert_eq!(receipt.head.as_ref(), &[row("new", "term")]);
    let exact = snapshot.query_view(PagedQueryLimits::new(2, 10 + receipt.head_bytes));
    assert_eq!(surface(&exact).len(), 2);
    assert_eq!(
        evidence(&exact).pages.consumed_bytes,
        10 + receipt.head_bytes
    );
    let zero = snapshot.query_view(PagedQueryLimits::new(0, 0));
    assert_eq!(zero.quads().count(), 0);
    assert!(matches!(
        zero.operation_status(),
        ViewOperationStatus::Failed {
            error: PagedQueryError::PageBudgetExceeded { .. },
            ..
        }
    ));
    let below = snapshot.query_view(PagedQueryLimits::new(2, 9 + receipt.head_bytes));
    assert_eq!(below.quads().count(), 1);
    assert!(matches!(
        below.operation_status(),
        ViewOperationStatus::Failed {
            error: PagedQueryError::ByteBudgetExceeded { .. },
            ..
        }
    ));
    assert_eq!(below.annotation_quads().count(), 0);
    assert_eq!(below.named_graphs().count(), 0);
    assert!(matches!(
        snapshot.compact(PagedQueryLimits::new(1, u64::MAX), bound(2)),
        Err(CanonicalPagedError::Read(
            PagedQueryError::PageBudgetExceeded { .. }
        ))
    ));
}

#[test]
fn drift_of_skipped_or_zero_page_sources_gates_the_entire_head_and_metadata() {
    for pages in [vec![], vec![page(&[row("skipped", "o")])]] {
        let provider = Arc::new(ProbeProvider::new(pages, None));
        let base = Arc::new(PagedDataset::from_provider(provider.clone()).unwrap());
        let mut stack = PagedStack::new(vec![base]).unwrap();
        stack.insert(row("head", "o")).unwrap();
        stack.declare_named_graph(iri("empty")).unwrap();
        let snapshot = stack.snapshot().unwrap();
        provider.reads.store(0, Ordering::Relaxed);
        provider.generation.store(8, Ordering::Relaxed);
        let view = snapshot.query_view(PagedQueryLimits::UNBOUNDED);
        assert!(matches!(
            view.operation_status(),
            ViewOperationStatus::Failed {
                error: PagedQueryError::SourceSnapshot { source: 0, .. },
                ..
            }
        ));
        assert_eq!(view.quads().count(), 0);
        assert_eq!(view.named_graphs().count(), 0);
        assert!(view.term_id_by_value(&iri("head")).is_err());
        assert_eq!(provider.reads.load(Ordering::Relaxed), 0);
        assert!(stack.contains(&row("absent", "o")).is_err());
    }
}

#[test]
fn page_count_drift_is_checked_without_materialization_and_is_sticky() {
    let provider = Arc::new(ProbeProvider::new(vec![], None));
    let mut stack = PagedStack::new(vec![Arc::new(
        PagedDataset::from_provider(provider.clone()).unwrap(),
    )])
    .unwrap();
    stack.insert(row("head", "o")).unwrap();
    let snapshot = stack.snapshot().unwrap();
    let view = snapshot.query_view(PagedQueryLimits::UNBOUNDED);
    provider.count.store(1, Ordering::Relaxed);
    let failed = view.operation_status();
    assert!(
        matches!(&failed, ViewOperationStatus::Failed { error: PagedQueryError::SourceSnapshot { error, .. }, .. } if matches!(**error, PagedQueryError::PageCountMismatch { expected: 0, actual: 1 }))
    );
    provider.count.store(0, Ordering::Relaxed);
    assert_eq!(view.operation_status(), failed);
    assert_eq!(view.quads().count(), 0);
}

#[test]
fn typed_faults_changed_charges_and_layouts_refuse_reads_mutation_and_fold() {
    for mode in 1..=6 {
        let provider = Arc::new(ProbeProvider::new(
            vec![page(&[row("a", "b")])],
            Some(page(&[row("x", "y")])),
        ));
        let mut stack = PagedStack::new(vec![Arc::new(
            PagedDataset::from_provider(provider.clone()).unwrap(),
        )])
        .unwrap();
        stack.insert(row("head", "o")).unwrap();
        let snapshot = stack.snapshot().unwrap();
        provider.mode.store(mode, Ordering::Relaxed);
        let view = snapshot.query_view(PagedQueryLimits::UNBOUNDED);
        assert_eq!(
            view.quads().count(),
            1,
            "head precedes fault; the failed checkpoint rejects this partial internal row"
        );
        let status = view.operation_status();
        assert!(matches!(status, ViewOperationStatus::Failed { .. }));
        assert_eq!(view.annotation_quads().count(), 0);
        assert_eq!(view.reifier_quads().count(), 0);
        assert_eq!(view.named_graphs().count(), 0);
        assert!(stack.remove(&row("a", "b")).is_err());
        assert!(
            snapshot
                .compact(PagedQueryLimits::UNBOUNDED, bound(1))
                .is_err()
        );
        assert_eq!(
            matches!(
                status,
                ViewOperationStatus::Failed {
                    error: PagedQueryError::Provider { .. },
                    ..
                }
            ),
            mode == 1
        );
        assert_eq!(
            matches!(
                status,
                ViewOperationStatus::Failed {
                    error: PagedQueryError::Stopped { .. },
                    ..
                }
            ),
            mode == 2 || mode == 3
        );
        assert_eq!(
            matches!(
                status,
                ViewOperationStatus::Failed {
                    error: PagedQueryError::InvalidData { .. },
                    ..
                }
            ),
            mode == 4 || mode == 5
        );
    }
}

fn reifier_rows() -> (
    purrdf_core::QuadValues,
    purrdf_core::QuadValues,
    purrdf_core::QuadValues,
) {
    let triple = TermValue::Triple {
        s: TermBox::new(iri("s")),
        p: TermBox::new(iri("p")),
        o: TermBox::new(iri("o")),
    };
    let first = purrdf_core::QuadValues {
        s: iri("r"),
        p: TermValue::iri(purrdf_core::vocab::rdf::REIFIES),
        o: triple.clone(),
        g: Some(iri("g")),
    };
    let second = purrdf_core::QuadValues {
        o: TermValue::Triple {
            s: TermBox::new(iri("other")),
            p: TermBox::new(iri("p")),
            o: TermBox::new(triple),
        },
        ..first.clone()
    };
    let annotation = purrdf_core::QuadValues {
        s: iri("r"),
        p: iri("p"),
        o: iri("value"),
        g: Some(iri("g")),
    };
    (first, second, annotation)
}
fn typed_page(
    reifiers: &[purrdf_core::QuadValues],
    annotations: &[purrdf_core::QuadValues],
) -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    for q in reifiers {
        let s = b.intern_value(&q.s);
        let o = b.intern_value(&q.o);
        let g = q.g.as_ref().map(|g| b.intern_value(g));
        b.push_reifier_in_graph(s, o, g);
    }
    for q in annotations {
        let s = b.intern_value(&q.s);
        let p = b.intern_value(&q.p);
        let o = b.intern_value(&q.o);
        let g = q.g.as_ref().map(|g| b.intern_value(g));
        b.push_annotation_in_graph(s, p, o, g);
    }
    b.freeze().unwrap()
}

#[test]
fn annotation_only_pages_demote_only_after_last_original_reifier_in_the_graph() {
    let (first, second, annotation) = reifier_rows();
    let mut stack = PagedStack::new(vec![sealed(vec![
        typed_page(&[first.clone(), second.clone()], &[]),
        typed_page(&[], std::slice::from_ref(&annotation)),
    ])])
    .unwrap();
    let original = stack.snapshot().unwrap();
    let original_view = original.query_view(PagedQueryLimits::UNBOUNDED);
    assert_eq!(original_view.annotation_quads().count(), 1);
    assert!(stack.remove(&first).unwrap());
    stack.seal_head(bound(1)).unwrap();
    let one = stack.snapshot().unwrap();
    assert_eq!(
        one.query_view(PagedQueryLimits::UNBOUNDED)
            .annotation_quads()
            .count(),
        1
    );
    assert!(stack.remove(&second).unwrap());
    let last = stack.snapshot().unwrap();
    let view = last.query_view(PagedQueryLimits::UNBOUNDED);
    let r = view.term_id_by_value(&iri("r")).unwrap().unwrap();
    let p = view.term_id_by_value(&iri("p")).unwrap().unwrap();
    let g = view.term_id_by_value(&iri("g")).unwrap().unwrap();
    assert_eq!(
        view.quads_for_pattern(Some(r), Some(p), None, GraphMatch::Named(g))
            .count(),
        1
    );
    assert_eq!(view.annotation_quads().count(), 0);
    assert_eq!(surface(&view), BTreeSet::from([annotation.clone()]));
    assert!(view.cardinality_estimate(Some(r), Some(p), None, GraphMatch::Named(g)) >= 1);
    evidence(&view);
    let orphan_stack = PagedStack::new(vec![sealed(vec![typed_page(
        &[],
        std::slice::from_ref(&annotation),
    )])])
    .unwrap();
    let orphan = orphan_stack.snapshot().unwrap();
    assert_eq!(
        orphan
            .query_view(PagedQueryLimits::UNBOUNDED)
            .quads()
            .count(),
        0
    );
    assert_eq!(
        orphan
            .query_view(PagedQueryLimits::UNBOUNDED)
            .annotation_quads()
            .count(),
        1
    );
}

#[test]
fn new_reifiers_promote_older_primary_rows_only_in_the_matching_graph() {
    let (first, _, annotation) = reifier_rows();
    let default = purrdf_core::QuadValues {
        g: None,
        ..annotation.clone()
    };
    let elsewhere = purrdf_core::QuadValues {
        g: Some(iri("elsewhere")),
        ..annotation.clone()
    };
    let mut stack = PagedStack::new(vec![sealed(vec![page(&[
        annotation.clone(),
        default.clone(),
        elsewhere.clone(),
    ])])])
    .unwrap();
    stack.insert(first.clone()).unwrap();
    let snapshot = stack.snapshot().unwrap();
    let view = snapshot.query_view(PagedQueryLimits::UNBOUNDED);
    assert_eq!(view.quads().count(), 2);
    assert_eq!(view.annotation_quads().count(), 1);
    assert_eq!(
        surface(&view),
        BTreeSet::from([first, annotation, default, elsewhere])
    );
    evidence(&view);
}

#[test]
fn graph_pruning_partial_removals_empty_declarations_and_scoped_head_values_survive() {
    let q = purrdf_core::QuadValues {
        g: Some(iri("implicit")),
        ..row("a", "b")
    };
    let mut b = RdfDatasetBuilder::new();
    let empty = b.intern_value(&iri("empty"));
    b.declare_named_graph(empty);
    let provider = Arc::new(ProbeProvider::new(
        vec![
            page(std::slice::from_ref(&q)),
            b.freeze().unwrap(),
            page(&[row("unrelated", "o")]),
        ],
        None,
    ));
    let mut stack = PagedStack::new(vec![Arc::new(
        PagedDataset::from_provider(provider.clone()).unwrap(),
    )])
    .unwrap();
    stack.remove(&q).unwrap();
    let scoped = purrdf_core::QuadValues {
        s: TermValue::Blank {
            label: "head".into(),
            scope: BlankScope(44),
        },
        ..row("unused", "o")
    };
    stack.insert(scoped.clone()).unwrap();
    stack.declare_named_graph(iri("head-empty")).unwrap();
    let snapshot = stack.snapshot().unwrap();
    let view = snapshot.query_view(PagedQueryLimits::UNBOUNDED);
    assert!(surface(&view).contains(&scoped));
    let graphs: BTreeSet<_> = view
        .named_graphs()
        .map(|g| view.term_value(g).unwrap())
        .collect();
    assert_eq!(graphs, BTreeSet::from([iri("empty"), iri("head-empty")]));
    evidence(&view);
    provider.reads.store(0, Ordering::Relaxed);
    let narrowed = snapshot.query_view(PagedQueryLimits::UNBOUNDED);
    let s = narrowed
        .term_id_by_value(&iri("unrelated"))
        .unwrap()
        .unwrap();
    assert_eq!(
        narrowed
            .quads_for_pattern(Some(s), None, None, GraphMatch::Default)
            .count(),
        1
    );
    assert_eq!(provider.reads.load(Ordering::Relaxed), 1);
    assert_eq!(
        evidence(&narrowed).requested_origins,
        vec![StackPageOrigin::Sealed {
            layer: 0,
            page: PageId(2)
        }]
    );
}

#[test]
fn canonical_fold_matches_independent_eager_typed_surface_and_dictionary_for_two_histories_and_bounds()
 {
    let (first, _, annotation) = reifier_rows();
    let directional = TermValue::Literal {
        lexical_form: "مرحبا".into(),
        datatype: purrdf_core::vocab::rdf::DIR_LANG_STRING.into(),
        language: Some("ar".into()),
        direction: Some(RdfTextDirection::Rtl),
    };
    let blank = TermValue::Blank {
        label: "same".into(),
        scope: BlankScope(17),
    };
    let nested = TermValue::Triple {
        s: TermBox::new(blank.clone()),
        p: TermBox::new(iri("p")),
        o: TermBox::new(first.o.clone()),
    };
    let special = purrdf_core::QuadValues::triple(blank, iri("p"), nested);
    let literal = purrdf_core::QuadValues::triple(iri("literal"), iri("p"), directional);
    let ordinary = page(&[special.clone(), literal.clone(), row("a", "b")]);
    let typed = typed_page(
        std::slice::from_ref(&first),
        std::slice::from_ref(&annotation),
    );
    let mut oracle = RdfDatasetBuilder::new();
    purrdf_core::ir::import::DatasetImporter::new(&mut oracle, ordinary.as_ref()).append();
    purrdf_core::ir::import::DatasetImporter::new(&mut oracle, typed.as_ref()).append();
    let graph = oracle.intern_value(&iri("empty"));
    oracle.declare_named_graph(graph);
    let oracle = oracle.freeze().unwrap();
    for n in [1, 3] {
        let eager = canonical_paged_seal(oracle.as_ref(), bound(n)).unwrap();
        for history in 0..2 {
            let mut stack = if history == 0 {
                PagedStack::new(vec![
                    sealed(vec![Arc::clone(&ordinary)]),
                    sealed(vec![Arc::clone(&typed)]),
                ])
                .unwrap()
            } else {
                let mut stack =
                    PagedStack::new(vec![sealed(vec![page(&[row("removed", "only")])])]).unwrap();
                stack.remove(&row("removed", "only")).unwrap();
                stack.seal_head(bound(2)).unwrap();
                stack
                    .append(
                        sealed(vec![typed_page(&[], std::slice::from_ref(&annotation))]),
                        vec![],
                    )
                    .unwrap();
                stack
                    .append(
                        sealed(vec![typed_page(std::slice::from_ref(&first), &[])]),
                        vec![],
                    )
                    .unwrap();
                for row in [literal.clone(), row("a", "b"), special.clone()] {
                    stack.insert(row).unwrap();
                }
                stack.seal_head(bound(1)).unwrap();
                stack
            };
            stack.declare_named_graph(iri("empty")).unwrap();
            let compacted = stack
                .compact(PagedQueryLimits::UNBOUNDED, bound(n))
                .unwrap();
            assert_eq!(page_bytes(&compacted), page_bytes(&eager));
            assert_eq!(dictionary_values(&compacted), dictionary_values(&eager));
            assert_eq!(
                surface(&compacted.query_view(PagedQueryLimits::UNBOUNDED)),
                surface(oracle.as_ref())
            );
            // Canonical partitioning can separate annotations and declarations.
            let roundtrip = PagedStack::new(vec![Arc::new(compacted)])
                .unwrap()
                .snapshot()
                .unwrap();
            assert_eq!(
                surface(&roundtrip.query_view(PagedQueryLimits::UNBOUNDED)),
                surface(oracle.as_ref())
            );
        }
    }
    let empty = RdfDatasetBuilder::new().freeze().unwrap();
    assert_eq!(
        canonical_paged_seal(empty.as_ref(), bound(1))
            .unwrap()
            .page_count(),
        0
    );
}

#[test]
fn invalid_rows_never_mutate_the_head_and_pending_head_never_reorders() {
    assert!(matches!(
        PagedStack::new(vec![]),
        Err(PagedStackError::MissingBase)
    ));
    let mut stack = PagedStack::new(vec![sealed(vec![])]).unwrap();
    let invalid = purrdf_core::QuadValues {
        p: TermValue::blank("predicate"),
        ..row("a", "b")
    };
    assert!(stack.insert(invalid).is_err());
    assert!(
        surface(
            &stack
                .snapshot()
                .unwrap()
                .query_view(PagedQueryLimits::UNBOUNDED)
        )
        .is_empty()
    );
    stack.insert(row("a", "b")).unwrap();
    assert!(matches!(
        stack.append(sealed(vec![]), vec![]),
        Err(PagedStackError::PendingHead)
    ));
}

#[test]
fn newer_typed_occurrence_wins_before_partition_and_native_primary_typing_is_retained() {
    let (declaration, _, q) = reifier_rows();
    for reverse in [false, true] {
        let primary = sealed(vec![page(std::slice::from_ref(&q))]);
        let annotation = sealed(vec![typed_page(&[], std::slice::from_ref(&q))]);
        let bases = if reverse {
            vec![annotation, primary]
        } else {
            vec![primary, annotation]
        };
        let snapshot = PagedStack::new(bases).unwrap().snapshot().unwrap();
        let view = snapshot.query_view(PagedQueryLimits::UNBOUNDED);
        assert_eq!(view.quads().count(), usize::from(reverse));
        assert_eq!(view.annotation_quads().count(), usize::from(!reverse));
        assert_eq!(surface(&view).len(), 1);
        let expected = if reverse {
            page(std::slice::from_ref(&q))
        } else {
            typed_page(&[], std::slice::from_ref(&q))
        };
        assert_eq!(
            page_bytes(
                &snapshot
                    .compact(PagedQueryLimits::UNBOUNDED, bound(1))
                    .unwrap()
            ),
            page_bytes(&canonical_paged_seal(expected.as_ref(), bound(1)).unwrap())
        );
    }
    let mut builder = RdfDatasetBuilder::new();
    purrdf_core::ir::import::DatasetImporter::new(
        &mut builder,
        page(std::slice::from_ref(&q)).as_ref(),
    )
    .append();
    purrdf_core::ir::import::DatasetImporter::new(
        &mut builder,
        typed_page(std::slice::from_ref(&declaration), &[]).as_ref(),
    )
    .append();
    let native = builder.freeze().unwrap();
    let snapshot = PagedStack::new(vec![sealed(vec![Arc::clone(&native)])])
        .unwrap()
        .snapshot()
        .unwrap();
    let view = snapshot.query_view(PagedQueryLimits::UNBOUNDED);
    assert_eq!(view.quads().count(), native.as_ref().quad_count());
    assert_eq!(view.annotation_quads().count(), 0);
    assert_eq!(
        page_bytes(
            &snapshot
                .compact(PagedQueryLimits::UNBOUNDED, bound(2))
                .unwrap()
        ),
        page_bytes(&canonical_paged_seal(native.as_ref(), bound(2)).unwrap())
    );
}

#[test]
fn native_equal_primary_and_virtual_reifier_rows_retain_both_streams() {
    let (declaration, _, _) = reifier_rows();
    let mut builder = RdfDatasetBuilder::new();
    for source in [
        page(std::slice::from_ref(&declaration)),
        typed_page(std::slice::from_ref(&declaration), &[]),
    ] {
        purrdf_core::ir::import::DatasetImporter::new(&mut builder, source.as_ref()).append();
    }
    let native = builder.freeze().unwrap();
    let snapshot = PagedStack::new(vec![sealed(vec![Arc::clone(&native)])])
        .unwrap()
        .snapshot()
        .unwrap();
    let view = snapshot.query_view(PagedQueryLimits::UNBOUNDED);
    assert_eq!(view.quads().count(), 1);
    assert_eq!(view.reifier_quads().count(), 1);
    assert_eq!(view.annotation_quads().count(), 0);
    assert_eq!(surface(&view).len(), 1);
    assert_eq!(
        page_bytes(
            &snapshot
                .compact(PagedQueryLimits::UNBOUNDED, bound(1))
                .unwrap()
        ),
        page_bytes(&canonical_paged_seal(native.as_ref(), bound(1)).unwrap())
    );
}

#[test]
fn sealed_implicit_graph_disappears_when_its_last_effective_row_is_removed() {
    let q = purrdf_core::QuadValues {
        g: Some(iri("implicit")),
        ..row("a", "b")
    };
    let mut stack = PagedStack::new(vec![sealed(vec![])]).unwrap();
    stack.insert(q.clone()).unwrap();
    stack.seal_head(bound(1)).unwrap();
    stack.remove(&q).unwrap();
    let snapshot = stack.snapshot().unwrap();
    let view = snapshot.query_view(PagedQueryLimits::UNBOUNDED);
    assert_eq!(view.named_graphs().count(), 0);
    assert_eq!(
        snapshot
            .compact(PagedQueryLimits::UNBOUNDED, bound(1))
            .unwrap()
            .page_count(),
        0
    );
}

#[test]
fn summary_drift_is_refused_even_with_equal_local_values_counts_and_charges() {
    let provider = Arc::new(ProbeProvider::new(
        vec![page(&[row("a", "a"), row("b", "b")])],
        Some(page(&[row("a", "a"), row("a", "b")])),
    ));
    let base = Arc::new(PagedDataset::from_provider(provider.clone()).unwrap());
    let snapshot = PagedStack::new(vec![base]).unwrap().snapshot().unwrap();
    provider.mode.store(5, Ordering::Relaxed);
    let view = snapshot.query_view(PagedQueryLimits::UNBOUNDED);
    assert_eq!(view.quads().count(), 0);
    assert!(
        matches!(view.operation_status(), ViewOperationStatus::Failed { error: PagedQueryError::InvalidData { message, .. }, .. } if message.contains("summary"))
    );
}

#[test]
fn deterministic_value_set_mutations_match_an_independent_oracle_at_each_snapshot() {
    let initial: Vec<_> = (0..4)
        .map(|i| row(&format!("s{i}"), &format!("o{i}")))
        .collect();
    let mut oracle: BTreeSet<_> = initial.iter().cloned().collect();
    let mut stack = PagedStack::new(vec![sealed(vec![page(&initial)])]).unwrap();
    let mut retained = Vec::new();
    for i in 0..64 {
        let q = row(
            &format!("s{}", (i * 13 + 3) % 9),
            &format!("o{}", (i * 7) % 5),
        );
        let changed = if i % 3 == 0 {
            assert_eq!(stack.remove(&q).unwrap(), oracle.remove(&q));
            false
        } else {
            let expected = oracle.insert(q.clone());
            assert_eq!(stack.insert(q).unwrap(), expected);
            expected
        };
        let _ = changed;
        let snapshot = stack.snapshot().unwrap();
        let view = snapshot.query_view(PagedQueryLimits::UNBOUNDED);
        assert_eq!(surface(&view), oracle);
        assert_eq!(view.quads().count(), oracle.len());
        evidence(&view);
        if i % 8 == 0 {
            retained.push((snapshot, oracle.clone()));
            stack.seal_head(bound(1 + i % 3)).unwrap();
        }
    }
    for (snapshot, expected) in retained {
        let view = snapshot.query_view(PagedQueryLimits::UNBOUNDED);
        assert_eq!(surface(&view), expected);
        evidence(&view);
    }
    let eager = page(&oracle.into_iter().collect::<Vec<_>>());
    for n in [1, 5] {
        assert_eq!(
            page_bytes(
                &stack
                    .compact(PagedQueryLimits::UNBOUNDED, bound(n))
                    .unwrap()
            ),
            page_bytes(&canonical_paged_seal(eager.as_ref(), bound(n)).unwrap())
        );
    }
}

#[test]
fn orphan_annotation_association_uses_its_visible_introduction_prefix() {
    let (declaration, _, annotation) = reifier_rows();
    // A reifier was removed BEFORE an explicitly typed orphan was introduced.
    let mut stack = PagedStack::new(vec![sealed(vec![typed_page(
        std::slice::from_ref(&declaration),
        &[],
    )])])
    .unwrap();
    stack.remove(&declaration).unwrap();
    stack.seal_head(bound(1)).unwrap();
    stack
        .append(
            sealed(vec![typed_page(&[], std::slice::from_ref(&annotation))]),
            vec![],
        )
        .unwrap();
    let snapshot = stack.snapshot().unwrap();
    let view = snapshot.query_view(PagedQueryLimits::UNBOUNDED);
    assert_eq!(view.annotation_quads().count(), 1);
    assert_eq!(view.quads().count(), 0);
    let oracle = typed_page(&[], std::slice::from_ref(&annotation));
    assert_eq!(
        page_bytes(
            &snapshot
                .compact(PagedQueryLimits::UNBOUNDED, bound(1))
                .unwrap()
        ),
        page_bytes(&canonical_paged_seal(oracle.as_ref(), bound(1)).unwrap())
    );
    // An orphan predates a temporarily visible future declaration. Its original
    // association stays absent when that declaration is later removed.
    let mut stack = PagedStack::new(vec![sealed(vec![Arc::clone(&oracle)])]).unwrap();
    stack.insert(declaration.clone()).unwrap();
    stack.seal_head(bound(1)).unwrap();
    stack.remove(&declaration).unwrap();
    let snapshot = stack.snapshot().unwrap();
    assert_eq!(
        snapshot
            .query_view(PagedQueryLimits::UNBOUNDED)
            .annotation_quads()
            .count(),
        1
    );
    assert_eq!(
        snapshot
            .query_view(PagedQueryLimits::UNBOUNDED)
            .quads()
            .count(),
        0
    );
    assert_eq!(
        page_bytes(
            &snapshot
                .compact(PagedQueryLimits::UNBOUNDED, bound(2))
                .unwrap()
        ),
        page_bytes(&canonical_paged_seal(oracle.as_ref(), bound(2)).unwrap())
    );
}

#[test]
fn metadata_point_reads_detect_source_drift_without_an_earlier_status_checkpoint() {
    for kind in 0..3 {
        let provider = Arc::new(ProbeProvider::new(vec![], None));
        let mut stack = PagedStack::new(vec![Arc::new(
            PagedDataset::from_provider(provider.clone()).unwrap(),
        )])
        .unwrap();
        stack.insert(row("head", "value")).unwrap();
        stack.declare_named_graph(iri("empty")).unwrap();
        let snapshot = stack.snapshot().unwrap();
        let id = snapshot
            .dictionary()
            .term_id_by_value(&iri("head"))
            .unwrap();
        let graph = snapshot
            .dictionary()
            .term_id_by_value(&iri("empty"))
            .unwrap();
        let view = snapshot.query_view(PagedQueryLimits::UNBOUNDED);
        provider.generation.store(8, Ordering::Relaxed);
        match kind {
            0 => assert!(view.term_id_by_value(&iri("head")).is_err()),
            1 => assert!(view.resolve(id).is_err()),
            _ => assert!(!view.has_named_graph(graph)),
        }
        assert_eq!(view.named_graphs().count(), 0);
        assert_eq!(view.quads().count(), 0);
        assert!(
            matches!(view.operation_status(), ViewOperationStatus::Failed { evidence, .. } if evidence.pages.requested_pages.is_empty())
        );
    }
}

#[test]
fn explicit_head_graph_lifetime_survives_sealing_while_populated_then_last_row_removal() {
    let graph = iri("declared");
    let q = purrdf_core::QuadValues {
        g: Some(graph.clone()),
        ..row("a", "b")
    };
    let mut stack = PagedStack::new(vec![sealed(vec![])]).unwrap();
    stack.declare_named_graph(graph.clone()).unwrap();
    stack.insert(q.clone()).unwrap();
    stack.seal_head(bound(1)).unwrap();
    stack.remove(&q).unwrap();
    stack.seal_head(bound(2)).unwrap();
    let snapshot = stack.snapshot().unwrap();
    let view = snapshot.query_view(PagedQueryLimits::UNBOUNDED);
    assert_eq!(view.quads().count(), 0);
    assert_eq!(
        view.named_graphs()
            .map(|id| view.term_value(id).unwrap())
            .collect::<Vec<_>>(),
        vec![graph.clone()]
    );
    let mut eager = RdfDatasetBuilder::new();
    let id = eager.intern_value(&graph);
    eager.declare_named_graph(id);
    assert_eq!(
        page_bytes(
            &snapshot
                .compact(PagedQueryLimits::UNBOUNDED, bound(1))
                .unwrap()
        ),
        page_bytes(&canonical_paged_seal(eager.freeze().unwrap().as_ref(), bound(1)).unwrap())
    );
}

#[test]
fn logical_id_filters_do_not_recheck_descriptors_per_row_or_cached_reread() {
    let mut measurements = Vec::new();
    for rows in [1, 32, 256] {
        let ordinary = page(
            &(0..rows)
                .map(|i| row(&format!("plain{i}"), "o"))
                .collect::<Vec<_>>(),
        );
        let mut builder = RdfDatasetBuilder::new();
        let triple = builder.intern_value(&purrdf_core::term_fixture::triple_chain(1));
        for i in 0..rows {
            let r = builder.intern_value(&iri(&format!("reifier{i}")));
            builder.push_reifier(r, triple);
        }
        let providers = [
            Arc::new(ProbeProvider::new(vec![ordinary], None)),
            Arc::new(ProbeProvider::new(vec![builder.freeze().unwrap()], None)),
        ];
        let snapshot = PagedStack::new(
            providers
                .iter()
                .map(|provider| Arc::new(PagedDataset::from_provider(provider.clone()).unwrap()))
                .collect(),
        )
        .unwrap()
        .snapshot()
        .unwrap();
        let view = snapshot.query_view(PagedQueryLimits::UNBOUNDED);
        for provider in &providers {
            provider.generation_reads.store(0, Ordering::Relaxed);
            provider.count_reads.store(0, Ordering::Relaxed);
            provider.reads.store(0, Ordering::Relaxed);
        }
        assert_eq!(view.quads().count(), rows);
        assert_eq!(view.reifier_quads().count(), rows);
        let admitted = providers
            .iter()
            .map(|provider| {
                (
                    provider.generation_reads.load(Ordering::Relaxed),
                    provider.count_reads.load(Ordering::Relaxed),
                    provider.reads.load(Ordering::Relaxed),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(view.quads().count(), rows);
        assert_eq!(view.reifier_quads().count(), rows);
        let reread = providers
            .iter()
            .map(|provider| {
                (
                    provider.generation_reads.load(Ordering::Relaxed),
                    provider.count_reads.load(Ordering::Relaxed),
                    provider.reads.load(Ordering::Relaxed),
                )
            })
            .collect::<Vec<_>>();
        eprintln!("ID drain rows={rows} sources=2 pages=2 admitted={admitted:?} reread={reread:?}");
        assert_eq!(
            reread, admitted,
            "cached ID filters must not query descriptors"
        );
        assert!(matches!(
            view.operation_status(),
            ViewOperationStatus::Ready { .. }
        ));
        measurements.push(admitted);
    }
    assert!(
        measurements.windows(2).all(|pair| pair[0] == pair[1]),
        "fixed-page/source admissions cannot scale with logical ID rows"
    );
}

#[test]
fn cached_id_rows_require_a_final_checkpoint_and_a_latched_fault_suppresses_open_iterators() {
    for change_count in [false, true] {
        let ordinary = page(&[row("a", "x"), row("b", "y")]);
        let mut builder = RdfDatasetBuilder::new();
        purrdf_core::ir::import::DatasetImporter::new(&mut builder, ordinary.as_ref()).append();
        let triple = builder.intern_value(&purrdf_core::term_fixture::triple_chain(1));
        let note = builder.intern_value(&iri("note"));
        let value = builder.intern_value(&iri("value"));
        for i in 0..2 {
            let r = builder.intern_value(&iri(&format!("reifier{i}")));
            builder.push_reifier(r, triple);
            builder.push_annotation(r, note, value);
        }
        let provider = Arc::new(ProbeProvider::new(vec![builder.freeze().unwrap()], None));
        let snapshot = PagedStack::new(vec![Arc::new(
            PagedDataset::from_provider(provider.clone()).unwrap(),
        )])
        .unwrap()
        .snapshot()
        .unwrap();
        let view = snapshot.query_view(PagedQueryLimits::UNBOUNDED);
        assert_eq!(view.quads().count(), 2);
        assert_eq!(view.reifier_quads().count(), 2);
        assert_eq!(view.annotation_quads().count(), 2);
        let mut open_rows = view.quads();
        let mut open_reifiers = view.reifier_quads();
        let mut open_annotations = view.annotation_quads();
        assert!(open_rows.next().is_some());
        assert!(open_reifiers.next().is_some());
        assert!(open_annotations.next().is_some());
        if change_count {
            provider.count.store(2, Ordering::Relaxed);
        } else {
            provider.generation.store(8, Ordering::Relaxed);
        }
        // ID rows from certified cache alone are not a completeness certificate.
        let status = view.operation_status();
        let ViewOperationStatus::Failed {
            error: PagedQueryError::SourceSnapshot { source, error },
            ..
        } = &status
        else {
            panic!("final checkpoint must refuse delayed descriptor drift: {status:?}");
        };
        assert_eq!(*source, 0);
        if change_count {
            assert!(matches!(
                **error,
                PagedQueryError::PageCountMismatch {
                    expected: 1,
                    actual: 2
                }
            ));
        } else {
            assert!(matches!(
                **error,
                PagedQueryError::StaleGeneration {
                    expected: PageGeneration(7),
                    actual: PageGeneration(8),
                    ..
                }
            ));
        }
        assert_eq!(
            open_rows.next(),
            None,
            "open ordinary iterator must observe the sticky latch"
        );
        assert_eq!(
            open_reifiers.next(),
            None,
            "buffered reifier must observe the sticky latch"
        );
        assert_eq!(
            open_annotations.next(),
            None,
            "buffered annotation must observe the sticky latch"
        );
        assert_eq!(view.reifier_quads().next(), None);
        assert_eq!(view.annotation_quads().next(), None);
        provider.count.store(1, Ordering::Relaxed);
        provider.generation.store(7, Ordering::Relaxed);
        assert_eq!(view.quads().next(), None);
        assert_eq!(
            view.operation_status(),
            status,
            "provider recovery cannot erase a fault"
        );
    }
}

#[test]
fn declaration_probe_fault_suppresses_the_candidate_that_triggered_it() {
    let ordinary = Arc::new(ProbeProvider::new(vec![page(&[row("r", "note")])], None));
    let mut builder = RdfDatasetBuilder::new();
    let r = builder.intern_value(&iri("r"));
    let triple = builder.intern_value(&purrdf_core::term_fixture::triple_chain(1));
    builder.push_reifier(r, triple);
    let declarations = Arc::new(ProbeProvider::new(vec![builder.freeze().unwrap()], None));
    let snapshot = PagedStack::new(
        vec![ordinary, declarations.clone()]
            .into_iter()
            .map(|provider| Arc::new(PagedDataset::from_provider(provider).unwrap()))
            .collect(),
    )
    .unwrap()
    .snapshot()
    .unwrap();
    declarations.mode.store(1, Ordering::Relaxed);
    let view = snapshot.query_view(PagedQueryLimits::UNBOUNDED);
    assert_eq!(
        view.quads().next(),
        None,
        "classification failure cannot yield its ordinary candidate"
    );
    assert_eq!(view.annotation_quads().next(), None);
    assert_eq!(view.reifier_quads().next(), None);
    assert!(matches!(
        view.operation_status(),
        ViewOperationStatus::Failed {
            error: PagedQueryError::Provider { .. },
            ..
        }
    ));
}
