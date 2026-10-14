// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Named-graph presence and capabilities agree before and after publication.

use std::sync::Arc;

use purrdf_core::term_fixture::{empty_dataset, intern_value, iri, triple_chain};
use purrdf_core::{
    BlankScope, DatasetMut, DatasetView, GraphExistenceMode, GraphMatchValue, MutableDataset,
    QuadValues, RdfDataset, RdfDatasetBuilder, TermValue,
};

const MODES: [GraphExistenceMode; 2] = [
    GraphExistenceMode::Implicit,
    GraphExistenceMode::RememberEmpty,
];

fn graph_names<D: DatasetView>(dataset: &D) -> Vec<TermValue> {
    let mut names: Vec<_> = dataset
        .named_graphs()
        .map(|id| dataset.term_value(id).expect("graph name resolves"))
        .collect();
    names.sort();
    assert_eq!(dataset.capabilities().named_graphs, !names.is_empty());
    names
}

fn published_names(mutable: &MutableDataset) -> Vec<TermValue> {
    let snapshot = mutable.snapshot_view().expect("snapshot publishes");
    let names = graph_names(&snapshot);
    let frozen = mutable.freeze().expect("freeze publishes");
    assert_eq!(names, graph_names(&*frozen));
    for name in &names {
        assert!(mutable.has_named_graph(name));
        let id = snapshot
            .term_id_by_value(name)
            .expect("snapshot read succeeds")
            .expect("present graph resolves");
        assert!(snapshot.has_named_graph(id));
    }
    names
}

fn named_row(graph: TermValue) -> QuadValues {
    QuadValues::quad(iri("s"), iri("p"), iri("o"), graph)
}

#[test]
fn omitted_policy_remembers_empty_slots_across_mutation_and_reconstruction() {
    assert_eq!(
        GraphExistenceMode::default(),
        GraphExistenceMode::RememberEmpty
    );
    let base = empty_dataset();
    let mut mutable = MutableDataset::new(Arc::clone(&base));
    let graph = iri("default-lifetime");
    mutable
        .create_named_graph(graph.clone())
        .expect("creates slot");
    let retained = mutable.snapshot_view().expect("retained empty snapshot");
    let row = named_row(graph.clone());
    mutable.insert(row.clone()).expect("inserts row");
    assert!(mutable.remove(&row));
    assert_eq!(published_names(&mutable), std::slice::from_ref(&graph));
    assert_eq!(graph_names(&retained), std::slice::from_ref(&graph));
    let mut rebuilt = MutableDataset::new(mutable.freeze().expect("freezes"));
    assert_eq!(rebuilt.graph_existence(), GraphExistenceMode::RememberEmpty);
    assert_eq!(published_names(&rebuilt), std::slice::from_ref(&graph));
    rebuilt.withdraw_graph_declaration(&graph);
    assert_eq!(published_names(&rebuilt), [] as [TermValue; 0]);
    assert_eq!(graph_names(&*base), [] as [TermValue; 0]);
    let mut implicit =
        MutableDataset::new_with_graph_existence(empty_dataset(), GraphExistenceMode::Implicit);
    implicit
        .insert(row.clone())
        .expect("implicit valid neighbor");
    assert!(implicit.remove(&row));
    assert_eq!(published_names(&implicit), [] as [TermValue; 0]);
}

#[test]
fn an_empty_graph_declaration_sets_the_frozen_capability() {
    let mut builder = RdfDatasetBuilder::new();
    let graph = builder.intern_iri("http://example.org/empty");
    builder.declare_named_graph(graph);
    let dataset = builder.freeze().expect("valid declaration");
    assert_eq!(dataset.named_graphs().count(), 1);
    assert!(dataset.capabilities().named_graphs);
}

#[test]
fn withdrawing_the_only_named_graph_clears_the_snapshot_capability() {
    let mut builder = RdfDatasetBuilder::new();
    let subject = builder.intern_iri("http://example.org/s");
    let predicate = builder.intern_iri("http://example.org/p");
    let object = builder.intern_iri("http://example.org/o");
    let graph = builder.intern_iri("http://example.org/g");
    builder.push_quad(subject, predicate, object, Some(graph));
    let base = builder.freeze().expect("valid row");
    let mut mutable = MutableDataset::new(Arc::clone(&base));
    let rows = mutable.quads_for_pattern(None, None, None, GraphMatchValue::Any);
    for row in rows {
        assert!(mutable.remove(&row));
    }
    mutable.withdraw_graph_declaration(&iri("g"));
    let snapshot = mutable.snapshot_view().expect("empty snapshot");
    assert_eq!(snapshot.named_graphs().count(), 0);
    assert!(!snapshot.capabilities().named_graphs);
    assert!(base.capabilities().named_graphs);
}

#[test]
fn mode_selection_is_local_and_default_construction_remembers_empty_graphs() {
    let base = empty_dataset();
    let mut remembered = MutableDataset::new_with_graph_existence(
        Arc::clone(&base),
        GraphExistenceMode::RememberEmpty,
    );
    let implicit =
        MutableDataset::new_with_graph_existence(Arc::clone(&base), GraphExistenceMode::Implicit);
    assert_eq!(
        GraphExistenceMode::default(),
        GraphExistenceMode::RememberEmpty
    );
    assert_eq!(implicit.graph_existence(), GraphExistenceMode::Implicit);
    assert_eq!(
        remembered.graph_existence(),
        GraphExistenceMode::RememberEmpty
    );
    remembered.create_named_graph(iri("g")).expect("fresh slot");
    assert_eq!(published_names(&remembered), [iri("g")]);
    assert_eq!(published_names(&implicit), [] as [TermValue; 0]);
    assert_eq!(graph_names(&*base), [] as [TermValue; 0]);
    let new_default_branch = MutableDataset::new(remembered.freeze().expect("freezes"));
    assert_eq!(
        new_default_branch.graph_existence(),
        GraphExistenceMode::RememberEmpty
    );
    assert_eq!(published_names(&new_default_branch), [iri("g")]);
}

#[test]
fn create_obeys_mode_and_explicit_declaration_remains_idempotent() {
    for mode in MODES {
        for graph in [iri("g"), TermValue::blank("g")] {
            let mut mutable = MutableDataset::new_with_graph_existence(empty_dataset(), mode);
            let remembers = mode == GraphExistenceMode::RememberEmpty;
            assert_eq!(
                mutable.create_named_graph(graph.clone()).unwrap(),
                remembers
            );
            assert_eq!(mutable.has_named_graph(&graph), remembers);
            if remembers {
                assert_eq!(
                    mutable.create_named_graph(graph.clone()).unwrap_err().code,
                    "rdf-ir-graph-already-exists"
                );
                assert!(!mutable.declare_named_graph(graph.clone()).unwrap());
            } else {
                assert!(!mutable.create_named_graph(graph.clone()).unwrap());
                assert!(mutable.declare_named_graph(graph.clone()).unwrap());
            }
            assert!(!mutable.declare_named_graph(graph.clone()).unwrap());
            assert_eq!(
                published_names(&mutable).as_slice(),
                std::slice::from_ref(&graph)
            );
            mutable.withdraw_graph_declaration(&graph);
            assert!(!mutable.has_named_graph(&graph));
            assert_eq!(published_names(&mutable), [] as [TermValue; 0]);
            assert_eq!(
                mutable.create_named_graph(graph.clone()).unwrap(),
                remembers
            );
            assert_eq!(mutable.has_named_graph(&graph), remembers);
        }
    }
}

#[test]
fn removing_the_last_added_row_preserves_only_remembered_slots() {
    for mode in MODES {
        for graph in [iri("g"), TermValue::blank("g")] {
            let mut mutable = MutableDataset::new_with_graph_existence(empty_dataset(), mode);
            let row = named_row(graph.clone());
            assert!(mutable.insert(row.clone()).unwrap());
            assert!(!mutable.insert(row.clone()).unwrap());
            assert!(mutable.has_named_graph(&graph));
            assert_eq!(
                published_names(&mutable).as_slice(),
                std::slice::from_ref(&graph)
            );
            let retained = mutable.snapshot_view().unwrap();
            assert!(mutable.remove(&row));
            assert!(!mutable.remove(&row));
            let remembers = mode == GraphExistenceMode::RememberEmpty;
            assert_eq!(mutable.has_named_graph(&graph), remembers);
            assert_eq!(published_names(&mutable).len(), usize::from(remembers));
            assert_eq!(
                graph_names(&retained).as_slice(),
                std::slice::from_ref(&graph)
            );
            assert!(mutable.insert(row.clone()).unwrap());
            assert_eq!(
                published_names(&mutable).as_slice(),
                std::slice::from_ref(&graph)
            );
            assert!(mutable.remove(&row));
            mutable.withdraw_graph_declaration(&graph);
            assert_eq!(published_names(&mutable), [] as [TermValue; 0]);
        }
    }
}

#[test]
fn a_graph_name_already_interned_as_an_object_gets_its_own_slot() {
    let mut builder = RdfDatasetBuilder::new();
    let subject = builder.intern_iri("http://example.org/default");
    let predicate = builder.intern_iri("http://example.org/p");
    let graph = builder.intern_iri("http://example.org/g");
    builder.push_quad(subject, predicate, graph, None);
    let base = builder.freeze().unwrap();
    assert!(base.named_graphs().next().is_none());
    for mode in MODES {
        let mut mutable = MutableDataset::new_with_graph_existence(Arc::clone(&base), mode);
        let row = named_row(iri("g"));
        assert!(mutable.insert(row.clone()).unwrap());
        assert_eq!(published_names(&mutable), [iri("g")]);
        assert!(mutable.remove(&row));
        assert_eq!(
            published_names(&mutable).len(),
            usize::from(mode == GraphExistenceMode::RememberEmpty)
        );
        assert_eq!(mutable.effective_count(), 1, "default row remains");
    }
}

fn role_base(graph: &TermValue, role: u8) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let subject = builder.intern_iri("http://example.org/s");
    let predicate = builder.intern_iri("http://example.org/p");
    let object = builder.intern_iri("http://example.org/o");
    let graph = intern_value(&mut builder, graph);
    let reifier = builder.intern_iri("http://example.org/reifier");
    match role {
        0 => builder.push_quad(subject, predicate, object, Some(graph)),
        1 => {
            let triple = builder.intern_triple(subject, predicate, object);
            builder.push_reifier_in_graph(reifier, triple, Some(graph));
        }
        2 => {
            let triple = builder.intern_triple(subject, predicate, object);
            builder.push_reifier(reifier, triple);
            builder.push_annotation_in_graph(reifier, predicate, object, Some(graph));
        }
        _ => unreachable!("the fixture has exactly three RDF roles"),
    }
    builder.freeze().expect("typed RDF role fixture")
}

#[test]
fn base_ordinary_reifier_and_annotation_rows_each_keep_remembered_slots() {
    for mode in MODES {
        for graph in [iri("g"), TermValue::blank("g")] {
            for role in 0..3 {
                let base = role_base(&graph, role);
                let mut mutable = MutableDataset::new_with_graph_existence(Arc::clone(&base), mode);
                let rows =
                    mutable.quads_for_pattern(None, None, None, GraphMatchValue::Named(&graph));
                assert_eq!(rows.len(), 1, "each named graph has only the tested role");
                if mode == GraphExistenceMode::RememberEmpty {
                    assert_eq!(
                        mutable.create_named_graph(graph.clone()).unwrap_err().code,
                        "rdf-ir-graph-already-exists"
                    );
                } else {
                    assert!(!mutable.create_named_graph(graph.clone()).unwrap());
                }
                assert!(mutable.remove(&rows[0]));
                let remembers = mode == GraphExistenceMode::RememberEmpty;
                assert_eq!(mutable.has_named_graph(&graph), remembers);
                assert_eq!(published_names(&mutable).len(), usize::from(remembers));
                assert_eq!(
                    graph_names(&*base).as_slice(),
                    std::slice::from_ref(&graph),
                    "base is immutable"
                );
                mutable.withdraw_graph_declaration(&graph);
                assert_eq!(published_names(&mutable), [] as [TermValue; 0]);
                assert!(mutable.insert(rows[0].clone()).unwrap());
                assert_eq!(
                    published_names(&mutable).as_slice(),
                    std::slice::from_ref(&graph)
                );
            }
        }
    }
}

#[test]
fn omitted_policy_retains_last_physical_row_graph_for_every_role() {
    for role in 0..3 {
        let graph = iri("role-default");
        let base = role_base(&graph, role);
        let mut mutable = MutableDataset::new(Arc::clone(&base));
        for row in mutable.quads_for_pattern(None, None, None, GraphMatchValue::Any) {
            assert!(mutable.remove(&row));
        }
        assert_eq!(published_names(&mutable), std::slice::from_ref(&graph));
        let retained = mutable
            .snapshot_view()
            .expect("retained empty role snapshot");
        mutable.withdraw_graph_declaration(&graph);
        assert_eq!(published_names(&mutable), [] as [TermValue; 0]);
        assert_eq!(graph_names(&retained), [graph]);
        assert!(base.capabilities().named_graphs);
    }
}

#[test]
fn declarations_stay_distinct_across_blank_scopes_and_bulk_withdrawal() {
    for mode in MODES {
        let names = [1, 2].map(|scope| TermValue::Blank {
            label: "same".into(),
            scope: BlankScope(scope),
        });
        let mut mutable = MutableDataset::new_with_graph_existence(empty_dataset(), mode);
        for name in &names {
            assert!(mutable.declare_named_graph(name.clone()).unwrap());
        }
        assert_eq!(published_names(&mutable), names);
        let snapshot = mutable.snapshot_view().unwrap();
        let mut rebased = MutableDataset::new_with_graph_existence(mutable.freeze().unwrap(), mode);
        assert_eq!(published_names(&rebased), names);
        rebased.withdraw_named_graph_declarations();
        mutable.withdraw_named_graph_declarations();
        assert_eq!(published_names(&rebased), [] as [TermValue; 0]);
        assert_eq!(published_names(&mutable), [] as [TermValue; 0]);
        assert_eq!(graph_names(&snapshot), names);
    }
}

#[test]
fn graph_ingress_refusals_do_not_create_a_slot_and_valid_neighbors_succeed() {
    for mode in MODES {
        let mut mutable = MutableDataset::new_with_graph_existence(empty_dataset(), mode);
        for graph in [TermValue::simple_literal("bad"), triple_chain(1)] {
            assert_eq!(
                mutable.create_named_graph(graph.clone()).unwrap_err().code,
                "rdf-ir-graph-name-invalid"
            );
            assert_eq!(
                mutable.declare_named_graph(graph).unwrap_err().code,
                "rdf-ir-graph-name-invalid"
            );
        }
        assert!(
            mutable
                .create_named_graph(TermValue::Iri("relative".into()))
                .is_err()
        );
        assert_eq!(published_names(&mutable), [] as [TermValue; 0]);
        assert_eq!(mutable.effective_count(), 0);
        assert!(mutable.declare_named_graph(iri("valid")).unwrap());
        assert_eq!(published_names(&mutable), [iri("valid")]);
        if mode == GraphExistenceMode::RememberEmpty {
            assert!(mutable.create_named_graph(iri("neighbor")).unwrap());
            assert_eq!(published_names(&mutable), [iri("neighbor"), iri("valid")]);
        }
    }
}

/// Withdraw a populated graph's declaration, then remove its last row. The answer
/// may depend on the mode, never on whether the declaration came from the base or
/// from this mutable dataset: an intermediate freeze and rebranch is invisible.
#[test]
fn withdrawing_a_populated_graph_answers_the_same_across_a_freeze() {
    for mode in MODES {
        for bulk in [false, true] {
            let mut images = Vec::new();
            for refreeze in [false, true] {
                let mut mutable = MutableDataset::new_with_graph_existence(empty_dataset(), mode);
                assert!(mutable.insert(named_row(iri("g"))).expect("absolute row"));
                if refreeze {
                    let frozen = mutable.freeze().expect("freezes");
                    mutable = MutableDataset::new_with_graph_existence(frozen, mode);
                }
                if bulk {
                    mutable.withdraw_named_graph_declarations();
                } else {
                    mutable.withdraw_graph_declaration(&iri("g"));
                }
                assert_eq!(
                    published_names(&mutable),
                    [iri("g")],
                    "rows keep it present"
                );
                assert!(mutable.remove(&named_row(iri("g"))));
                images.push(published_names(&mutable));
            }
            let expected = if mode == GraphExistenceMode::RememberEmpty {
                vec![iri("g")]
            } else {
                Vec::new()
            };
            assert_eq!(images, [expected.clone(), expected], "{mode:?} bulk={bulk}");
        }
    }
}

/// The neighbouring case: withdrawing an EMPTY remembered slot removes it, from
/// either origin and through either withdrawal call.
#[test]
fn withdrawing_an_empty_remembered_slot_removes_it_from_either_origin() {
    for bulk in [false, true] {
        for refreeze in [false, true] {
            let mut mutable = MutableDataset::new_with_graph_existence(
                empty_dataset(),
                GraphExistenceMode::RememberEmpty,
            );
            mutable.create_named_graph(iri("g")).expect("fresh slot");
            if refreeze {
                let frozen = mutable.freeze().expect("freezes");
                mutable = MutableDataset::new_with_graph_existence(
                    frozen,
                    GraphExistenceMode::RememberEmpty,
                );
            }
            assert_eq!(published_names(&mutable), [iri("g")]);
            if bulk {
                mutable.withdraw_named_graph_declarations();
            } else {
                mutable.withdraw_graph_declaration(&iri("g"));
            }
            assert_eq!(
                published_names(&mutable),
                [] as [TermValue; 0],
                "bulk={bulk} refreeze={refreeze}"
            );
        }
    }
}
