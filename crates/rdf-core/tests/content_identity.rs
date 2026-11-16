// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The content identity: its value is a persisted law, and nothing computes it
//! until a caller asks for it.

use std::sync::Arc;

use purrdf_core::{
    BlankScope, CompositeDatasetView, ContentIdentified, DatasetMut, MutableDataset, QuadValues,
    RdfDataset, RdfDatasetBuilder, RdfLiteral, TermValue, ViewLimits,
};
use purrdf_lex::allocation::{Memory, Resident};
use purrdf_xsd::datatype::XSD_INTEGER;

const EX: &str = "http://example.org/";

/// Every row kind the identity binds: default and named graph quads, a declared
/// empty named graph, simple, typed and language-tagged literals, a blank node, a
/// triple term, a reifier and an annotation on it.
fn builder() -> RdfDatasetBuilder {
    let mut b = RdfDatasetBuilder::new();
    let iri =
        |b: &mut RdfDatasetBuilder, local: &str| b.intern_iri(&format!("{EX}{local}")).unwrap();
    let (alice, bob, knows, name, age) = (
        iri(&mut b, "alice"),
        iri(&mut b, "bob"),
        iri(&mut b, "knows"),
        iri(&mut b, "name"),
        iri(&mut b, "age"),
    );
    let (graph, empty, source) = (
        iri(&mut b, "g"),
        iri(&mut b, "empty"),
        iri(&mut b, "source"),
    );
    let simple = b.intern_literal(RdfLiteral::simple("Alice")).unwrap();
    let tagged = b
        .intern_literal(RdfLiteral::language_tagged("Robert", "en"))
        .unwrap();
    let typed = b
        .intern_literal(RdfLiteral::typed("42", XSD_INTEGER))
        .unwrap();
    let blank = b.intern_blank("b0", BlankScope::DEFAULT).unwrap();
    b.push_quad(alice, knows, bob, None).unwrap();
    b.push_quad(alice, name, simple, None).unwrap();
    b.push_quad(bob, name, tagged, Some(graph)).unwrap();
    b.push_quad(blank, age, typed, Some(graph)).unwrap();
    b.declare_named_graph(empty).unwrap();
    let statement = b.intern_triple(alice, knows, bob).unwrap();
    let reifier = iri(&mut b, "claim");
    b.push_reifier(reifier, statement).unwrap();
    b.push_annotation(reifier, source, bob).unwrap();
    b
}

/// The identity this fixture had when the identity law was last changed. A
/// change here changes every persisted identity of every dataset.
const GOLDEN: &str = "23989607954605f059ee12b49ab3b2a45f55a6b4971cc56eeb05c4b99ec6739c";
const COMPOSITE_GOLDEN: &str = "34840a0e7c474035c77265a5de3deef0b4f3484cb4e18f3333d438daba732c7b";
const DELTA_GOLDEN: &str = "a658f81754da777a81137e8c8665035993e77b75634a2f5dac322f3b2500e9e4";

/// Every identity is sealed behind a once-lock the first time it is asked for, so
/// an unset lock in a carrier's `Debug` is exactly "no identity was computed".
const UNSEALED: &str = "snapshot_identity: OnceLock(<uninit>)";

fn sealed(carrier: &impl std::fmt::Debug) -> bool {
    !format!("{carrier:?}").contains(UNSEALED)
}

fn identity<V: ContentIdentified>(view: &V) -> String
where
    V::IdentityError: std::fmt::Debug,
{
    view.content_identity().unwrap().to_string()
}

/// Neither freeze computes the identity; asked for, both produce the same, frozen
/// value: the resident freeze reads every row, the admitted freeze its eager
/// statistics' records.
#[test]
fn both_freezes_leave_the_identity_unsealed_and_compute_the_frozen_value_on_request() {
    let resident: Arc<RdfDataset> = builder().freeze().unwrap();
    let mut admission = Resident;
    let admitted = builder();
    let admitted_bytes = admitted.native_buffer_bytes().unwrap();
    let admitted = admitted
        .freeze_with_memory(&mut Memory::resume(&mut admission, admitted_bytes))
        .unwrap();
    assert!(!sealed(&resident) && !sealed(&admitted));
    assert_eq!(identity(&resident), GOLDEN);
    assert_eq!(identity(&admitted), GOLDEN);
    assert!(sealed(&resident) && sealed(&admitted));
}

/// One more quad in its own graph, for the composed views.
fn other() -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let s = b.intern_iri(&format!("{EX}carol")).unwrap();
    let p = b.intern_iri(&format!("{EX}knows")).unwrap();
    let g = b.intern_iri(&format!("{EX}h")).unwrap();
    b.push_quad(s, p, s, Some(g)).unwrap();
    b.freeze().unwrap()
}

/// A default-graph quad over `example.org` names.
fn quad(subject: &str, predicate: &str, object: TermValue) -> QuadValues {
    QuadValues {
        s: TermValue::Iri(format!("{EX}{subject}")),
        p: TermValue::Iri(format!("{EX}{predicate}")),
        o: object,
        g: None,
    }
}

/// Building a composite hashes neither it nor its sources; its identity, which
/// binds its sources' identities and how it composes them, is computed on the
/// first request, shared by clones, and keeps its frozen value.
#[test]
fn a_composite_hashes_nothing_until_its_identity_is_asked_for() {
    let base = builder().freeze().unwrap();
    let composite =
        CompositeDatasetView::new(vec![Arc::clone(&base), other()], ViewLimits::default()).unwrap();
    let clone = composite.clone();
    assert!(!sealed(&composite) && !sealed(&base));
    assert_eq!(identity(&clone), COMPOSITE_GOLDEN);
    assert!(sealed(&composite) && sealed(&base));
    assert_eq!(identity(&composite), COMPOSITE_GOLDEN);
}

/// A mutation snapshot hashes nothing at construction; its identity binds the
/// base, the delta and the suppressed rows when asked for.
#[test]
fn a_mutation_snapshot_hashes_nothing_until_its_identity_is_asked_for() {
    let base = builder().freeze().unwrap();
    let mut mutation = MutableDataset::new(Arc::clone(&base));
    assert!(mutation.remove(&quad("alice", "name", TermValue::simple_literal("Alice"))));
    let alice = TermValue::Iri(format!("{EX}alice"));
    assert!(mutation.insert(quad("carol", "knows", alice)).unwrap());
    let delta = mutation.snapshot_view().unwrap();
    assert!(!sealed(&delta) && !sealed(&base));
    assert_eq!(identity(&delta), DELTA_GOLDEN);
    assert!(sealed(&delta) && sealed(&base));
}
