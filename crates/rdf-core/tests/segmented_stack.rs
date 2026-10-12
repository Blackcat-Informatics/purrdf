// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Real header-read and allocation controls for shallow segmented publications.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use purrdf_alloc_probe::{CountingAllocator, CurrentThreadWindow};
use purrdf_core::TermFactory;
use purrdf_core::{
    RdfDatasetBuilder, SegmentedBuildLimits, SegmentedBuilder, SegmentedBytes, SegmentedError,
    SegmentedLayer, SegmentedLayerMetadata, SegmentedProvider, SegmentedReadLimits,
    SegmentedSnapshot, SegmentedStack, TermValue,
};

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

#[derive(Debug)]
struct Drift {
    bytes: SegmentedBytes,
    drift: Arc<AtomicBool>,
}
impl SegmentedProvider for Drift {
    fn snapshot(&self) -> SegmentedSnapshot {
        self.bytes.snapshot()
    }
    fn byte_len(&self) -> u64 {
        self.bytes.byte_len() + u64::from(self.drift.load(Ordering::Relaxed))
    }
    fn read_at(&self, position: u64, output: &mut [u8]) -> Result<(), SegmentedError> {
        self.bytes.read_at(position, output)
    }
}

#[derive(Debug)]
struct HeaderProbe {
    bytes: SegmentedBytes,
    headers: Arc<AtomicUsize>,
}

impl SegmentedProvider for HeaderProbe {
    fn snapshot(&self) -> SegmentedSnapshot {
        self.bytes.snapshot()
    }
    fn byte_len(&self) -> u64 {
        self.bytes.byte_len()
    }
    fn read_at(&self, position: u64, output: &mut [u8]) -> Result<(), SegmentedError> {
        if output.len() == 512 {
            assert_eq!(position, 0, "publication read a non-header block");
            self.headers.fetch_add(1, Ordering::Relaxed);
        } else {
            assert_eq!(
                output.len(),
                32,
                "only authentication proof reads are allowed"
            );
        }
        self.bytes.read_at(position, output)
    }
}

fn layer(index: usize, terms: usize, headers: &Arc<AtomicUsize>) -> Arc<SegmentedLayer> {
    let values: Vec<_> = (0..terms)
        .map(|term| TermValue::iri(format!("urn:source:{index}:term:{term}")))
        .collect();
    let mut builder =
        SegmentedBuilder::new(SegmentedBuildLimits::new(4096, 200_000, 4096, 512, 4).unwrap())
            .unwrap();
    builder.intern_batch(&values, |_, _| {}).unwrap();
    let image = builder.seal().unwrap();
    let mut removals = RdfDatasetBuilder::new();
    for value in &values {
        let id = removals.intern_value(value).unwrap();
        removals.push_quad(id, id, id, None).unwrap();
    }
    let metadata = SegmentedLayerMetadata::new(Some(removals.freeze().unwrap()), None).unwrap();
    Arc::new(
        SegmentedLayer::new(
            Arc::new(HeaderProbe {
                bytes: image.provider(),
                headers: Arc::clone(headers),
            }),
            image.receipt().clone(),
            metadata,
        )
        .unwrap(),
    )
}

fn limits() -> SegmentedReadLimits {
    SegmentedReadLimits::new(4_000_000, 2, 20_000, 10_000_000, 4)
}

fn statement(subject: &str, object: &str, graph: Option<&str>) -> purrdf_core::QuadValues {
    purrdf_core::QuadValues {
        s: TermValue::iri(subject),
        p: TermValue::iri("urn:p"),
        o: TermValue::iri(object),
        g: graph.map(TermValue::iri),
    }
}

fn removal_carrier(rows: &[purrdf_core::QuadValues]) -> Arc<purrdf_core::RdfDataset> {
    statement_carrier([rows, &[], &[]], &[])
}

fn statement_carrier(
    rows: [&[purrdf_core::QuadValues]; 3],
    declarations: &[TermValue],
) -> Arc<purrdf_core::RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    for (stream, row) in rows
        .into_iter()
        .enumerate()
        .flat_map(|(stream, rows)| rows.iter().map(move |row| (stream, row)))
    {
        let s = builder.intern_value(&row.s).unwrap();
        let p = builder.intern_value(&row.p).unwrap();
        let o = builder.intern_value(&row.o).unwrap();
        let g = row.g.as_ref().map(|g| builder.intern_value(g).unwrap());
        match stream {
            0 => builder.push_quad(s, p, o, g).unwrap(),
            1 => builder.push_reifier_in_graph(s, o, g).unwrap(),
            2 => builder.push_annotation_in_graph(s, p, o, g).unwrap(),
            _ => unreachable!(),
        }
    }
    for graph in declarations {
        let id = builder.intern_value(graph).unwrap();
        builder.declare_named_graph(id).unwrap();
    }
    builder.freeze().unwrap()
}

fn statement_layer(
    rows: [&[purrdf_core::QuadValues]; 3],
    metadata: SegmentedLayerMetadata,
) -> Arc<SegmentedLayer> {
    let image = statement_image(rows);
    Arc::new(
        SegmentedLayer::new(
            Arc::new(image.provider()),
            image.receipt().clone(),
            metadata,
        )
        .unwrap(),
    )
}

fn statement_image(rows: [&[purrdf_core::QuadValues]; 3]) -> purrdf_core::SegmentedImage {
    let mut builder =
        SegmentedBuilder::new(SegmentedBuildLimits::new(4096, 200_000, 4096, 512, 4).unwrap())
            .unwrap();
    for (stream, rows) in rows.into_iter().enumerate() {
        for row in rows {
            let mut ids = Vec::new();
            let mut values = vec![row.s.clone(), row.p.clone(), row.o.clone()];
            values.extend(row.g.clone());
            builder.intern_batch(&values, |_, id| ids.push(id)).unwrap();
            let row = purrdf_core::QuadIds {
                s: ids[0],
                p: ids[1],
                o: ids[2],
                g: ids.get(3).copied(),
            };
            match stream {
                0 => builder.push_quad(row).unwrap(),
                1 => builder.push_reifier(row.s, row.o, row.g).unwrap(),
                2 => builder.push_annotation(row).unwrap(),
                _ => unreachable!(),
            }
        }
    }
    builder.seal().unwrap()
}

#[test]
fn generated_chronological_publications_match_eager_typed_seals() {
    use purrdf_core::{
        BlankScope, InMemoryPageProvider, PackBuilder, PagedDataset, PagedQueryLimits, PagedStack,
        canonical_paged_seal,
    };
    use purrdf_testkit::rng::SplitMix64;
    use std::num::NonZeroUsize;
    let page_rows = NonZeroUsize::new(3).unwrap();
    for seed in 0..16 {
        let mut draws = SplitMix64::new(seed);
        let mut value = statement("urn:s", "urn:unused", Some("urn:g"));
        value.s = TermValue::Blank {
            label: "same".into(),
            scope: BlankScope(7),
        };
        value.o = TermValue::Triple {
            s: TermValue::Blank {
                label: "same".into(),
                scope: BlankScope(8),
            }
            .into(),
            p: TermValue::iri("urn:p").into(),
            o: TermValue::typed_literal(
                format!("[_:{}]", BlankScope(9).qualify_label("embedded")),
                purrdf_cdt::CDT_LIST,
            )
            .into(),
        };
        let annotation = statement("urn:r", "urn:value", Some("urn:g"));
        let mut declaration = statement("urn:r", "urn:unused", Some("urn:g"));
        declaration.p = TermValue::iri(purrdf_iri::vocab::rdf::REIFIES);
        declaration.o = value.o.clone();
        let universe = [&value, &annotation, &declaration];
        let empty = Arc::new(
            PagedDataset::from_provider(Arc::new(InMemoryPageProvider::new(Vec::new()))).unwrap(),
        );
        let mut paged = PagedStack::new(vec![empty]).unwrap();
        let mut stack = SegmentedStack::default();
        let mut layers = Vec::new();
        for generation in 0..8 {
            let choice = draws.next_u64();
            let ordinary: Vec<_> = [&value, &annotation]
                .into_iter()
                .enumerate()
                .filter(|(index, _)| choice & (1 << index) != 0)
                .map(|(_, row)| row.clone())
                .collect();
            let reifiers: Vec<_> = (choice & 4 != 0)
                .then(|| declaration.clone())
                .into_iter()
                .collect();
            let annotations: Vec<_> = (choice & 8 != 0)
                .then(|| annotation.clone())
                .into_iter()
                .collect();
            let removed: Vec<_> = universe
                .into_iter()
                .enumerate()
                .filter(|(index, _)| choice & (16 << index) != 0)
                .map(|(_, row)| row.clone())
                .collect();
            let declared: Vec<_> = (choice & 128 != 0)
                .then(|| TermValue::iri(format!("urn:explicit:{}", generation % 2)))
                .into_iter()
                .collect();
            let rows = [
                ordinary.as_slice(),
                reifiers.as_slice(),
                annotations.as_slice(),
            ];
            let resident = statement_carrier(rows, &declared);
            let source = Arc::new(
                PagedDataset::from_provider(Arc::new(InMemoryPageProvider::new(vec![resident])))
                    .unwrap(),
            );
            paged.append(source, removed.clone()).unwrap();
            let metadata = SegmentedLayerMetadata::new(
                Some(removal_carrier(&removed)),
                Some(statement_carrier([&[], &[], &[]], &declared)),
            )
            .unwrap();
            layers.push(statement_layer(rows, metadata));
            stack.publish(layers.clone(), limits()).unwrap();
            let snapshot = stack.snapshot().unwrap();
            let query = snapshot.query(limits()).unwrap();
            let sparse_fold = canonical_paged_seal(&query, page_rows).unwrap();
            query.check_ready().unwrap();
            let eager_fold = paged
                .snapshot()
                .unwrap()
                .compact(PagedQueryLimits::UNBOUNDED, page_rows)
                .unwrap();
            let sparse_bytes =
                PackBuilder::build_view_bytes(&sparse_fold.query_view(PagedQueryLimits::UNBOUNDED))
                    .unwrap();
            let eager_bytes =
                PackBuilder::build_view_bytes(&eager_fold.query_view(PagedQueryLimits::UNBOUNDED))
                    .unwrap();
            assert_eq!(
                sparse_bytes, eager_bytes,
                "seed={seed}, generation={generation}"
            );
        }
    }
}

#[test]
fn empty_certified_streams_do_not_probe_dictionary_for_bound_head_row() {
    use purrdf_core::{
        DatasetView, FallibleDatasetView, GraphMatch, SegmentedResidentHead, ViewOperationStatus,
    };
    let headers = Arc::new(AtomicUsize::new(0));
    // This provider rejects every non-header read, despite its nonempty
    // dictionary and historical removal carrier.
    let sealed = layer(0, 10, &headers);
    let mut stack = SegmentedStack::default();
    stack.publish(vec![sealed], limits()).unwrap();
    let row = statement("urn:head-only", "urn:object", None);
    stack.set_head(Some(
        SegmentedResidentHead::new(
            removal_carrier(std::slice::from_ref(&row)),
            SegmentedLayerMetadata::default(),
        )
        .unwrap(),
    ));
    let snapshot = stack.snapshot().unwrap();
    let query = snapshot.query(limits()).unwrap();
    let ViewOperationStatus::Ready { evidence: before } = query.operation_status() else {
        panic!("header-only open must be ready");
    };
    let subject = query
        .intern_source_term(purrdf_core::SegmentedStackSource::Head, 0)
        .unwrap();
    assert_eq!(
        query
            .quads_for_pattern(Some(subject), None, None, GraphMatch::Any)
            .count(),
        1
    );
    assert_eq!(query.annotation_quads().count(), 0);
    assert_eq!(query.reifier_quads().count(), 0);
    let ViewOperationStatus::Ready { evidence: after } = query.operation_status() else {
        panic!("empty sealed tables need no dictionary probes");
    };
    assert_eq!(after.request_count(), before.request_count());
    assert_eq!(after.io_bytes(), before.io_bytes());
    assert_eq!(headers.load(Ordering::Relaxed), 2);
}

#[test]
fn persisted_purge_rewrites_only_affected_source_and_releases_old_pins_last() {
    use purrdf_core::{DatasetView, FallibleDatasetView, ViewOperationStatus};
    let secret = "purged-private-lexical-marker-495";
    let datatype = "urn:purged-private-datatype-495";
    let mut private = statement("urn:private", "urn:unused", None);
    private.o = TermValue::typed_literal(secret, datatype);
    let kept = statement("urn:kept", "urn:public", None);
    let untouched = statement_image([&[statement("urn:untouched", "urn:other", None)], &[], &[]]);
    let original = statement_image([&[private.clone(), kept.clone()], &[], &[]]);
    for removed in [secret, datatype] {
        assert!(
            original
                .bytes()
                .windows(removed.len())
                .any(|b| b == removed.as_bytes())
        );
    }
    let original_provider = Arc::new(original.provider());
    let original_weak = Arc::downgrade(&original_provider);
    let original_layer = Arc::new(
        SegmentedLayer::new(
            original_provider.clone(),
            original.receipt().clone(),
            SegmentedLayerMetadata::default(),
        )
        .unwrap(),
    );
    let original_layer_weak = Arc::downgrade(&original_layer);
    let untouched_provider = Arc::new(untouched.provider());
    let untouched_layer = Arc::new(
        SegmentedLayer::new(
            untouched_provider.clone(),
            untouched.receipt().clone(),
            SegmentedLayerMetadata::default(),
        )
        .unwrap(),
    );
    let mut stack = SegmentedStack::default();
    stack
        .publish(vec![original_layer, Arc::clone(&untouched_layer)], limits())
        .unwrap();
    drop(original_provider);
    drop(original);
    let old = stack.snapshot().unwrap();
    let old_query = old.query(limits()).unwrap();
    let private_id = old_query.term_id_by_value(&private.o).unwrap().unwrap();
    let ViewOperationStatus::Ready { evidence } = old_query.operation_status() else {
        panic!("original publication must be ready");
    };

    // Encode a real successor from retained values. No dictionary or term ID
    // from the original segment is carried into the replacement image.
    let successor = statement_image([std::slice::from_ref(&kept), &[], &[]]);
    let drift = Arc::new(AtomicBool::new(false));
    let successor_provider = Arc::new(Drift {
        bytes: successor.provider(),
        drift: Arc::clone(&drift),
    });
    let successor_layer = Arc::new(
        SegmentedLayer::new(
            successor_provider.clone(),
            successor.receipt().clone(),
            SegmentedLayerMetadata::default(),
        )
        .unwrap(),
    );
    let replacement = || vec![Arc::clone(&successor_layer), Arc::clone(&untouched_layer)];
    assert_eq!(
        stack.publish(
            replacement(),
            SegmentedReadLimits::new(4_000_000, 2, 0, 10_000_000, 4)
        ),
        Err(SegmentedError::EvidenceExhausted),
    );
    assert_eq!(stack.snapshot().unwrap(), old);
    drift.store(true, Ordering::Relaxed);
    assert_eq!(
        stack.publish(replacement(), limits()),
        Err(SegmentedError::SnapshotMismatch)
    );
    assert_eq!(stack.snapshot().unwrap(), old);
    drift.store(false, Ordering::Relaxed);
    stack.publish(replacement(), limits()).unwrap();
    let current = stack.snapshot().unwrap();
    assert!(Arc::ptr_eq(&current.layers()[1], &untouched_layer));
    assert_eq!(current.layers()[1].receipt(), untouched.receipt());
    let query = current.query(limits()).unwrap();
    assert_eq!(query.quads().count(), 2);
    assert_eq!(query.term_id_by_value(&private.o).unwrap(), None);
    assert_eq!(
        query.term_id_by_value(&TermValue::iri(datatype)).unwrap(),
        None
    );
    query.check_ready().unwrap();
    assert_eq!(
        old_query.term_value(private_id).unwrap().value(),
        &private.o
    );
    old_query.check_ready().unwrap();
    drop(old_query);
    drop(old);
    assert!(original_weak.upgrade().is_some());
    assert!(original_layer_weak.upgrade().is_some());
    assert_eq!(evidence.identity().layers().len(), 2);
    drop(evidence);
    assert!(original_weak.upgrade().is_none());
    assert!(original_layer_weak.upgrade().is_none());

    // Read the consumer-owned persisted providers after the last original pin
    // is gone: membership-only deletion would leave this lexical scan failing.
    for provider in [
        successor_provider.as_ref() as &dyn SegmentedProvider,
        untouched_provider.as_ref(),
    ] {
        let mut bytes = vec![0; usize::try_from(provider.byte_len()).unwrap()];
        provider.read_at(0, &mut bytes).unwrap();
        for removed in [secret, datatype] {
            assert!(
                !bytes
                    .windows(removed.len())
                    .any(|b| b == removed.as_bytes())
            );
        }
        if provider.snapshot() == untouched.receipt().snapshot() {
            assert_eq!(bytes.as_slice(), untouched.bytes());
        }
    }
}

#[test]
fn chronological_view_removes_reinserts_and_preserves_explicit_empty_graphs() {
    use purrdf_core::{DatasetView, GraphMatch};
    let a = statement("urn:s", "urn:a", Some("urn:implicit"));
    let b = statement("urn:s", "urn:b", None);
    let first = statement_layer(
        [&[a.clone(), b.clone()], &[], &[]],
        SegmentedLayerMetadata::default(),
    );
    let mut declarations = RdfDatasetBuilder::new();
    let graph = declarations.intern_iri("urn:explicit").unwrap();
    declarations.declare_named_graph(graph).unwrap();
    let erased = statement_layer(
        [&[], &[], &[]],
        SegmentedLayerMetadata::new(
            Some(removal_carrier(&[a.clone(), b.clone()])),
            Some(declarations.freeze().unwrap()),
        )
        .unwrap(),
    );
    let mut stack = SegmentedStack::default();
    stack
        .publish(vec![Arc::clone(&first), Arc::clone(&erased)], limits())
        .unwrap();
    let snapshot = stack.snapshot().unwrap();
    let query = snapshot.query(limits()).unwrap();
    assert_eq!(query.quads().count(), 0);
    let graphs: Vec<_> = query
        .named_graphs()
        .map(|id| query.term_value(id).unwrap().value().clone())
        .collect();
    assert_eq!(graphs, [TermValue::iri("urn:explicit")]);
    query.check_ready().unwrap();

    let latest = statement_layer(
        [std::slice::from_ref(&b), &[], &[]],
        SegmentedLayerMetadata::default(),
    );
    stack
        .publish(vec![first, erased, latest], limits())
        .unwrap();
    let next = stack.snapshot().unwrap();
    let query = next.query(limits()).unwrap();
    let subject = query.term_id_by_value(&b.s).unwrap().unwrap();
    let rows: Vec<_> = query
        .quads_for_pattern(Some(subject), None, None, GraphMatch::Any)
        .collect();
    assert_eq!(rows.len(), 1);
    assert_eq!(query.term_value(rows[0].o).unwrap().value(), &b.o);
    let missing = query.intern_source_term(purrdf_core::SegmentedStackSource::Layer(0), u64::MAX);
    assert!(missing.is_err());
    assert!(query.read_error().is_some());
    assert_eq!(
        query.quads().count(),
        0,
        "failure never becomes an unbound scan"
    );
}

#[test]
fn effective_annotation_classification_uses_original_chronology_and_first_occurrence() {
    use purrdf_core::DatasetView;
    let annotation = statement("urn:r", "urn:value", Some("urn:g"));
    let mut declaration = statement("urn:r", "urn:unused", Some("urn:g"));
    declaration.p = TermValue::iri(purrdf_iri::vocab::rdf::REIFIES);
    declaration.o = TermValue::Triple {
        s: Box::new(TermValue::iri("urn:s")).into(),
        p: Box::new(TermValue::iri("urn:p")).into(),
        o: Box::new(TermValue::iri("urn:o")).into(),
    };
    let initial = statement_layer(
        [
            &[],
            std::slice::from_ref(&declaration),
            std::slice::from_ref(&annotation),
        ],
        SegmentedLayerMetadata::default(),
    );
    let removal = statement_layer(
        [&[], &[], &[]],
        SegmentedLayerMetadata::new(
            Some(removal_carrier(std::slice::from_ref(&declaration))),
            None,
        )
        .unwrap(),
    );
    let orphan = statement_layer(
        [&[], &[], std::slice::from_ref(&annotation)],
        SegmentedLayerMetadata::default(),
    );
    let ordinary = statement_layer(
        [std::slice::from_ref(&annotation), &[], &[]],
        SegmentedLayerMetadata::default(),
    );
    let mut stack = SegmentedStack::default();
    for (layers, expected) in [
        (vec![Arc::clone(&initial)], (0, 1, 1)),
        (vec![Arc::clone(&initial), Arc::clone(&removal)], (1, 0, 0)),
        (
            vec![Arc::clone(&initial), Arc::clone(&removal), orphan],
            (0, 0, 1),
        ),
        (vec![Arc::clone(&initial), ordinary], (0, 1, 1)),
    ] {
        stack.publish(layers, limits()).unwrap();
        let snapshot = stack.snapshot().unwrap();
        let query = snapshot.query(limits()).unwrap();
        assert_eq!(
            (
                query.quads().count(),
                query.reifier_quads().count(),
                query.annotation_quads().count()
            ),
            expected,
            "original operational cause: {:?}",
            query.read_error()
        );
        query.check_ready().unwrap();
    }
}

#[test]
fn removal_only_minted_nested_scoped_value_roundtrips_through_dataset_view() {
    use purrdf_core::{BlankScope, DatasetView, SegmentedStackSource, TermGuard};
    let value = TermValue::Triple {
        s: TermValue::Blank {
            label: "only-removed".into(),
            scope: BlankScope(43),
        }
        .into(),
        p: TermValue::iri("urn:p").into(),
        o: TermValue::typed_literal("nested", purrdf_xsd::datatype::XSD_STRING).into(),
    };
    let mut removals = RdfDatasetBuilder::new();
    let local = removals.intern_value(&value).unwrap();
    let metadata = SegmentedLayerMetadata::new(Some(removals.freeze().unwrap()), None).unwrap();
    let mut stack = SegmentedStack::default();
    stack
        .publish(vec![statement_layer([&[], &[], &[]], metadata)], limits())
        .unwrap();
    let snapshot = stack.snapshot().unwrap();
    let query = snapshot.query(limits()).unwrap();
    let id = query
        .intern_source_term(SegmentedStackSource::Removals(0), local.index() as u64)
        .unwrap();
    assert_eq!(query.term_id_by_value(&value).unwrap(), Some(id));
    assert_eq!(DatasetView::term_value(&query, id).unwrap(), value);
    let guard = query.resolve(id).unwrap();
    let purrdf_core::TermRef::Triple { s, .. } = guard.term() else {
        panic!("expected quoted term");
    };
    assert_eq!(
        query
            .term_id_by_value(&TermValue::Blank {
                label: "only-removed".into(),
                scope: BlankScope(43)
            })
            .unwrap(),
        Some(s)
    );
    assert!(
        query
            .term_id_by_value(&TermValue::Blank {
                label: "only-removed".into(),
                scope: BlankScope(44)
            })
            .unwrap()
            .is_none()
    );
    query.check_ready().unwrap();
}

#[test]
fn fallible_evidence_pins_complete_publication_and_immutable_request_prefix() {
    use purrdf_core::{
        DatasetView, FallibleDatasetView, SegmentedResidentHead, ViewOperationStatus,
    };
    let row = statement("urn:s", "urn:o", None);
    let sealed = statement_layer(
        [std::slice::from_ref(&row), &[], &[]],
        SegmentedLayerMetadata::default(),
    );
    let sealed_weak = Arc::downgrade(&sealed);
    let mut removed = RdfDatasetBuilder::new();
    removed
        .intern_value(&TermValue::simple_literal("r".repeat(50_000)))
        .unwrap();
    let removed = removed.freeze().unwrap();
    let removed_weak = Arc::downgrade(&removed);
    let mut declared = RdfDatasetBuilder::new();
    let graph = declared
        .intern_iri(&format!("urn:{}", "g".repeat(80_000)))
        .unwrap();
    declared.declare_named_graph(graph).unwrap();
    let declared = declared.freeze().unwrap();
    let declared_weak = Arc::downgrade(&declared);
    let head = removal_carrier(std::slice::from_ref(&row));
    let head_weak = Arc::downgrade(&head);
    let mut stack = SegmentedStack::default();
    stack.publish(vec![sealed], limits()).unwrap();
    stack.set_head(Some(
        SegmentedResidentHead::new(
            head,
            SegmentedLayerMetadata::new(Some(removed), Some(declared)).unwrap(),
        )
        .unwrap(),
    ));
    let snapshot = stack.snapshot().unwrap();
    let query = snapshot.query(limits()).unwrap();
    assert_eq!(query.touched_terms(), 0);
    assert!(
        query.max_owned_term_bytes().unwrap() >= 80_004,
        "certificate includes untouched metadata"
    );
    let ViewOperationStatus::Ready { evidence } = query.operation_status() else {
        panic!("ready publication");
    };
    let prefix = evidence.request_count();
    let digest = *evidence.request_digest();
    let window = CurrentThreadWindow::open();
    let retained = evidence.clone();
    let measured = window.close();
    assert_eq!(
        measured.allocations, 0,
        "evidence clone only pins existing owners"
    );
    assert_eq!(query.quads().count(), 1);
    let ViewOperationStatus::Ready { evidence: after } = query.operation_status() else {
        panic!("complete query");
    };
    assert!(after.request_count() > prefix);
    assert_eq!(evidence.request_count(), prefix);
    assert_eq!(*evidence.request_digest(), digest);
    evidence.with_requests(|requests| assert_eq!(requests.len(), prefix));
    drop(after);
    drop(query);
    drop(snapshot);
    drop(stack);
    drop(evidence);
    assert!(sealed_weak.upgrade().is_some());
    assert!(head_weak.upgrade().is_some());
    assert!(removed_weak.upgrade().is_some());
    assert!(declared_weak.upgrade().is_some());
    assert_eq!(retained.identity().layers().len(), 1);
    assert!(retained.identity().head().is_some());
    drop(retained);
    assert!(sealed_weak.upgrade().is_none());
    assert!(head_weak.upgrade().is_none());
    assert!(removed_weak.upgrade().is_none());
    assert!(declared_weak.upgrade().is_none());
}

#[test]
fn thousand_distinct_sources_open_only_headers_under_one_request_budget() {
    let headers = Arc::new(AtomicUsize::new(0));
    let layers = (0..1000).map(|index| layer(index, 1, &headers)).collect();
    let mut stack = SegmentedStack::default();
    stack.publish(layers, limits()).unwrap();
    assert_eq!(headers.load(Ordering::Relaxed), 1000);
    let snapshot = stack.snapshot().unwrap();
    assert_eq!(
        headers.load(Ordering::Relaxed),
        1000,
        "snapshot performs no block reads"
    );
    assert_eq!(snapshot.layers().len(), 1000);
    snapshot.validate(limits()).unwrap();
    assert_eq!(headers.load(Ordering::Relaxed), 2000);
    let query = snapshot.query(limits()).unwrap();
    assert_eq!(query.touched_terms(), 0);
    assert_eq!(headers.load(Ordering::Relaxed), 3000);
    query.check_ready().unwrap();
    drop(query);
    assert!(matches!(
        snapshot.validate(SegmentedReadLimits::new(4_000_000, 2, 20, 10_000_000, 4)),
        Err(SegmentedError::EvidenceExhausted)
    ));
}

#[test]
fn tenfold_historical_dictionary_and_removals_do_not_grow_snapshot_allocation() {
    let mut measurements = Vec::new();
    for terms in [10, 100] {
        let headers = Arc::new(AtomicUsize::new(0));
        let layers = (0..4).map(|index| layer(index, terms, &headers)).collect();
        let mut stack = SegmentedStack::default();
        stack.publish(layers, limits()).unwrap();
        let original = stack.snapshot().unwrap();
        let reads = headers.load(Ordering::Relaxed);
        let window = CurrentThreadWindow::open();
        let snapshot = stack.snapshot().unwrap();
        let measured = window.close();
        assert_eq!(headers.load(Ordering::Relaxed), reads);
        for (old, new) in original.layers().iter().zip(snapshot.layers()) {
            assert!(Arc::ptr_eq(old, new));
            assert!(Arc::ptr_eq(
                old.metadata().removals().unwrap(),
                new.metadata().removals().unwrap()
            ));
        }
        measurements.push((
            measured.allocations,
            measured.requested_bytes,
            measured.retained_bytes,
            measured.peak_working_bytes,
        ));
    }
    assert_eq!(measurements[0], measurements[1]);
}

fn value_layer(values: &[TermValue]) -> (Arc<SegmentedLayer>, Vec<u64>) {
    let mut builder =
        SegmentedBuilder::new(SegmentedBuildLimits::new(4096, 200_000, 4096, 512, 4).unwrap())
            .unwrap();
    let mut ids = Vec::new();
    builder
        .intern_batch(values, |_, id| ids.push(id.index()))
        .unwrap();
    let image = builder.seal().unwrap();
    (
        Arc::new(
            SegmentedLayer::new(
                Arc::new(image.provider()),
                image.receipt().clone(),
                SegmentedLayerMetadata::default(),
            )
            .unwrap(),
        ),
        ids,
    )
}

#[test]
fn query_ids_touch_only_values_and_preserve_source_scopes() {
    use purrdf_core::{BlankScope, SegmentedStackSource, TermBox};
    let blank = TermValue::Blank {
        label: "same".into(),
        scope: BlankScope(7),
    };
    let literal = TermValue::simple_literal("exact lexical payload");
    let triple = TermValue::Triple {
        s: TermBox::new(blank.clone()),
        p: TermBox::new(TermValue::iri("urn:p")),
        o: TermBox::new(literal.clone()),
    };
    let values = [TermValue::iri("urn:first"), blank, literal, triple];
    let (first, a) = value_layer(&values);
    let mut others = values.to_vec();
    others[0] = TermValue::iri("urn:other");
    let (second, b) = value_layer(&others);
    let mut head = RdfDatasetBuilder::new();
    let head_id = head.intern_value(&values[3]).unwrap();
    let distinct = head
        .intern_value(&TermValue::Blank {
            label: "same".into(),
            scope: BlankScope(8),
        })
        .unwrap();
    let mut stack = SegmentedStack::default();
    stack.publish(vec![first, second], limits()).unwrap();
    stack.set_head(Some(
        purrdf_core::SegmentedResidentHead::new(
            head.freeze().unwrap(),
            SegmentedLayerMetadata::default(),
        )
        .unwrap(),
    ));
    let snapshot = stack.snapshot().unwrap();
    let window = CurrentThreadWindow::open();
    let query = snapshot.query(limits()).unwrap();
    assert_eq!(query.touched_terms(), 0);
    let first = query
        .intern_source_term(SegmentedStackSource::Layer(0), a[0])
        .unwrap();
    let second = query
        .intern_source_term(SegmentedStackSource::Layer(1), b[0])
        .unwrap();
    assert_ne!(first, second, "equal local IDs do not imply equal values");
    let mut shared = Vec::new();
    for index in 1..values.len() {
        let left = query
            .intern_source_term(SegmentedStackSource::Layer(0), a[index])
            .unwrap();
        assert_eq!(
            left,
            query
                .intern_source_term(SegmentedStackSource::Layer(1), b[index])
                .unwrap()
        );
        shared.push(left);
    }
    assert_eq!(
        shared[2],
        query
            .intern_source_term(
                SegmentedStackSource::Head,
                u64::try_from(head_id.index()).unwrap()
            )
            .unwrap()
    );
    assert_ne!(
        shared[0],
        query
            .intern_source_term(
                SegmentedStackSource::Head,
                u64::try_from(distinct.index()).unwrap()
            )
            .unwrap()
    );
    assert_eq!(query.touched_terms(), 6);
    let retained = query.term_value(shared[2]).unwrap();
    assert_eq!(retained.value(), &values[3]);
    query.check_ready().unwrap();
    drop(shared);
    drop(query);
    assert_eq!(
        retained.value(),
        &values[3],
        "owned result survives its query"
    );
    drop(retained);
    let measured = window.close();
    assert_eq!(measured.retained_bytes, 0);
    assert!(measured.peak_working_bytes <= i64::try_from(limits().live_bytes()).unwrap());
}

#[test]
fn memoized_term_hits_allocate_nothing_and_failed_import_latches() {
    use purrdf_core::SegmentedStackSource;
    let (layer, ids) = value_layer(&[TermValue::iri("urn:known")]);
    let mut stack = SegmentedStack::default();
    stack.publish(vec![layer], limits()).unwrap();
    let snapshot = stack.snapshot().unwrap();
    let query = snapshot.query(limits()).unwrap();
    let id = query
        .intern_source_term(SegmentedStackSource::Layer(0), ids[0])
        .unwrap();
    let window = CurrentThreadWindow::open();
    for _ in 0..100 {
        assert_eq!(
            query
                .intern_source_term(SegmentedStackSource::Layer(0), ids[0])
                .unwrap(),
            id
        );
    }
    let measured = window.close();
    assert_eq!(measured.allocations, 0);
    let original = query
        .intern_source_term(SegmentedStackSource::Layer(0), u64::MAX)
        .unwrap_err();
    assert_eq!(original, SegmentedError::AddressExhausted);
    assert_eq!(query.check_ready(), Err(original.clone()));
    assert_eq!(
        query.intern_source_term(SegmentedStackSource::Layer(0), ids[0]),
        Err(original)
    );
}

#[test]
fn admitted_identity_walk_releases_spill_and_preserves_hash_law() {
    use purrdf_core::TermBox;
    use purrdf_lex::allocation::{Admission, Memory, StorageError};
    use std::hash::{Hash as _, Hasher as _};
    struct Grant {
        limit: usize,
        live: usize,
        peak: usize,
    }
    impl Admission for Grant {
        fn resize(&mut self, live: usize) -> Result<(), StorageError> {
            if live > self.limit {
                return Err(StorageError::AdmissionFailed);
            }
            self.live = live;
            self.peak = self.peak.max(live);
            Ok(())
        }
    }
    let mut value = TermValue::blank("leaf");
    for _ in 0..40 {
        value = TermValue::Triple {
            s: TermBox::new(value),
            p: TermBox::new(TermValue::iri("urn:p")),
            o: TermBox::new(TermValue::simple_literal("object")),
        };
    }
    let copy = value.clone();
    let mut expected = purrdf_hash::fixed::FixedHasher::default();
    value.hash(&mut expected);
    let mut grant = Grant {
        limit: 100_000,
        live: 0,
        peak: 0,
    };
    let mut hash = purrdf_hash::fixed::FixedHasher::default();
    let mut memory = Memory::new(&mut grant);
    value.try_hash_with_memory(&mut hash, &mut memory).unwrap();
    assert_eq!(hash.finish(), expected.finish());
    assert_eq!(memory.admitted_bytes(), 0);
    assert!(
        value
            .try_cmp_with_memory(&copy, &mut memory)
            .unwrap()
            .is_eq()
    );
    assert_eq!(memory.admitted_bytes(), 0);
    assert!(grant.peak > 0);
    assert_eq!(grant.live, 0);
    grant.limit = 0;
    let mut memory = Memory::new(&mut grant);
    assert_eq!(
        value.try_hash_with_memory(&mut hash, &mut memory),
        Err(StorageError::AdmissionFailed)
    );
    assert_eq!(memory.admitted_bytes(), 0);
    assert_eq!(
        value.try_cmp_with_memory(&copy, &mut memory),
        Err(StorageError::AdmissionFailed)
    );
    assert_eq!(memory.admitted_bytes(), 0);
}

#[test]
fn final_query_readiness_checks_untouched_evicted_descriptors() {
    use purrdf_core::SegmentedStackSource;
    let mut builder =
        SegmentedBuilder::new(SegmentedBuildLimits::new(32, 4096, 32, 512, 4).unwrap()).unwrap();
    builder
        .intern_batch(&[TermValue::iri("urn:untouched")], |_, _| {})
        .unwrap();
    let image = builder.seal().unwrap();
    let drift = Arc::new(AtomicBool::new(false));
    let first = Arc::new(
        SegmentedLayer::new(
            Arc::new(Drift {
                bytes: image.provider(),
                drift: Arc::clone(&drift),
            }),
            image.receipt().clone(),
            SegmentedLayerMetadata::default(),
        )
        .unwrap(),
    );
    let (second, ids) = value_layer(&[TermValue::iri("urn:read")]);
    let mut stack = SegmentedStack::default();
    stack.publish(vec![first, second], limits()).unwrap();
    let snapshot = stack.snapshot().unwrap();
    let query = snapshot
        .query(SegmentedReadLimits::new(4_000_000, 1, 2000, 10_000_000, 4))
        .unwrap();
    query
        .intern_source_term(SegmentedStackSource::Layer(1), ids[0])
        .unwrap();
    drift.store(true, Ordering::Relaxed);
    assert_eq!(query.check_ready(), Err(SegmentedError::SnapshotMismatch));
    drift.store(false, Ordering::Relaxed);
    assert_eq!(
        query.check_ready(),
        Err(SegmentedError::SnapshotMismatch),
        "first fault survives restored descriptor"
    );
}

#[test]
fn resident_head_copy_refuses_under_the_shared_operation_grant() {
    use purrdf_core::{SegmentedResidentHead, SegmentedStackSource};
    let mut head = RdfDatasetBuilder::new();
    let id = head
        .intern_value(&TermValue::simple_literal("x".repeat(80_000)))
        .unwrap();
    let mut stack = SegmentedStack::default();
    stack.set_head(Some(
        SegmentedResidentHead::new(head.freeze().unwrap(), SegmentedLayerMetadata::default())
            .unwrap(),
    ));
    let snapshot = stack.snapshot().unwrap();
    // Admit the fixed operation owners while keeping the 80,000-byte touched
    // payload unaffordable; both refusals remain governed by the same ceiling.
    let budget = 32_000;
    let window = CurrentThreadWindow::open();
    let query = snapshot
        .query(SegmentedReadLimits::new(budget, 1, 16, 100_000, 2))
        .unwrap();
    let cause = query
        .intern_source_term(
            SegmentedStackSource::Head,
            u64::try_from(id.index()).unwrap(),
        )
        .unwrap_err();
    assert!(
        matches!(cause, SegmentedError::Residency { requested, limit } if requested > limit && limit == budget)
    );
    assert_eq!(query.touched_terms(), 0);
    assert_eq!(query.check_ready(), Err(cause));
    drop(query);
    let measured = window.close();
    assert_eq!(measured.retained_bytes, 0);
    assert!(measured.peak_working_bytes <= i64::try_from(budget).unwrap());
    let healthy = snapshot.query(limits()).unwrap();
    let id = healthy
        .intern_source_term(
            SegmentedStackSource::Head,
            u64::try_from(id.index()).unwrap(),
        )
        .unwrap();
    assert_eq!(
        healthy.term_value(id).unwrap().value(),
        &TermValue::simple_literal("x".repeat(80_000))
    );
}
