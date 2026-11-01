// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The public compact carrier, original canonical home and operational boundary.

use purrdf_core::{
    BlankScope, CanonHash, ContentIdScheme, DatasetView, FallibleDatasetView, GraphMatch,
    GraphSelector, QuadHandle, RdfDataset, RdfDatasetBuilder, RdfLiteral, RdfLocation, RdfTerm,
    RdfTextDirection, TermId, TermRef, TermValue, TripleDataset, TripleDatasetBuilder, TripleIds,
    TripleReadError, TripleReifierRow, ViewOperationStatus, WorkspaceReservation,
    try_canonicalize_flat_view, try_flat_digest_view,
};
use std::sync::Arc;

fn statement_fixture(label: &str, scope: BlankScope) -> (RdfDataset, [TermId; 4]) {
    let mut builder = RdfDatasetBuilder::with_content_addressing(
        ContentIdScheme::new("urn:cid:").unwrap(),
        Some("urn:test:derives".into()),
    );
    let s = builder.intern_blank(label, scope).unwrap();
    let p = builder.intern_iri("urn:test:predicate").unwrap();
    let o = builder
        .intern_literal(RdfLiteral::language_tagged("Bonjour", "FR"))
        .unwrap();
    let quoted = builder.intern_triple(s, p, o).unwrap();
    let outer = builder.intern_triple(s, p, quoted).unwrap();
    let reifier = builder.intern_iri("urn:test:reifier").unwrap();
    let handle = builder.push_quad_with_handle(s, p, outer, None).unwrap();
    builder.attach_location(
        handle,
        RdfLocation {
            line: Some(19),
            ..RdfLocation::default()
        },
    );
    builder.push_quad(s, p, o, None).unwrap();
    builder.push_reifier(reifier, quoted).unwrap();
    builder.push_annotation(reifier, p, o).unwrap();
    let composite = builder
        .intern_owned_term_scoped(
            &RdfTerm::Literal(RdfLiteral::typed(
                format!("[_:outer, '[_:inner]'^^<{}>]", purrdf_cdt::CDT_LIST),
                purrdf_cdt::CDT_LIST,
            )),
            BlankScope(73),
        )
        .unwrap();
    builder.push_quad(s, p, composite, None).unwrap();
    let mut directional = RdfLiteral::language_tagged("مرحبا", "ar");
    directional.direction = Some(RdfTextDirection::Rtl);
    let directional = builder.intern_literal(directional).unwrap();
    builder.push_annotation(reifier, p, directional).unwrap();
    (
        Arc::try_unwrap(builder.freeze().unwrap()).unwrap(),
        [s, p, o, reifier],
    )
}

fn compact_layout_and_all_bound_shapes() {
    assert_eq!(size_of::<TripleIds>(), 12);
    assert_eq!(size_of::<TripleReifierRow>(), 8);
    let mut builder = TripleDatasetBuilder::default();
    let ids: Vec<_> = (0..7)
        .map(|i| builder.intern_iri(&format!("urn:test:{i}")).unwrap())
        .collect();
    for s in 0..7 {
        for p in 0..7 {
            let o = (s * 3 + p * 5) % 7;
            builder.push_triple(ids[s], ids[p], ids[o]).unwrap();
            builder.push_triple(ids[s], ids[p], ids[o]).unwrap();
        }
    }
    let dataset = builder.freeze().unwrap();
    assert_eq!(dataset.len(), 49);
    let cold = dataset.storage_layout();
    assert_eq!(cold.triples, 12 * 49);
    assert_eq!(cold.triple_orders, [0, 0]);
    assert_eq!(cold.reifiers, 0);
    assert_eq!(cold.annotations, 0);
    for mask in 0..8 {
        for &s in &ids {
            for &p in &ids {
                for &o in &ids {
                    let pattern = [
                        (mask & 1 != 0).then_some(s),
                        (mask & 2 != 0).then_some(p),
                        (mask & 4 != 0).then_some(o),
                    ];
                    let mut expected: Vec<_> = dataset
                        .triples()
                        .filter(|row| {
                            [row.s, row.p, row.o]
                                .into_iter()
                                .zip(pattern)
                                .all(|(actual, bound)| bound.is_none_or(|bound| actual == bound))
                        })
                        .collect();
                    let mut actual: Vec<_> = dataset
                        .triples_for_pattern(pattern[0], pattern[1], pattern[2])
                        .unwrap()
                        .collect();
                    expected.sort_unstable();
                    actual.sort_unstable();
                    assert_eq!(actual, expected, "mask {mask}, {s:?}/{p:?}/{o:?}");
                }
            }
        }
    }
    assert_eq!(dataset.storage_layout().triple_orders, [4 * 49; 2]);
}

fn compact_statement_probes_populate_only_the_requested_ordinal_arrays() {
    let mut builder = TripleDatasetBuilder::default();
    let ids: Vec<_> = (0..8)
        .map(|i| builder.intern_iri(&format!("urn:side:{i}")).unwrap())
        .collect();
    let mut targets = Vec::new();
    for &id in &ids {
        targets.push(builder.intern_triple(id, ids[0], ids[1]).unwrap());
    }
    for (r, &reifier) in ids.iter().enumerate() {
        builder.push_reifier(reifier, targets[(r * 3) % 8]).unwrap();
        builder
            .push_annotation(reifier, ids[(r + 1) % 8], ids[(r * 5) % 8])
            .unwrap();
    }
    let dataset = builder.freeze().unwrap();
    let cold = dataset.storage_layout();
    assert_eq!(
        (cold.triples, cold.reifiers, cold.annotations),
        (0, 8 * 8, 12 * 8)
    );
    assert_eq!((cold.reifier_targets, cold.annotation_orders), (0, [0, 0]));
    for reifier in std::iter::once(None).chain(ids.iter().copied().map(Some)) {
        for target in std::iter::once(None).chain(targets.iter().copied().map(Some)) {
            let expected: Vec<_> = dataset
                .reifiers()
                .filter(|&(r, t)| {
                    reifier.is_none_or(|r0| r0 == r) && target.is_none_or(|t0| t0 == t)
                })
                .collect();
            let mut actual: Vec<_> = dataset
                .reifiers_for_pattern(reifier, target)
                .unwrap()
                .collect();
            actual.sort_unstable();
            assert_eq!(actual, expected);
        }
    }
    assert_eq!(dataset.storage_layout().reifier_targets, 4 * 8);
    assert_eq!(dataset.storage_layout().reifier_components, [0; 7]);
    for mask in 0..8 {
        for target in targets.iter().copied().chain([ids[7]]) {
            let parts = match dataset.resolve(target) {
                TermRef::Triple { s, p, o } => [s, p, o],
                _ => [target; 3],
            };
            let pattern =
                std::array::from_fn(|index| (mask & (1 << index) != 0).then_some(parts[index]));
            let expected: Vec<_> = dataset
                .reifiers()
                .filter(|&(_, target)| {
                    let TermRef::Triple { s, p, o } = dataset.resolve(target) else {
                        panic!("validated triple target");
                    };
                    [s, p, o]
                        .into_iter()
                        .zip(pattern)
                        .all(|(actual, bound)| bound.is_none_or(|value| value == actual))
                })
                .collect();
            let actual = dataset.reifiers_for_components(pattern).unwrap();
            assert_eq!(actual.len(), expected.len());
            assert_eq!(actual.collect::<Vec<_>>(), expected);
        }
    }
    assert_eq!(dataset.storage_layout().reifier_components, [4 * 8; 7]);
    let read = Arc::new(dataset.read().unwrap());
    for &target in targets.iter().chain(ids.iter()) {
        let expected: Vec<_> = read
            .reifier_quads()
            .filter(|quad| quad.o == target)
            .collect();
        for graph in [GraphMatch::Any, GraphMatch::Default] {
            assert_eq!(
                read.reifier_quads_for_target(target, graph)
                    .collect::<Vec<_>>(),
                expected
            );
        }
    }
    assert_eq!(dataset.storage_layout().annotation_orders, [0, 0]);
    for mask in 0..8 {
        for row in dataset.annotations() {
            let pattern = [
                (mask & 1 != 0).then_some(row.s),
                (mask & 2 != 0).then_some(row.p),
                (mask & 4 != 0).then_some(row.o),
            ];
            let expected: Vec<_> = dataset
                .annotations()
                .filter(|row| {
                    [row.s, row.p, row.o]
                        .into_iter()
                        .zip(pattern)
                        .all(|(actual, bound)| bound.is_none_or(|value| value == actual))
                })
                .collect();
            let mut actual: Vec<_> = dataset
                .annotations_for_pattern(pattern[0], pattern[1], pattern[2])
                .unwrap()
                .collect();
            actual.sort_unstable();
            assert_eq!(actual, expected);
        }
    }
    let warm = dataset.storage_layout();
    assert_eq!(warm.annotation_orders, [4 * 8; 2]);
    assert_eq!(warm.triple_orders, [0, 0]);
}

fn owned_transfer_preserves_lexical_owner_and_flat_statement_identity() {
    let (quad, [s, p, o, reifier]) = statement_fixture("claim", BlankScope(71));
    let nesting_bound = quad.triple_term_nesting_bound();
    assert_eq!(nesting_bound, Some(purrdf_events::MAX_TERM_NESTING_DEPTH));
    let before = try_canonicalize_flat_view(
        &quad,
        CanonHash::Sha256,
        &mut purrdf_core::CanonicalizationControl::new(purrdf_core::RDFC_V2_CORPUS_POLICY, None),
    )
    .unwrap();
    let digest = try_flat_digest_view(
        &quad,
        CanonHash::Sha256,
        &mut purrdf_core::CanonicalizationControl::new(purrdf_core::RDFC_V2_CORPUS_POLICY, None),
    )
    .unwrap();
    let TermRef::Iri(iri) = quad.resolve(p) else {
        panic!("predicate IRI")
    };
    let original_pointer = iri.as_ptr();
    let locations: Vec<_> = quad
        .quads()
        .enumerate()
        .filter_map(|(ordinal, _)| {
            let handle = QuadHandle::from_index(u32::try_from(ordinal).unwrap());
            quad.location_of(handle)
                .map(|location| (handle, location.clone()))
        })
        .collect();
    let annotation_count = quad.annotations_with_graph().count();
    let compact = TripleDataset::try_from(quad).unwrap();
    let TermRef::Iri(iri) = compact.resolve(p) else {
        panic!("predicate IRI")
    };
    assert_eq!(
        iri.as_ptr(),
        original_pointer,
        "lexical allocation moved intact"
    );
    assert_eq!(compact.reifiers().count(), 1);
    assert_eq!(compact.annotations().count(), annotation_count);
    assert!(
        compact
            .annotations()
            .any(|row| row == TripleIds { s: reifier, p, o })
    );
    assert_eq!(
        compact.content_id_scheme(),
        Some(&ContentIdScheme::new("urn:cid:").unwrap())
    );
    assert_eq!(compact.derivation_predicate_iri(), Some("urn:test:derives"));
    for (handle, location) in locations {
        assert_eq!(compact.location_of(handle), Some(&location));
    }
    assert!(matches!(
        compact.resolve(s),
        TermRef::Blank {
            scope: BlankScope(71),
            ..
        }
    ));
    let read = compact.read().unwrap();
    assert_eq!(read.triple_term_nesting_bound(), nesting_bound);
    assert_eq!(
        DatasetView::annotations_of_with_graph(&read, reifier).count(),
        annotation_count
    );
    let materialized = purrdf_core::dataset_from_view(&read)
        .expect("scoped triple read materializes before retaining canonical output");
    assert_eq!(
        try_canonicalize_flat_view(
            &*materialized,
            CanonHash::Sha256,
            &mut purrdf_core::CanonicalizationControl::new(
                purrdf_core::RDFC_V2_CORPUS_POLICY,
                None
            )
        )
        .unwrap()
        .nquads(),
        before.nquads()
    );
    assert_eq!(
        try_flat_digest_view(
            &*materialized,
            CanonHash::Sha256,
            &mut purrdf_core::CanonicalizationControl::new(
                purrdf_core::RDFC_V2_CORPUS_POLICY,
                None
            )
        )
        .unwrap(),
        digest
    );
    assert!(read.read_error().is_none());

    let (relabeled, [renamed, ..]) = statement_fixture("renamed-subject", BlankScope(901));
    assert!(matches!(
        relabeled.resolve(renamed),
        TermRef::Blank {
            label: "renamed-subject",
            scope: BlankScope(901),
        }
    ));
    assert_eq!(
        try_canonicalize_flat_view(
            &relabeled,
            CanonHash::Sha256,
            &mut purrdf_core::CanonicalizationControl::new(
                purrdf_core::RDFC_V2_CORPUS_POLICY,
                None
            )
        )
        .unwrap()
        .nquads(),
        before.nquads()
    );
    assert_eq!(
        try_flat_digest_view(
            &relabeled,
            CanonHash::Sha256,
            &mut purrdf_core::CanonicalizationControl::new(
                purrdf_core::RDFC_V2_CORPUS_POLICY,
                None
            )
        )
        .unwrap(),
        digest
    );
    let relabeled = TripleDataset::try_from(relabeled).unwrap();
    let relabeled_read = relabeled.read().unwrap();
    assert_eq!(
        try_canonicalize_flat_view(
            &relabeled_read,
            CanonHash::Sha256,
            &mut purrdf_core::CanonicalizationControl::new(
                purrdf_core::RDFC_V2_CORPUS_POLICY,
                None
            )
        )
        .unwrap()
        .nquads(),
        before.nquads()
    );
    assert_eq!(
        try_flat_digest_view(
            &relabeled_read,
            CanonHash::Sha256,
            &mut purrdf_core::CanonicalizationControl::new(
                purrdf_core::RDFC_V2_CORPUS_POLICY,
                None
            )
        )
        .unwrap(),
        digest
    );
    assert!(relabeled_read.read_error().is_none());
}

fn conversion_refuses_every_named_role_including_empty_declarations() {
    for role in 0..4 {
        let mut builder = RdfDatasetBuilder::new();
        let s = builder.intern_iri("urn:test:subject").unwrap();
        let p = builder.intern_iri("urn:test:predicate").unwrap();
        let g = builder.intern_iri("urn:test:graph").unwrap();
        let quoted = builder.intern_triple(s, p, s).unwrap();
        match role {
            0 => builder.push_quad(s, p, s, Some(g)).unwrap(),
            1 => builder.push_reifier_in_graph(s, quoted, Some(g)).unwrap(),
            2 => builder.push_annotation_in_graph(s, p, s, Some(g)).unwrap(),
            _ => builder.declare_named_graph(g).unwrap(),
        }
        let dataset = Arc::try_unwrap(builder.freeze().unwrap()).unwrap();
        assert_eq!(
            TripleDataset::try_from(dataset).unwrap_err(),
            TripleReadError::NamedGraphsUnsupported
        );
    }
}

fn refused_child_stops_existing_streams_and_fresh_session_is_healthy() {
    let (quad, [s, p, o, _]) = statement_fixture("claim", BlankScope(71));
    let compact = TripleDataset::try_from(quad).unwrap();
    let read = compact.read().unwrap();
    let child = read.fork().unwrap();
    let mut stream = read.quads();
    let mut workspace = read.reserve_workspace(32).unwrap();
    assert!(stream.next().is_some());
    let refuse_named = || {
        child
            .quads_for_pattern(None, None, None, GraphMatch::Named(p))
            .count()
    };
    #[cfg(not(target_family = "wasm"))]
    let refused = std::thread::scope(|scope| scope.spawn(refuse_named).join().unwrap());
    #[cfg(target_family = "wasm")]
    let refused = refuse_named();
    assert_eq!(refused, 0);
    assert_eq!(stream.next(), None);
    assert_eq!(
        workspace.resize(0),
        Err(TripleReadError::NamedGraphsUnsupported)
    );
    assert_eq!(
        read.resolve(s),
        Err(TripleReadError::NamedGraphsUnsupported)
    );
    assert_eq!(
        read.checked_read(|_| ()),
        Err(TripleReadError::NamedGraphsUnsupported)
    );
    assert!(matches!(
        read.operation_status(),
        ViewOperationStatus::Failed {
            error: TripleReadError::NamedGraphsUnsupported,
            ..
        }
    ));
    let healthy = compact.read().unwrap();
    // A new indexed seam obeys the same graph-free refusal and sticky cause,
    // even when its target is absent. A fresh read remains independent.
    let refused_target = compact.read().unwrap();
    assert_eq!(
        refused_target
            .reifier_quads_for_target(s, GraphMatch::Named(s))
            .count(),
        0
    );
    assert_eq!(
        refused_target.read_error(),
        Some(TripleReadError::NamedGraphsUnsupported)
    );
    assert_eq!(
        refused_target
            .reifier_quads_for_target(s, GraphMatch::Default)
            .count(),
        0
    );
    assert_eq!(
        healthy
            .quads_for_pattern(Some(s), Some(p), Some(o), GraphMatch::Default)
            .count(),
        1
    );
    assert!(healthy.read_error().is_none());
    let empty = TripleDatasetBuilder::default().freeze().unwrap();
    let empty_read = empty.read().unwrap();
    assert_eq!(
        empty_read.require_named_graphs(),
        Err(TripleReadError::NamedGraphsUnsupported)
    );
    assert_eq!(
        empty_read.checked_read(|v| v.quads().count()),
        Err(TripleReadError::NamedGraphsUnsupported)
    );
    let selector = GraphSelector::Named(TermValue::iri("urn:missing:graph"));
    for dataset in [&compact, &empty] {
        let read = Arc::new(dataset.read().unwrap());
        assert!(!read.supports_named_graphs());
        assert_eq!(
            selector.resolve(&read),
            Err(TripleReadError::NamedGraphsUnsupported)
        );
        assert_eq!(
            read.read_error(),
            Some(TripleReadError::NamedGraphsUnsupported)
        );
    }
    let quad = RdfDatasetBuilder::new().freeze().unwrap();
    assert_eq!(selector.resolve(&quad), Ok(None));
}

fn statement_mask_probes_preserve_frozen_order_and_graph_multiplicity() {
    let mut builder = RdfDatasetBuilder::new();
    let ids: Vec<_> = (0..9)
        .map(|index| builder.intern_iri(&format!("urn:mask:{index}")).unwrap())
        .collect();
    for index in 0..64 {
        let point = (index * 37) % 64;
        let target = builder
            .intern_triple(ids[point / 16], ids[(point / 4) % 4], ids[point % 4])
            .unwrap();
        let reifier = ids[4 + index % 4];
        let graph = (index % 3 == 0).then_some(ids[8]);
        builder
            .push_reifier_in_graph(reifier, target, graph)
            .unwrap();
        builder
            .push_annotation_in_graph(reifier, ids[(point / 8) % 4], ids[point % 4], graph)
            .unwrap();
        if point % 5 == 0 {
            builder
                .push_reifier_in_graph(reifier, target, None)
                .unwrap();
            builder
                .push_reifier_in_graph(reifier, target, Some(ids[8]))
                .unwrap();
        }
    }
    let dataset = builder.freeze().unwrap();
    for mask in 0..8 {
        for point in 0..=64 {
            let parts = if point == 64 {
                [ids[8]; 3]
            } else {
                [ids[point / 16], ids[(point / 4) % 4], ids[point % 4]]
            };
            let pattern =
                std::array::from_fn(|index| (mask & (1 << index) != 0).then_some(parts[index]));
            for graph in [
                GraphMatch::Any,
                GraphMatch::Default,
                GraphMatch::Named(ids[8]),
                GraphMatch::Named(ids[0]),
            ] {
                let expected: Vec<_> = dataset
                    .reifier_quads()
                    .filter(|quad| {
                        let TermRef::Triple { s, p, o } = dataset.as_ref().resolve(quad.o) else {
                            panic!("validated target");
                        };
                        graph.matches(quad.g)
                            && [s, p, o]
                                .into_iter()
                                .zip(pattern)
                                .all(|(actual, bound)| bound.is_none_or(|value| value == actual))
                    })
                    .collect();
                assert_eq!(
                    dataset
                        .reifier_quads_for_components(pattern, graph)
                        .collect::<Vec<_>>(),
                    expected
                );
            }
        }
    }
    for predicate in std::iter::once(None).chain(ids.iter().copied().map(Some)) {
        for object in std::iter::once(None).chain(ids.iter().copied().map(Some)) {
            for graph in [
                GraphMatch::Any,
                GraphMatch::Default,
                GraphMatch::Named(ids[8]),
                GraphMatch::Named(ids[0]),
            ] {
                let expected: Vec<_> = dataset
                    .annotation_quads()
                    .filter(|quad| {
                        predicate.is_none_or(|value| value == quad.p)
                            && object.is_none_or(|value| value == quad.o)
                            && graph.matches(quad.g)
                    })
                    .collect();
                assert_eq!(
                    dataset
                        .annotation_quads_for_pattern(predicate, object, graph)
                        .collect::<Vec<_>>(),
                    expected
                );
            }
        }
    }
}

purrdf_testkit::harness_main!(
    statement_mask_probes_preserve_frozen_order_and_graph_multiplicity,
    compact_statement_probes_populate_only_the_requested_ordinal_arrays,
    compact_layout_and_all_bound_shapes,
    owned_transfer_preserves_lexical_owner_and_flat_statement_identity,
    conversion_refuses_every_named_role_including_empty_declarations,
    refused_child_stops_existing_streams_and_fresh_session_is_healthy,
);
