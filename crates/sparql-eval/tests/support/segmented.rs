// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Persistent selective-query fixtures shared by correctness tests and benchmarks.

use purrdf_core::{
    GlobalTermId, QuadIds, RdfDataset, RdfDatasetBuilder, SegmentedBuildLimits, SegmentedBuilder,
    SegmentedImage, SegmentedReadLimits, SegmentedSession, TermValue,
};
use std::sync::Arc;

/// A two-pattern selective join whose answer is identical on both backends.
pub const QUERY: &str = "SELECT ?label WHERE { <https://example.org/target> <https://example.org/p> ?label . <https://example.org/target> <https://example.org/q> ?label }";
/// Shared storage/operator ceiling for the small fixture.
pub const CEILING: u64 = 8_000_000;

/// Build the sparse-cache correctness fixture.
pub fn fixture() -> (SegmentedImage, Arc<RdfDataset>) {
    fixture_with_layout(512, 4)
}

/// Build the same RDF rows using explicit dictionary/block geometry.
pub fn fixture_with_layout(
    block_bytes: u32,
    records_per_block: u32,
) -> (SegmentedImage, Arc<RdfDataset>) {
    let limits = SegmentedBuildLimits::new(4096, 500_000, 4096, block_bytes, records_per_block)
        .unwrap()
        .with_first_term_index((1_u64 << 53) + 1)
        .unwrap();
    let mut persistent = SegmentedBuilder::new(limits);
    let mut resident = RdfDatasetBuilder::new();
    let p = TermValue::iri("https://example.org/p");
    let q = TermValue::iri("https://example.org/q");
    // Unrelated rows and dictionary blocks must not turn selective join admission
    // into a reservation for the whole dictionary or a full materialization.
    let mut triples = Vec::new();
    for n in 0..200 {
        triples.push([
            TermValue::iri(format!("https://example.org/unrelated/{n:04}")),
            p.clone(),
            TermValue::simple_literal(format!("unrelated 中文 {n:04}")),
        ]);
    }
    let target = TermValue::iri("https://example.org/target");
    let label = TermValue::lang_literal("漢字", "zh");
    triples.push([target.clone(), p, label.clone()]);
    triples.push([target, q, label]);
    for triple in &triples {
        let mut ids: Vec<GlobalTermId> = Vec::new();
        persistent
            .intern_batch(triple, |_, id| ids.push(id))
            .unwrap();
        persistent
            .push_quad(QuadIds {
                s: ids[0],
                p: ids[1],
                o: ids[2],
                g: None,
            })
            .unwrap();
        let s = purrdf_core::term_fixture::intern_value(&mut resident, &triple[0]);
        let p = purrdf_core::term_fixture::intern_value(&mut resident, &triple[1]);
        let o = purrdf_core::term_fixture::intern_value(&mut resident, &triple[2]);
        resident.push_quad(s, p, o, None);
    }
    (persistent.seal().unwrap(), resident.freeze().unwrap())
}

/// Open a cold session with the sparse two-block cache used by correctness tests.
pub fn open(image: &SegmentedImage, ceiling: u64) -> SegmentedSession {
    open_with_cache(image, ceiling, 2)
}

/// The actual fixed session footprint, leaving no room for an owned query control.
pub fn baseline_ceiling(image: &SegmentedImage) -> u64 {
    let source = open(image, CEILING);
    source.evidence().live_bytes()
}

/// Open a session with explicit cache occupancy for hot/cold benchmark cases.
pub fn open_with_cache(
    image: &SegmentedImage,
    ceiling: u64,
    cache_blocks: u32,
) -> SegmentedSession {
    open_with_evidence(image, ceiling, cache_blocks, 2048)
}

/// Explicit evidence capacity for computed-query matrices with many reverse lookups.
/// This independent dimension does not change the physical live-byte ceiling.
pub fn open_with_evidence(
    image: &SegmentedImage,
    ceiling: u64,
    cache_blocks: u32,
    evidence_entries: u32,
) -> SegmentedSession {
    SegmentedSession::open(
        Arc::new(image.provider()),
        image.receipt(),
        SegmentedReadLimits::new(ceiling, cache_blocks, evidence_entries, 20_000_000, 8),
    )
    .unwrap()
}
