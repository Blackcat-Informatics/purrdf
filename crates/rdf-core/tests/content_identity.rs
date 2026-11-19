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
    fixture(false)
}

/// The fixture with its rows pushed in the given order. The dictionary, whose
/// layout the identity also binds, is interned in one order either way.
fn fixture(reverse: bool) -> RdfDatasetBuilder {
    let mut b = RdfDatasetBuilder::new();
    let locals = [
        "alice", "bob", "knows", "name", "age", "g", "empty", "source",
    ];
    let mut ids = std::collections::BTreeMap::new();
    for local in locals {
        ids.insert(local, b.intern_iri(&format!("{EX}{local}")).unwrap());
    }
    let iri = |local: &str| ids[local];
    let (alice, bob, knows, name, age) = (
        iri("alice"),
        iri("bob"),
        iri("knows"),
        iri("name"),
        iri("age"),
    );
    let (graph, empty, source) = (iri("g"), iri("empty"), iri("source"));
    let simple = b.intern_literal(RdfLiteral::simple("Alice")).unwrap();
    let tagged = b
        .intern_literal(RdfLiteral::language_tagged("Robert", "en"))
        .unwrap();
    let typed = b
        .intern_literal(RdfLiteral::typed("42", XSD_INTEGER))
        .unwrap();
    let blank = b.intern_blank("b0", BlankScope::DEFAULT).unwrap();
    let mut quads = [
        (alice, knows, bob, None),
        (alice, name, simple, None),
        (bob, name, tagged, Some(graph)),
        (blank, age, typed, Some(graph)),
    ];
    if reverse {
        quads.reverse();
    }
    for (s, p, o, g) in quads {
        b.push_quad(s, p, o, g).unwrap();
    }
    b.declare_named_graph(empty).unwrap();
    let statement = b.intern_triple(alice, knows, bob).unwrap();
    let reifier = b.intern_iri(&format!("{EX}claim")).unwrap();
    b.push_reifier(reifier, statement).unwrap();
    b.push_annotation(reifier, source, bob).unwrap();
    b
}

/// The identity this fixture had when the identity law was last changed. A
/// change here changes every persisted identity of every dataset. The current
/// values are those of the 5.0 law, in which each row family is BLAKE3 over its
/// sorted row digests in place of an additive sum.
const GOLDEN: &str = "7c7487f99acbb86f921684480bbc792514ba6a10e922bb8d67f46f1d2e837f5b";
const COMPOSITE_GOLDEN: &str = "ae6b179290d2324b6953e13f9e08398632bc137c0380db1eac80ec5919575704";
const DELTA_GOLDEN: &str = "38b4a88d90b623cb09b5c2468ab1a116d38270dac720c1d7ec0234d802703d12";

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

/// The identity is a function of the row set, not of the order the rows were
/// pushed in: the same rows pushed in reverse, frozen either way, keep the
/// golden value.
#[test]
fn the_identity_of_equal_rows_does_not_depend_on_their_order() {
    let resident: Arc<RdfDataset> = fixture(true).freeze().unwrap();
    let mut admission = Resident;
    let admitted = fixture(true);
    let admitted_bytes = admitted.native_buffer_bytes().unwrap();
    let admitted = admitted
        .freeze_with_memory(&mut Memory::resume(&mut admission, admitted_bytes))
        .unwrap();
    assert_eq!(identity(&resident), GOLDEN);
    assert_eq!(identity(&admitted), GOLDEN);
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

    // The same mutation in the other order, over a base whose rows were pushed
    // in reverse, holds the same rows: its identity is a function of them, not
    // of the edit history.
    let base = fixture(true).freeze().unwrap();
    let mut mutation = MutableDataset::new(Arc::clone(&base));
    let alice = TermValue::Iri(format!("{EX}alice"));
    assert!(mutation.insert(quad("carol", "knows", alice)).unwrap());
    assert!(mutation.remove(&quad("alice", "name", TermValue::simple_literal("Alice"))));
    assert_eq!(identity(&mutation.snapshot_view().unwrap()), DELTA_GOLDEN);
}
