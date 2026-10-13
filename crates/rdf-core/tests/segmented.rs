// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Real block-provider, eviction, pin, exact-evidence and reopening laws.

use std::sync::Arc;

#[global_allocator]
static GLOBAL: purrdf_alloc_probe::CountingAllocator = purrdf_alloc_probe::CountingAllocator;

use purrdf_core::{
    BlankScope, DatasetView, GlobalTermId, GraphMatch, QuadIds, RdfTextDirection,
    SegmentedBuildLimits, SegmentedBuilder, SegmentedBytes, SegmentedError, SegmentedImage,
    SegmentedReadLimits, SegmentedSession, TermBox, TermGuard, TermRef, TermValue, ViewTermId,
    WorkspaceReservation,
};

fn build(values: &[TermValue], first: u64) -> (SegmentedImage, Vec<GlobalTermId>) {
    let limits = SegmentedBuildLimits::new(4096, 200_000, 4096, 1024, 4)
        .unwrap()
        .with_first_term_index(first)
        .unwrap();
    let mut builder = SegmentedBuilder::new(limits);
    let mut ids = Vec::with_capacity(values.len());
    builder.intern_batch(values, |_, id| ids.push(id)).unwrap();
    for &id in &ids {
        builder
            .push_quad(QuadIds {
                s: ids[0],
                p: ids[0],
                o: id,
                g: None,
            })
            .unwrap();
    }
    (builder.seal().unwrap(), ids)
}
fn session(image: &SegmentedImage, cache: u32) -> SegmentedSession {
    SegmentedSession::open(
        Arc::new(image.provider()),
        image.receipt(),
        SegmentedReadLimits::new(2_000_000, cache, 20_000, 10_000_000, 8),
    )
    .unwrap()
}

fn indexed_reopen_preserves_high_ids_and_bidirectional_batches() {
    let values: Vec<_> = (0..37)
        .map(|n| TermValue::iri(format!("http://example.org/中文/long/{n:04}")))
        .collect();
    let (image, ids) = build(&values, (1_u64 << 54) + 7);
    let read = session(&image, 2);
    assert_eq!(
        read.max_owned_term_bytes(),
        Some(144 + u64::try_from(values[0].as_iri().unwrap().len()).unwrap()),
        "the certified wire-profile cost must be identical on 32-bit and 64-bit targets"
    );
    assert_eq!(
        SegmentedImage::certify(image.bytes().to_vec())
            .unwrap()
            .bytes(),
        image.bytes(),
        "complete certification retains the portable representation verbatim"
    );
    let opened = read.evidence();
    opened.with_requests(|requests| {
        assert_eq!(requests[0].position(), 0);
        assert_eq!(requests[0].bytes(), 1024);
        assert_eq!(
            requests
                .iter()
                .filter(|request| request.bytes() == 1024)
                .count(),
            1,
            "opening reads the authenticated header, not dictionary/index blocks"
        );
    });
    let mut resolved = 0;
    read.resolve_batch(&ids, |id, term| {
        assert!(id.index() > 1_u64 << 53);
        assert_eq!(id.encode(), u128::from(id.index()));
        assert_eq!(term, TermRef::Iri(values[resolved].as_iri().unwrap()));
        resolved += 1;
    })
    .unwrap();
    let mut looked_up = 0;
    read.lookup_batch(&values, |value, id| {
        assert_eq!(value, &values[looked_up]);
        assert_eq!(id, Some(ids[looked_up]));
        looked_up += 1;
    })
    .unwrap();
    assert_eq!(resolved, values.len());
    assert_eq!(looked_up, values.len());
    assert_eq!(
        read.term_id_by_value(&TermValue::iri("http://example.org/absent"))
            .unwrap(),
        None
    );
    let evidence = read.evidence();
    assert!(evidence.evictions() > 0);
    assert!(evidence.peak_bytes() <= 2_000_000);
    assert_eq!(
        opened.request_count(),
        opened.with_requests(<[purrdf_core::SegmentedRequest]>::len)
    );
    assert!(
        opened.request_count() < evidence.request_count(),
        "older evidence keeps its exact prefix"
    );
    // Reopen a fresh host attachment using the same full-validation authority.
    let fresh = SegmentedBytes::new(Arc::from(image.bytes()), image.receipt().snapshot());
    let reopened = SegmentedSession::open(
        Arc::new(fresh),
        image.receipt(),
        SegmentedReadLimits::new(2_000_000, 2, 20_000, 10_000_000, 8),
    )
    .unwrap();
    for (&id, value) in ids.iter().zip(&values) {
        assert_eq!(reopened.term_value(id).unwrap(), *value);
    }
}

fn live_pins_are_charged_and_cannot_be_evicted() {
    let values: Vec<_> = (0..13)
        .map(|n| TermValue::iri(format!("http://example.org/{n}")))
        .collect();
    let (image, ids) = build(&values, 0);
    let read = session(&image, 1);
    let first = read.resolve(ids[0]).unwrap();
    let same = read.resolve(ids[1]).unwrap();
    assert_eq!(first.term(), TermRef::Iri(values[0].as_iri().unwrap()));
    assert!(matches!(
        read.resolve(ids[8]),
        Err(SegmentedError::PinnedBlocks)
    ));
    assert!(matches!(
        read.read_error(),
        Some(SegmentedError::PinnedBlocks)
    ));
    assert_eq!(
        same.term(),
        TermRef::Iri(values[1].as_iri().unwrap()),
        "live pins retain authenticated text after admission refusal"
    );
    drop(first);
    drop(same);
    // A fresh operation releases lexical pins and can evict/reload the same cache.
    let read = session(&image, 1);
    for &id in &[ids[0], ids[8], ids[0]] {
        let guard = read.resolve(id).unwrap();
        assert!(matches!(guard.term(), TermRef::Iri(_)));
    }
    assert!(read.evidence().evictions() >= 3);
}

fn exact_evidence_and_workspace_refuse_before_unbounded_growth() {
    let (image, ids) = build(
        &[
            TermValue::iri("http://example.org/p"),
            TermValue::iri("http://example.org/o"),
        ],
        0,
    );
    assert!(matches!(
        SegmentedSession::open(
            Arc::new(image.provider()),
            image.receipt(),
            SegmentedReadLimits::new(1_000_000, 1, 0, 1_000_000, 1)
        ),
        Err(SegmentedError::EvidenceExhausted)
    ));
    let read = session(&image, 1);
    let live = read.evidence().live_bytes();
    let mut reservation = read.reserve_workspace(3000).unwrap();
    assert!(
        read.evidence().live_bytes() >= live + 3000 - 1200,
        "admission may evict the old header"
    );
    reservation.resize(4000).unwrap();
    assert!(reservation.resize(3_000_000).is_err());
    drop(reservation);
    assert!(read.term_value(ids[0]).is_ok());
    assert!(read.evidence().peak_bytes() <= read.storage_live_budget().unwrap());
}

fn cache_eviction_preserves_exact_non_cache_owner_conservation() {
    let (image, ids) = build(
        &[
            TermValue::iri("http://example.org/p"),
            TermValue::iri("http://example.org/o"),
        ],
        0,
    );
    let read = session(&image, 2);
    let opening = read.evidence();
    let owners = opening.live_bytes() - opening.unpinned_cache_bytes();
    assert!(opening.unpinned_cache_bytes() > 0);
    let mut reservation = read.reserve_workspace(3000).unwrap();
    let held = read.evidence();
    assert_eq!(
        held.live_bytes() - held.unpinned_cache_bytes(),
        owners + 3000
    );
    reservation.resize(4000).unwrap();
    assert!(matches!(
        reservation.resize(3_000_000),
        Err(SegmentedError::Residency { .. })
    ));
    let refused = read.evidence();
    assert_eq!(
        refused.live_bytes() - refused.unpinned_cache_bytes(),
        owners + 4000
    );
    drop(reservation);
    let released = read.evidence();
    assert!(released.evictions() > opening.evictions());
    assert_eq!(
        released.live_bytes() - released.unpinned_cache_bytes(),
        owners
    );
    let pin = read.resolve(ids[0]).unwrap();
    let pinned = read.evidence();
    assert!(pinned.live_bytes() - pinned.unpinned_cache_bytes() > owners);
    drop(pin);
    let unpinned = read.evidence();
    assert_eq!(
        unpinned.live_bytes() - unpinned.unpinned_cache_bytes(),
        owners
    );
    assert!(read.read_error().is_none());
}

fn corruption_cannot_turn_an_omitted_fact_into_success() {
    let values: Vec<_> = (0..21)
        .map(|n| TermValue::iri(format!("http://example.org/{n:02}")))
        .collect();
    let (image, ids) = build(&values, 0);
    let mut bytes = image.bytes().to_vec();
    // Alter the first dictionary fence, leaving the receipt unchanged. Recomputed
    // attacker-controlled local checksums could not replace this certified root.
    bytes[1024 + 17 + 7] ^= 1;
    let forged = SegmentedBytes::new(bytes.into(), image.receipt().snapshot());
    let read = SegmentedSession::open(
        Arc::new(forged),
        image.receipt(),
        SegmentedReadLimits::new(2_000_000, 2, 20_000, 10_000_000, 8),
    )
    .unwrap();
    assert!(matches!(
        read.resolve(ids[0]),
        Err(SegmentedError::Corrupt(_))
    ));
    assert!(
        read.checked_read(|view| view.quads().count()).is_err(),
        "sticky corruption refuses the complete result"
    );
}

fn rdf12_references_empty_graphs_and_streamed_lines_survive() {
    let subject = TermValue::Blank {
        label: "主题".into(),
        scope: BlankScope(71),
    };
    let predicate = TermValue::iri("http://example.org/p");
    let literal = TermValue::Literal {
        lexical_form: "方向".into(),
        datatype: purrdf_iri::vocab::rdf::DIR_LANG_STRING.into(),
        language: Some("zh".into()),
        direction: Some(RdfTextDirection::Rtl),
    };
    let triple = TermValue::Triple {
        s: TermBox::new(subject.clone()),
        p: TermBox::new(predicate.clone()),
        o: TermBox::new(literal.clone()),
    };
    let values = [
        subject,
        predicate,
        literal,
        triple,
        TermValue::iri("http://example.org/claim"),
        TermValue::iri("http://example.org/g"),
        TermValue::iri("http://example.org/empty"),
    ];
    let limits = SegmentedBuildLimits::new(100, 20_000, 30, 2048, 3).unwrap();
    let mut builder = SegmentedBuilder::new(limits);
    let mut ids = Vec::new();
    builder.intern_batch(&values, |_, id| ids.push(id)).unwrap();
    builder
        .push_quad(QuadIds {
            s: ids[0],
            p: ids[1],
            o: ids[3],
            g: Some(ids[5]),
        })
        .unwrap();
    builder.push_reifier(ids[4], ids[3], Some(ids[5])).unwrap();
    builder
        .push_annotation(QuadIds {
            s: ids[4],
            p: ids[1],
            o: ids[2],
            g: Some(ids[5]),
        })
        .unwrap();
    builder.declare_named_graph(ids[6]).unwrap();
    let image = builder.seal().unwrap();
    let read = session(&image, 4);
    for (&id, value) in ids.iter().zip(&values) {
        assert_eq!(read.term_value(id).unwrap(), *value);
    }
    assert_eq!(read.checked_read(|view| view.quads().count()).unwrap(), 1);
    assert_eq!(
        read.checked_read(|view| view.reifier_quads().count())
            .unwrap(),
        1
    );
    assert_eq!(
        read.checked_read(|view| view.annotation_quads().count())
            .unwrap(),
        1
    );
    assert_eq!(
        read.checked_read(|view| view.named_graphs().count())
            .unwrap(),
        2
    );
    assert_eq!(
        read.checked_read(|view| view
            .quads_for_pattern(Some(ids[0]), None, None, GraphMatch::Named(ids[5]))
            .count())
            .unwrap(),
        1
    );
    let mut output = Vec::new();
    read.export_trig_lines(&mut purrdf_core::sink::WriterDrain(&mut output))
        .unwrap();
    let output = String::from_utf8(output).unwrap();
    assert!(output.contains("<http://example.org/empty> {}\n"));
    assert!(output.contains(purrdf_iri::vocab::rdf::REIFIES));
    assert!(output.contains("@zh--rtl"));
    assert!(output.contains("<<("));
    assert!(read.max_owned_term_bytes().unwrap() >= u64::try_from(size_of::<TermValue>()).unwrap());
}

fn retained_v1_pack_reader_migrates_into_the_versioned_representation() {
    let mut builder = purrdf_core::RdfDatasetBuilder::new();
    let p = builder.intern_iri("http://example.org/p");
    let empty = builder.intern_iri("http://example.org/empty");
    builder.push_quad(p, p, p, None);
    builder.declare_named_graph(empty);
    let source = builder.freeze().unwrap();
    let limits = SegmentedBuildLimits::new(100, 20_000, 30, 1024, 4).unwrap();
    let graph_count = |image: &SegmentedImage| {
        session(image, 3)
            .checked_read(|view| view.named_graphs().count())
            .unwrap()
    };

    // A v1 pack carries a declaration-only graph as a zero-row partition, so the
    // migration keeps it without a sidecar.
    let current = purrdf_core::PackBuilder::build_bytes(source.as_ref()).unwrap();
    let migrated = SegmentedImage::from_pack_v1(&current, limits).unwrap();
    let read = session(&migrated, 3);
    assert_eq!(read.checked_read(|view| view.quads().count()).unwrap(), 1);
    assert_eq!(
        graph_count(&migrated),
        1,
        "the pack carries the declaration"
    );
    // Restating a declaration the pack already carries is idempotent.
    let restated = SegmentedImage::from_pack_v1_with_graphs(
        &current,
        &[TermValue::iri("http://example.org/empty")],
        limits,
    )
    .unwrap();
    assert_eq!(graph_count(&restated), 1);
    let restored = purrdf_core::dataset_from_view(&read).unwrap();
    assert!(purrdf_core::datasets_isomorphic(
        source.as_ref(),
        restored.as_ref()
    ));
    assert_eq!(restored.named_graphs().count(), 1);

    // A pack written without the declaration (as every pack written before
    // declarations were carried is) still needs the sidecar to restate it.
    let mut builder = purrdf_core::RdfDatasetBuilder::new();
    let p = builder.intern_iri("http://example.org/p");
    builder.push_quad(p, p, p, None);
    let old = purrdf_core::PackBuilder::build_bytes(builder.freeze().unwrap().as_ref()).unwrap();
    assert_eq!(
        graph_count(&SegmentedImage::from_pack_v1(&old, limits).unwrap()),
        0,
        "nothing in the pack names the graph"
    );
    let with_declarations = SegmentedImage::from_pack_v1_with_graphs(
        &old,
        &[TermValue::iri("http://example.org/empty")],
        limits,
    )
    .unwrap();
    assert_eq!(graph_count(&with_declarations), 1);
}

struct RetainedManifest([u8; purrdf_core::SegmentedReceipt::ENCODED_BYTES]);
impl purrdf_core::SegmentedReceiptAuthority for RetainedManifest {
    fn authenticate(&self, bytes: &[u8]) -> Result<(), SegmentedError> {
        if bytes == self.0 {
            Ok(())
        } else {
            Err(SegmentedError::SnapshotMismatch)
        }
    }
}

fn persisted_certification_reopens_only_with_retained_authority() {
    let (image, ids) = build(&[TermValue::iri("http://example.org/original")], 0);
    let retained = RetainedManifest(image.receipt().encode());
    let restored =
        purrdf_core::SegmentedReceipt::from_authenticated_bytes(&retained.0, &retained).unwrap();
    assert_eq!(&restored, image.receipt());
    let provider = SegmentedBytes::new(Arc::from(image.bytes()), restored.snapshot());
    let read = SegmentedSession::open(
        Arc::new(provider),
        &restored,
        SegmentedReadLimits::new(1_000_000, 2, 200, 100_000, 4),
    )
    .unwrap();
    assert_eq!(
        read.term_value(ids[0]).unwrap(),
        TermValue::iri("http://example.org/original")
    );
    let mut replaced = retained.0;
    replaced[40] ^= 1;
    assert!(matches!(
        purrdf_core::SegmentedReceipt::from_authenticated_bytes(&replaced, &retained),
        Err(SegmentedError::SnapshotMismatch)
    ));
}

fn snapshot_qualified_handles_do_not_reinterpret_reused_compact_ids() {
    let (first, ids) = build(&[TermValue::iri("http://example.org/first")], 0);
    let (second, second_ids) = build(&[TermValue::iri("http://example.org/second")], 0);
    assert_eq!(ids, second_ids);
    let first_read = session(&first, 2);
    let handle = first_read.handle(ids[0]).unwrap();
    let handle = purrdf_core::SegmentedHandle::from_encoded_bytes(&handle.encode()).unwrap();
    assert_eq!(first_read.attach(handle).unwrap(), ids[0]);
    assert_eq!(session(&first, 2).attach(handle).unwrap(), ids[0]);
    assert!(matches!(
        session(&second, 2).attach(handle),
        Err(SegmentedError::SnapshotMismatch)
    ));
    assert!(
        first_read
            .handle(GlobalTermId::checked_from_index(999).unwrap())
            .is_err()
    );
}

#[derive(Debug)]
struct StoppedProvider {
    bytes: SegmentedBytes,
    stop_after: std::sync::atomic::AtomicU64,
}
impl purrdf_core::SegmentedProvider for StoppedProvider {
    fn snapshot(&self) -> purrdf_core::SegmentedSnapshot {
        self.bytes.snapshot()
    }
    fn byte_len(&self) -> u64 {
        self.bytes.byte_len()
    }
    fn read_at(&self, position: u64, output: &mut [u8]) -> Result<(), SegmentedError> {
        if self
            .stop_after
            .fetch_sub(1, std::sync::atomic::Ordering::Relaxed)
            == 0
        {
            Err(SegmentedError::Stopped(purrdf_core::StopCause::Cancelled))
        } else {
            self.bytes.read_at(position, output)
        }
    }
}

fn cancellation_retains_the_exact_failed_request_and_refuses_partial_drains() {
    let (image, ids) = build(&[TermValue::iri("http://example.org/p")], 0);
    let provider = Arc::new(StoppedProvider {
        bytes: image.provider(),
        stop_after: std::sync::atomic::AtomicU64::new(u64::MAX),
    });
    let read = SegmentedSession::open(
        provider.clone(),
        image.receipt(),
        SegmentedReadLimits::new(1_000_000, 2, 200, 100_000, 4),
    )
    .unwrap();
    let before = read.evidence();
    provider
        .stop_after
        .store(0, std::sync::atomic::Ordering::Relaxed);
    assert!(matches!(
        read.resolve(ids[0]),
        Err(SegmentedError::Stopped(_))
    ));
    let after = read.evidence();
    assert_eq!(after.request_count(), before.request_count() + 1);
    after.with_requests(|requests| assert!(!requests.last().unwrap().completed()));
    assert!(read.checked_read(|view| view.quads().count()).is_err());
    assert!(purrdf_core::try_blank_count_view(&read).is_err());
}

fn charged_read_peak_covers_measured_allocations_including_streamed_output() {
    let values = (0..125)
        .map(|n| TermValue::iri(format!("http://example.org/{n:04}")))
        .collect::<Vec<_>>();
    let (image, ids) = build(&values, 0);
    let provider = Arc::new(image.provider());
    let mut drain = purrdf_core::sink::Measure::default();
    // Provider-owned immutable fixture bytes exist before this window. The read
    // session must charge all of its own cache, index, scratch, sink and evidence.
    let window = purrdf_alloc_probe::CurrentThreadWindow::open();
    let read = SegmentedSession::open(
        provider,
        image.receipt(),
        SegmentedReadLimits::new(256 * 1024, 4, 2000, 10_000_000, 8),
    )
    .unwrap();
    for &id in &ids {
        let guard = read.resolve(id).unwrap();
        std::hint::black_box(guard.term());
    }
    let evidence = read.export_trig_lines(&mut drain).unwrap();
    let measured = window.close();
    assert!(
        u64::try_from(measured.peak_working_bytes).unwrap() <= evidence.peak_bytes(),
        "all measured reader/output allocations must fit inside the conservative ledger"
    );
    assert!(evidence.peak_bytes() <= 256 * 1024);
    assert!(evidence.evictions() > 0);
}

fn pin_refusal_precedes_io_and_owned_pin_survives_session_drop() {
    let (image, ids) = build(&[TermValue::iri("http://example.org/p")], 0);
    let read = SegmentedSession::open(
        Arc::new(image.provider()),
        image.receipt(),
        SegmentedReadLimits::new(1_000_000, 2, 200, 100_000, 0),
    )
    .unwrap();
    let before = read.evidence().request_count();
    assert!(matches!(
        read.resolve(ids[0]),
        Err(SegmentedError::PinnedBlocks)
    ));
    assert_eq!(read.evidence().request_count(), before);
    let guard = {
        let read = session(&image, 2);
        read.resolve(ids[0]).unwrap()
    };
    assert_eq!(guard.term(), TermRef::Iri("http://example.org/p"));
}

fn composite_lexical_blank_references_keep_their_scoped_dictionary_closure() {
    let mut values = vec![TermValue::iri("https://example.org/p")];
    let scopes = [BlankScope(71), BlankScope(72)];
    for scope in scopes {
        let label = scope.qualify_label("lexical-only");
        values.push(TermValue::typed_literal(
            format!("[_:{label}, [_:{label}], {{\"key\": _:{label}}}]"),
            purrdf_cdt::datatype::CDT_LIST,
        ));
    }
    let (image, ids) = build(&values, (1_u64 << 33) + 1);
    let read = session(&image, 2);
    for (index, scope) in scopes.into_iter().enumerate() {
        let literal = read.term_value(ids[index + 1]).unwrap();
        assert_eq!(literal, values[index + 1]);
        let TermValue::Literal {
            lexical_form,
            datatype,
            ..
        } = literal
        else {
            panic!("composite fixture is a literal")
        };
        let references = purrdf_core::cdt_blank::cdt_embedded_blanks(&lexical_form, &datatype);
        assert_ne!(references.len(), 0);
        for (label, actual_scope) in references {
            assert_eq!(actual_scope, scope);
            let value = TermValue::Blank { label, scope };
            let id = read
                .term_id_by_value(&value)
                .unwrap()
                .expect("embedded blank is in the certified dictionary");
            assert_eq!(read.term_value(id).unwrap(), value);
        }
    }
}

purrdf_testkit::harness_main!(
    indexed_reopen_preserves_high_ids_and_bidirectional_batches,
    live_pins_are_charged_and_cannot_be_evicted,
    exact_evidence_and_workspace_refuse_before_unbounded_growth,
    cache_eviction_preserves_exact_non_cache_owner_conservation,
    corruption_cannot_turn_an_omitted_fact_into_success,
    rdf12_references_empty_graphs_and_streamed_lines_survive,
    retained_v1_pack_reader_migrates_into_the_versioned_representation,
    persisted_certification_reopens_only_with_retained_authority,
    snapshot_qualified_handles_do_not_reinterpret_reused_compact_ids,
    cancellation_retains_the_exact_failed_request_and_refuses_partial_drains,
    charged_read_peak_covers_measured_allocations_including_streamed_output,
    pin_refusal_precedes_io_and_owned_pin_survives_session_drop,
    composite_lexical_blank_references_keep_their_scoped_dictionary_closure,
);
