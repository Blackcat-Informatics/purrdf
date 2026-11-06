// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original term and proof refusal data through the portable public API.

use purrdf_datalog::term::{RetainedTerm, TermDag};
use purrdf_hash::error::ErrorValue;
use purrdf_testkit::error_capture::Capture;

fn fol_construction_checks_resident_handles_and_original_positions() {
    use purrdf_datalog::id::NodeId;
    use purrdf_datalog::resolve_fol::{
        FolClause, FolConstructionError, FolLit, FolPosition, FolProgram,
    };
    use std::collections::BTreeMap;

    let mut dag = TermDag::default();
    let head = dag.intern_leaf("head").unwrap();
    let unknown = NodeId::from_index(dag.len() as u64).unwrap();
    assert_eq!(
        FolClause::new(&dag, unknown, vec![]).unwrap_err(),
        FolConstructionError::UnknownNode {
            node: unknown,
            position: FolPosition::Head,
            clause: None,
            nodes: dag.len(),
        }
    );
    assert_eq!(
        FolClause::new(&dag, head, vec![FolLit::Neg(unknown)]).unwrap_err(),
        FolConstructionError::UnknownNode {
            node: unknown,
            position: FolPosition::Body { literal: 0 },
            clause: None,
            nodes: dag.len(),
        }
    );
    let clause = FolClause::new(&dag, head, vec![FolLit::Pos(head)]).unwrap();
    assert_eq!(
        FolProgram::new(&dag, vec![clause.clone()], unknown, vec![], BTreeMap::new()).unwrap_err(),
        FolConstructionError::UnknownNode {
            node: unknown,
            position: FolPosition::Goal,
            clause: None,
            nodes: dag.len(),
        }
    );

    assert_eq!(
        FolProgram::new(
            &dag,
            vec![clause.clone()],
            head,
            vec![(unknown, "original name".into())],
            BTreeMap::new()
        )
        .unwrap_err(),
        FolConstructionError::UnknownNode {
            node: unknown,
            position: FolPosition::GoalVariable {
                index: 0,
                name: "original name".into()
            },
            clause: None,
            nodes: dag.len(),
        }
    );

    // Revalidation catches a previously valid clause supplied with a smaller arena.
    let empty = TermDag::default();
    assert_eq!(
        FolProgram::new(&empty, vec![clause.clone()], head, vec![], BTreeMap::new()).unwrap_err(),
        FolConstructionError::UnknownNode {
            node: head,
            position: FolPosition::Head,
            clause: Some(0),
            nodes: 0,
        }
    );
    FolProgram::new(&dag, vec![clause], head, vec![], BTreeMap::new()).unwrap();
}

fn fol_projection_validation_preserves_names_and_admits_aliases() {
    use purrdf_datalog::resolve_fol::{
        FolBudget, FolClause, FolConstructionError, FolControl, FolProgram, resolve_fol,
    };
    use purrdf_datalog::unify::SortContext;
    use std::collections::BTreeMap;

    let mut dag = TermDag::default();
    let pred = dag.intern_leaf("p").unwrap();
    let value = dag.intern_leaf("value").unwrap();
    let (_, var) = dag.fresh_meta().unwrap();
    let (_, other) = dag.fresh_meta().unwrap();
    let goal = dag.intern_app(pred, vec![var]).unwrap();
    let asserted = dag.intern_app(pred, vec![value]).unwrap();
    let fact = FolClause::new(&dag, asserted, vec![]).unwrap();
    let construct =
        |projection| FolProgram::new(&dag, vec![fact.clone()], goal, projection, BTreeMap::new());
    assert_eq!(
        construct(vec![(value, "constant".into())]).unwrap_err(),
        FolConstructionError::NotMetavariable {
            index: 0,
            node: value,
            name: "constant".into()
        }
    );
    assert_eq!(
        construct(vec![(other, "unrelated".into())]).unwrap_err(),
        FolConstructionError::OutsideGoal {
            index: 0,
            node: other,
            name: "unrelated".into(),
            goal
        }
    );
    let error = construct(vec![
        (var, "original name".into()),
        (var, "original name".into()),
    ])
    .unwrap_err();
    assert_eq!(
        error,
        FolConstructionError::DuplicateName {
            name: "original name".into(),
            first: 0,
            second: 1,
            first_node: var,
            second_node: var,
        }
    );
    let retained = error.clone();
    drop(error);
    assert!(matches!(
        Capture::read(&retained).field("name"),
        ErrorValue::Text("original name")
    ));

    let program = construct(vec![(var, "X".into()), (var, "alias".into())]).unwrap();
    let FolControl::Decided(outcome) = resolve_fol(
        &mut dag,
        &program,
        &SortContext::default(),
        &FolBudget { max_steps: 100 },
    )
    .unwrap() else {
        panic!("a ground fact must be decidable");
    };
    assert_eq!(outcome.answers.len(), 1);
    assert_eq!(outcome.answers[0].atom, asserted);
    assert_eq!(outcome.answers[0].bindings.get("X").unwrap(), "value");
    assert_eq!(outcome.answers[0].bindings.get("alias").unwrap(), "value");
}

fn fol_sort_construction_checks_handles_without_rejecting_unused_declarations() {
    use purrdf_datalog::id::{MetaId, NodeId};
    use purrdf_datalog::resolve_fol::{FolConstructionError, FolPosition, FolProgram};
    use std::collections::BTreeMap;

    let mut dag = TermDag::default();
    let goal = dag.intern_leaf("goal").unwrap();
    let sort = dag.intern_leaf("sort").unwrap();
    let (unused, _) = dag.fresh_meta().unwrap();
    let unknown = MetaId::from_index(unused.index() + 1).unwrap();
    assert_eq!(
        FolProgram::new(
            &dag,
            vec![],
            goal,
            vec![],
            BTreeMap::from([(unknown, sort)])
        )
        .unwrap_err(),
        FolConstructionError::UnknownMetavariable {
            variable: unknown,
            sort
        }
    );
    let unknown_sort = NodeId::from_index(dag.len() as u64).unwrap();
    assert_eq!(
        FolProgram::new(
            &dag,
            vec![],
            goal,
            vec![],
            BTreeMap::from([(unused, unknown_sort)])
        )
        .unwrap_err(),
        FolConstructionError::UnknownNode {
            node: unknown_sort,
            position: FolPosition::Sort { variable: unused },
            clause: None,
            nodes: dag.len(),
        }
    );
    FolProgram::new(&dag, vec![], goal, vec![], BTreeMap::from([(unused, sort)])).unwrap();
}

fn reachable_snapshot_preserves_sharing_binders_and_deep_source_death() {
    let mut dag = TermDag::default();
    let op = dag.intern_leaf("operator").unwrap();
    let body = dag.intern_bound(u32::MAX, u16::MAX).unwrap();
    let binder = dag.intern_binder(op, vec![op], body).unwrap();
    let unreachable = dag.intern_leaf("unreachable").unwrap();
    let mut root = binder;
    for _ in 0..10_000 {
        root = dag.intern_app(op, vec![root, binder]).unwrap();
    }
    let snapshot = RetainedTerm::capture(&dag, root).unwrap();
    drop(dag);
    let clone = snapshot.clone();
    drop(snapshot);
    let captured = Capture::read(&clone);
    assert!(
        matches!(captured.field("root"), ErrorValue::Unsigned(value) if value == u128::from(root.index()))
    );
    let nodes = captured.records("nodes");
    assert_eq!(nodes.len(), 10_003);
    assert!(!nodes.iter().any(|node| matches!(node.field("id"), ErrorValue::Unsigned(value) if value == u128::from(unreachable.index()))));
    assert!(
        !nodes
            .iter()
            .any(|node| matches!(node.field("symbol"), ErrorValue::Text("unreachable")))
    );
    let bound = nodes
        .iter()
        .map(|node| node.child("data"))
        .find(|node| matches!(node.field("kind"), ErrorValue::Code("bound")))
        .unwrap();
    assert!(
        matches!(bound.field("debruijn"), ErrorValue::Unsigned(value) if value == u128::from(u32::MAX))
    );
    assert!(
        matches!(bound.field("slot"), ErrorValue::Unsigned(value) if value == u128::from(u16::MAX))
    );
}

fn actual_proof_decoder_preserves_rejected_bytes_and_native_lengths() {
    use purrdf_datalog::proof::{ProofArena, ProofMalformed};

    let error = ProofArena::<4>::decode(&[]).unwrap_err();
    let fields = Capture::read(&error).child("detail");
    assert_eq!(fields.code, "proof-truncated");
    assert!(matches!(fields.field("required"), ErrorValue::ByteCount(8)));
    assert!(matches!(
        fields.field("remaining"),
        ErrorValue::ByteCount(0)
    ));
    let error = ProofArena::<4>::decode(&u64::MAX.to_le_bytes()).unwrap_err();
    let fields = Capture::read(&error).child("detail");
    if usize::BITS < 64 {
        assert_eq!(fields.code, "proof-length");
        assert!(
            matches!(fields.field("value"), ErrorValue::Unsigned(value) if value == u128::from(u64::MAX))
        );
        assert!(
            matches!(fields.field("maximum"), ErrorValue::Unsigned(value) if value == usize::MAX as u128)
        );
    } else {
        assert_eq!(fields.code, "proof-truncated");
        assert!(
            matches!(fields.field("required"), ErrorValue::ByteCount(value) if value == u128::from(u64::MAX))
        );
        assert!(matches!(
            fields.field("remaining"),
            ErrorValue::ByteCount(0)
        ));
    }
    let mut input = Vec::new();
    purrdf_hash::frame::frame_le(&mut input, &[0xff, 0, 0xab]);
    let error = ProofArena::<4>::decode(&input).unwrap_err();
    drop(input);
    let copy = error.clone();
    drop(error);
    let fields = Capture::read(&copy).child("detail");
    assert_eq!(fields.code, "proof-encoding-tag");
    assert!(matches!(fields.field("actual"), ErrorValue::Bytes(bytes) if bytes == [0xff, 0, 0xab]));
    assert!(
        std::error::Error::source(&copy)
            .unwrap()
            .is::<ProofMalformed>()
    );
}

purrdf_testkit::harness_main!(
    fol_construction_checks_resident_handles_and_original_positions,
    fol_projection_validation_preserves_names_and_admits_aliases,
    fol_sort_construction_checks_handles_without_rejecting_unused_declarations,
    reachable_snapshot_preserves_sharing_binders_and_deep_source_death,
    actual_proof_decoder_preserves_rejected_bytes_and_native_lengths,
);
