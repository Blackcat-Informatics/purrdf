// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Publication must avoid every identity retained by a mutable destination.

use std::collections::BTreeSet;
use std::ops::ControlFlow;

use purrdf_core::{
    BlankScope, DatasetMut, MutableDataset, QuadValues, RdfDatasetBuilder, RdfLiteral, TermBox,
    TermValue,
};

const EX: &str = "http://example.org/";
const LIST: &str = "http://w3id.org/awslabs/neptune/SPARQL-CDTs/List";

#[test]
fn inventory_keeps_suppressed_and_nested_base_and_delta_identities() {
    let mut builder = RdfDatasetBuilder::new();
    let subject = builder.intern_blank("base", BlankScope(5));
    let predicate = builder.intern_iri(&format!("{EX}p"));
    let suppressed = builder.intern_blank("suppressed", BlankScope::DEFAULT);
    let graph = builder.intern_blank("base-graph", BlankScope(9));
    builder.push_quad(subject, predicate, suppressed, Some(graph));
    let nested_subject = builder.intern_blank("base-nested", BlankScope(7));
    let nested_object = builder.intern_iri(&format!("{EX}o"));
    let nested = builder.intern_triple(nested_subject, predicate, nested_object);
    builder.push_quad(subject, predicate, nested, None);
    let composite = builder.intern_literal(RdfLiteral::typed("[_:base-composite]", LIST));
    builder.push_quad(subject, predicate, composite, None);
    let base = builder.freeze().expect("base identity inventory");
    let suppressed_quad = QuadValues::quad(
        base.term_value(subject),
        base.term_value(predicate),
        base.term_value(suppressed),
        base.term_value(graph),
    );
    let mut mutable = MutableDataset::new(base);
    assert!(mutable.remove(&suppressed_quad));

    let scoped_composite = BlankScope(17).qualify_label("delta-scoped-composite");
    let delta = QuadValues::quad(
        TermValue::Blank {
            label: "delta".into(),
            scope: BlankScope(11),
        },
        TermValue::iri(format!("{EX}p")),
        TermValue::Triple {
            s: TermBox::new(TermValue::blank("delta-nested")),
            p: TermBox::new(TermValue::iri(format!("{EX}p"))),
            o: TermBox::new(TermValue::typed_literal(
                format!("[_:delta-composite, [_:{scoped_composite}]]"),
                LIST,
            )),
        },
        TermValue::blank("delta-graph"),
    );
    assert!(mutable.insert(delta.clone()).expect("nested delta insert"));
    assert!(mutable.remove(&delta));

    let mut seen = BTreeSet::new();
    let complete: ControlFlow<()> = mutable.visit_blank_identities(|label, scope| {
        seen.insert((label.to_owned(), scope));
        ControlFlow::Continue(())
    });
    assert_eq!(complete, ControlFlow::Continue(()));
    let expected: BTreeSet<_> = [
        ("base", BlankScope(5)),
        ("suppressed", BlankScope::DEFAULT),
        ("base-graph", BlankScope(9)),
        ("base-nested", BlankScope(7)),
        ("base-composite", BlankScope::DEFAULT),
        ("delta", BlankScope(11)),
        ("delta-nested", BlankScope::DEFAULT),
        ("delta-composite", BlankScope::DEFAULT),
        ("delta-scoped-composite", BlankScope(17)),
        ("delta-graph", BlankScope::DEFAULT),
    ]
    .into_iter()
    .map(|(label, scope)| (label.to_owned(), scope))
    .collect();
    assert_eq!(
        seen, expected,
        "inventory includes removed, still-owned terms"
    );
}

#[test]
fn visitor_stops_at_the_first_break_in_either_storage_layer() {
    for delta_only in [false, true] {
        let mut builder = RdfDatasetBuilder::new();
        if !delta_only {
            let subject = builder.intern_blank("first", BlankScope(3));
            let predicate = builder.intern_iri(&format!("{EX}p"));
            let object = builder.intern_blank("second", BlankScope::DEFAULT);
            builder.push_quad(subject, predicate, object, None);
        }
        let mut mutable = MutableDataset::new(builder.freeze().expect("visitor base"));
        if delta_only {
            mutable
                .insert(QuadValues::triple(
                    TermValue::iri(format!("{EX}s")),
                    TermValue::iri(format!("{EX}p")),
                    TermValue::typed_literal("[_:first, _:second]", LIST),
                ))
                .expect("delta composite");
        }
        let mut calls = 0;
        let result = mutable.visit_blank_identities(|label, scope| {
            calls += 1;
            ControlFlow::Break((label.to_owned(), scope))
        });
        assert_eq!(calls, 1, "Break stops further identities");
        assert_eq!(
            result,
            ControlFlow::Break((
                "first".to_owned(),
                if delta_only {
                    BlankScope::DEFAULT
                } else {
                    BlankScope(3)
                },
            ))
        );
    }
    let empty = MutableDataset::new(RdfDatasetBuilder::new().freeze().expect("empty base"));
    let result: ControlFlow<()> = empty.visit_blank_identities(|_, _| panic!("no identity"));
    assert_eq!(result, ControlFlow::Continue(()));
}

/// The fresh-blank namespace law every mutable-destination allocator applies
/// through [`MutableDataset::visit_blank_identities`] (SPARQL UPDATE's `INSERT DATA`,
/// `INSERT … WHERE`, and LOAD-then-INSERT): with no default-scope identity in the
/// destination the minter keeps its ordinary spelling, otherwise it takes the first
/// `append{n}_` namespace no default-scope identity starts with.
fn mint_namespace(destination: &MutableDataset) -> Option<String> {
    let occupied = |prefix: &str| {
        destination
            .visit_blank_identities(|label, scope| {
                if scope == BlankScope::DEFAULT && label.starts_with(prefix) {
                    ControlFlow::Break(())
                } else {
                    ControlFlow::Continue(())
                }
            })
            .is_break()
    };
    if !occupied("") {
        return None;
    }
    (0_u64..)
        .map(|ordinal| format!("append{ordinal}_"))
        .find(|candidate| !occupied(candidate))
}

/// The label an allocator mints for its first fresh blank (`c1` under the
/// namespace, exactly as an update template's first blank is spelled).
fn first_fresh_label(destination: &MutableDataset) -> String {
    format!("{}c1", mint_namespace(destination).unwrap_or_default())
}

#[test]
fn a_declared_blank_graph_name_is_a_retained_identity_fresh_blanks_avoid() {
    let mut mutable = MutableDataset::new(RdfDatasetBuilder::new().freeze().expect("empty base"));
    let declared = TermValue::blank("c1");
    assert_eq!(mutable.declare_named_graph(declared.clone()), Ok(true));
    // The declaration owns no row, yet the dataset retains it as a graph.
    assert_eq!(mutable.declared_named_graphs().count(), 1);

    let fresh = first_fresh_label(&mutable);
    assert_ne!(
        TermValue::blank(fresh.clone()),
        declared,
        "a fresh blank must not merge with the declared graph name it would collide with"
    );
    assert_eq!(fresh, "append0_c1");

    // Inserting the fresh blank keeps two distinct identities: a row whose subject is
    // the fresh blank, and the still-empty declared graph.
    mutable
        .insert(QuadValues::triple(
            TermValue::blank(fresh),
            TermValue::iri(format!("{EX}p")),
            TermValue::iri(format!("{EX}o")),
        ))
        .expect("fresh row");
    let published = mutable.freeze().expect("fresh row with a declared graph");
    assert_eq!(published.quad_count(), 1);
    assert_eq!(published.named_graphs().count(), 1);
    let subjects: Vec<_> = published
        .quads()
        .map(|quad| published.term_value(quad.s))
        .collect();
    let graphs: Vec<_> = published
        .named_graphs()
        .map(|g| published.term_value(g))
        .collect();
    assert_eq!(graphs, [declared]);
    assert_ne!(subjects, graphs, "two identities, never one merged node");
}

#[test]
fn declarations_add_only_their_blank_names_to_the_base_and_delta_visit() {
    let mut builder = RdfDatasetBuilder::new();
    let subject = builder.intern_blank("base", BlankScope(5));
    let predicate = builder.intern_iri(&format!("{EX}p"));
    let object = builder.intern_blank("object", BlankScope::DEFAULT);
    builder.push_quad(subject, predicate, object, None);
    let mut mutable = MutableDataset::new(builder.freeze().expect("visitor base"));
    mutable
        .insert(QuadValues::quad(
            TermValue::blank("delta"),
            TermValue::iri(format!("{EX}p")),
            TermValue::iri(format!("{EX}o")),
            TermValue::blank("delta-graph"),
        ))
        .expect("delta row");
    let visit = |mutable: &MutableDataset| {
        let mut seen = Vec::new();
        let complete: ControlFlow<()> = mutable.visit_blank_identities(|label, scope| {
            seen.push((label.to_owned(), scope));
            ControlFlow::Continue(())
        });
        assert_eq!(complete, ControlFlow::Continue(()));
        seen
    };
    let before = visit(&mutable);
    let expected: Vec<_> = [
        ("base", BlankScope(5)),
        ("object", BlankScope::DEFAULT),
        ("delta", BlankScope::DEFAULT),
        ("delta-graph", BlankScope::DEFAULT),
    ]
    .into_iter()
    .map(|(label, scope)| (label.to_owned(), scope))
    .collect();
    assert_eq!(before, expected, "base terms, then delta values, in order");
    // A base or delta blank still answers the allocator exactly as before.
    assert_eq!(first_fresh_label(&mutable), "append0_c1");

    // An IRI-named declaration adds no identity.
    assert_eq!(
        mutable.declare_named_graph(TermValue::iri(format!("{EX}empty"))),
        Ok(true)
    );
    assert_eq!(visit(&mutable), before);

    // A blank-named declaration adds exactly its own name, after the delta, in its
    // own scope.
    let scoped = TermValue::Blank {
        label: "declared".into(),
        scope: BlankScope(9),
    };
    assert_eq!(mutable.declare_named_graph(scoped), Ok(true));
    let mut extended = before;
    extended.push(("declared".to_owned(), BlankScope(9)));
    assert_eq!(visit(&mutable), extended);
    assert_eq!(first_fresh_label(&mutable), "append0_c1");
}
