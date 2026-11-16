// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The frozen dataset's content identity: its value is a persisted law.

use std::sync::Arc;

use purrdf_core::{
    BlankScope, CompositeDatasetView, DatasetMut, DatasetView, MutableDataset, QuadValues,
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

fn identity(dataset: &RdfDataset) -> String {
    DatasetView::snapshot_identity(dataset)
        .unwrap()
        .expect("a frozen dataset has an identity")
        .to_string()
}

/// Both freezes produce the same, frozen identity value: the resident freeze
/// reads every row, the admitted freeze reads its eager statistics' records.
#[test]
fn the_content_identity_value_is_frozen_for_both_freezes() {
    let resident: Arc<RdfDataset> = builder().freeze().unwrap();
    let mut admission = Resident;
    let admitted = builder();
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

fn iri(local: &str) -> TermValue {
    TermValue::Iri(format!("{EX}{local}"))
}

/// The composite's identity, which binds its sources' identities and how it
/// composes them, is a persisted law too.
#[test]
fn the_composite_identity_value_is_frozen() {
    let composite = CompositeDatasetView::new(
        vec![builder().freeze().unwrap(), other()],
        ViewLimits::default(),
    )
    .unwrap();
    let identity = DatasetView::snapshot_identity(&composite)
        .unwrap()
        .expect("a composite has an identity")
        .to_string();
    assert_eq!(identity, COMPOSITE_GOLDEN);
}

/// The mutation snapshot's identity binds the base, the delta and the
/// suppressed rows.
#[test]
fn the_delta_identity_value_is_frozen() {
    let mut mutation = MutableDataset::new(builder().freeze().unwrap());
    assert!(mutation.remove(&QuadValues {
        s: iri("alice"),
        p: iri("name"),
        o: TermValue::simple_literal("Alice"),
        g: None,
    }));
    assert!(
        mutation
            .insert(QuadValues {
                s: iri("carol"),
                p: iri("knows"),
                o: iri("alice"),
                g: None,
            })
            .unwrap()
    );
    let delta = mutation.snapshot_view().unwrap();
    let identity = DatasetView::snapshot_identity(&delta)
        .unwrap()
        .expect("a mutation snapshot has an identity")
        .to_string();
    assert_eq!(identity, DELTA_GOLDEN);
}

const COMPOSITE_GOLDEN: &str = "34840a0e7c474035c77265a5de3deef0b4f3484cb4e18f3333d438daba732c7b";
const DELTA_GOLDEN: &str = "a658f81754da777a81137e8c8665035993e77b75634a2f5dac322f3b2500e9e4";
