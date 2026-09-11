// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The ONE definition of the "keystone" GTS-ingestion fixture.
//!
//! This module exists for THIS crate's own tests and benches, exactly as
//! [`capture_support`](crate::capture_support) and
//! [`gts_dict_vectors`](crate::gts_dict_vectors) do: a `tests/` integration
//! target and a `benches/` target are separate crates that can only share code
//! through the library, and a bench cannot `include!` a test file. The fixture is
//! long, and its details are load-bearing — every element is a way the view
//! ingestion path and the flat carrier path could disagree — so two copies of it
//! is two definitions of what "the keystone shape" means, and only one of them
//! gets updated when a trap is added.
//!
//! It is `#[doc(hidden)]` at the crate root: shipped, because the targets that
//! consume it are built from the same crate, but not public API. Nothing here is
//! part of any stability promise, and no shipping code path calls it.
//!
//! # What the shape carries, and why
//!
//! * **Two quad-bearing named graphs plus default-graph rows**, so graph
//!   partitioning and default-graph relocation are both exercised.
//! * **One blank LABEL living in two scopes per group** — two nodes a
//!   label-keyed identity would conflate. They are kept structurally
//!   distinguishable so canonicalization stays linear rather than exploring
//!   automorphisms.
//! * **All three literal shapes**: bare, language-tagged, and a directional
//!   `ar`/`rtl` literal whose direction is part of its identity.
//! * **The RDF 1.2 statement layer** every eighth group: a triple term, a
//!   reifier binding in a named graph, and an annotation on that reifier.
//! * **One named graph declared and never filled**, which the ingest report must
//!   NAME as omitted rather than silently skip.

use std::sync::Arc;

use crate::gts_compose::{DEFAULT_RSYNCABLE_THRESHOLD, MediumPlan, SnapshotBuilder, emit_gts};
use crate::{
    BlankScope, ContentStore, DatasetMut, DatasetProvenance, DeltaDatasetView, MutableDataset,
    QuadValues, RdfDataset, RdfDatasetBuilder, RdfLiteral, RdfLookaside, RdfTextDirection,
    TermValue,
};

/// The named graph every relocated default-graph row lands in.
pub const SELECTED: &str = "https://example.org/selected";
/// The base's first quad-bearing named graph — scoped blanks, triple terms and
/// the statement layer live here.
pub const KEY_G1: &str = "https://example.org/keystone/g1";
/// The base's second quad-bearing named graph.
pub const KEY_G2: &str = "https://example.org/keystone/g2";
/// The graph a contribution is folded into; absent from the base.
pub const KEY_G3: &str = "https://example.org/keystone/g3";
/// Declared by the base and left empty everywhere — the graph the ingest report
/// must NAME rather than drop.
pub const KEY_DECLARED: &str = "https://example.org/keystone/declared";
/// How many row groups the test-side base carries. Six ordinary quads per group,
/// plus a statement-layer triple every eighth group.
pub const KEYSTONE_GROUPS: usize = 60;

/// A largish shared base of `groups` row groups: quads across two named graphs
/// and the default graph, one blank label living in two scopes per group, triple
/// terms, reifier bindings, statement annotations, all three literal shapes — and
/// one named graph declared and never filled.
///
/// See the module docs for what each element is there to catch.
pub fn keystone_base(groups: usize) -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let p = b.intern_iri("https://example.org/p");
    let q = b.intern_iri("https://example.org/q");
    let g1 = b.intern_iri(KEY_G1);
    let g2 = b.intern_iri(KEY_G2);
    let declared = b.intern_iri(KEY_DECLARED);
    let plain = b.intern_literal(RdfLiteral::simple("bare"));
    let tagged = b.intern_literal(RdfLiteral::language_tagged("cat", "en"));
    let directional = b.intern_literal(RdfLiteral {
        direction: Some(RdfTextDirection::Rtl),
        ..RdfLiteral::language_tagged("مرحبا", "ar")
    });
    for index in 0..groups {
        let s = b.intern_iri(&format!("https://example.org/s{index}"));
        let o = b.intern_iri(&format!("https://example.org/o{index}"));
        // Default-graph rows, relocated to <selected> at ingest time.
        b.push_quad(s, p, o, None);
        b.push_quad(s, q, plain, None);
        // Named-graph rows, one literal shape each.
        b.push_quad(s, p, directional, Some(g1));
        b.push_quad(s, q, tagged, Some(g2));
        // One local label in two scopes: two nodes a label-keyed identity would
        // conflate, kept structurally distinguishable so canonicalization stays
        // linear rather than exploring automorphisms.
        let shared = b.intern_blank(&format!("n{index}"), BlankScope::DEFAULT);
        let scoped = b.intern_blank(&format!("n{index}"), BlankScope(4));
        b.push_quad(shared, p, o, Some(g1));
        b.push_quad(scoped, q, o, Some(g1));
        if index % 8 == 0 {
            let triple = b.intern_triple(shared, p, directional);
            let reifier = b.intern_blank(&format!("st{index}"), BlankScope(7));
            b.push_reifier_in_graph(reifier, triple, Some(g1));
            b.push_annotation_in_graph(reifier, q, tagged, Some(g1));
        }
    }
    // A named graph owning no row at all.
    b.declare_named_graph(declared);
    b.freeze().expect("the keystone base freezes")
}

/// A small contribution of `rows` row pairs, wholly contained in `graph`, whose
/// blanks deliberately reuse the base's LABEL and scopes: a carrier that renamed
/// scopes carelessly would either conflate these nodes with the base's or break
/// their co-reference, and either moves the emitted bytes.
///
/// `space` selects the term space: two contributions built at the same `space`
/// share every IRI they name, two built at distinct ones share none. Containment
/// is checked before anything is folded, so each contribution names exactly one
/// graph — which is why a shape varies term overlap rather than row overlap.
pub fn keystone_contribution(graph: &str, space: usize, rows: usize) -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let p = b.intern_iri("https://example.org/p");
    let q = b.intern_iri("https://example.org/q");
    let g = b.intern_iri(graph);
    let node = b.intern_blank("n0", BlankScope::DEFAULT);
    let elsewhere = b.intern_blank("n0", BlankScope(4));
    for index in 0..rows {
        let o = b.intern_iri(&format!("https://example.org/space{space}/c{index}"));
        b.push_quad(node, p, o, Some(g));
        b.push_quad(elsewhere, q, o, Some(g));
    }
    b.push_quad(node, q, elsewhere, Some(g));
    b.freeze().expect("the keystone contribution freezes")
}

/// A delta view whose EFFECTIVE content is exactly `base`'s, reached through a
/// real mutation round trip rather than an untouched passthrough — so the delta
/// machinery (suppression rows, delta-only ids) is genuinely in the read path.
pub fn keystone_delta(base: &Arc<RdfDataset>) -> Arc<DeltaDatasetView> {
    let mut mutable = MutableDataset::new(Arc::clone(base));
    let scratch = QuadValues::triple(
        TermValue::iri("https://example.org/scratch"),
        TermValue::iri("https://example.org/p"),
        TermValue::iri("https://example.org/o"),
    );
    assert!(
        mutable
            .insert(scratch.clone())
            .expect("the scratch row inserts")
    );
    assert!(mutable.remove(&scratch), "and is taken back out again");
    Arc::new(mutable.snapshot_view().expect("the delta publishes"))
}

/// The sidecars every keystone carrier travels with. Identical on every carrier
/// built from this fixture, so nothing here can explain a byte difference.
pub fn keystone_loadout() -> (RdfLookaside, Arc<ContentStore>, DatasetProvenance) {
    (
        RdfLookaside::default(),
        Arc::new(ContentStore::new()),
        DatasetProvenance::new(),
    )
}

/// The emitted container bytes — what actually ships. `identity` rather than
/// `zstd` so compression is neither inside a measurement nor between a byte
/// comparison and the tables it is really about.
pub fn emitted(builder: &SnapshotBuilder) -> Vec<u8> {
    emit_gts(
        builder,
        "dist",
        Some(vec!["identity".to_owned()]),
        Vec::new(),
        Vec::new(),
        None,
        None,
        None,
        DEFAULT_RSYNCABLE_THRESHOLD,
        &MediumPlan::undicted(None),
    )
    .expect("the keystone fixture emits")
}
