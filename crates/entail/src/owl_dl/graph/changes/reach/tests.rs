// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native cache/witness controls, with no repository corpus production.

use crate::owl_dl::graph::changes::{Journal, Origin, ReadFact, ReadOwner};
use crate::owl_dl::{
    Kb,
    concept::Role,
    graph::{Assumptions, Graph, find},
};

const R: u32 = 20;
const IND: u32 = 40;

fn steps<'a>(
    slice: &'a crate::owl_dl::graph::changes::slice::Slice<'_>,
) -> impl Iterator<Item = super::Step> + 'a {
    slice
        .transitive_nodes
        .iter()
        .filter_map(|node| match node.path {
            super::Path::Step(step) => Some(step),
            super::Path::Join { .. } => None,
        })
}

#[test]
fn transitive_cache_composes_appended_edges_with_owner_and_destination_paths() {
    for warm_destination in [false, true] {
        let mut kb = Kb::empty();
        kb.transitive.insert(R);
        kb.abox_roles.extend([
            (IND, R, IND + 1),
            (IND + 1, R, IND + 2),
            (IND + 3, R, IND + 4),
            (IND + 10, R, IND + 11),
        ]);
        kb.finalize().unwrap();
        let g = Graph::new(&kb, u64::MAX);
        g.begin_changes(Journal::default());
        let mut state = g.init_state(&Assumptions::of_kb());
        let nodes: Vec<_> = (0..5)
            .map(|offset| state.root_of[&(IND + offset)])
            .collect();
        // These are ordinary native warm reads, outside selected-body scopes.
        drop(g.neighbors(&state, nodes[0], Role::Named(R)));
        if warm_destination {
            drop(g.neighbors(&state, nodes[3], Role::Named(R)));
        }
        g.change_cause(Origin::AssumedRole, 0);
        g.push_edge(&mut state, nodes[2], nodes[3], R);
        g.change_cause(Origin::Unattributed, 0);
        g.begin_round(&state, 1);
        g.select_reads(Some(ReadOwner::Clash { clause: 0 }));
        assert!(g.is_neighbour(&state, nodes[0], Role::Named(R), nodes[4]));
        g.select_reads(None);
        assert!(g.storage_refusal().is_none());
        let journal = g.finish_changes().unwrap();
        let slice = journal.normalized_slice(&kb).unwrap();
        assert_eq!(
            steps(&slice).map(|step| step.edge).collect::<Vec<_>>(),
            [0, 1, 4, 2]
        );
        assert!(
            !slice
                .effects
                .iter()
                .map(|(_, event)| &event.cause)
                .any(|cause| cause.origin == Origin::PreparedRole && cause.index == 3)
        );
        assert!(
            slice
                .effects
                .iter()
                .map(|(_, event)| &event.cause)
                .any(|cause| cause.origin == Origin::AssumedRole && cause.index == 0)
        );
        assert!(
            steps(&slice)
                .collect::<Vec<_>>()
                .windows(2)
                .all(|steps| steps[0].to == steps[1].from)
        );
    }
}

#[test]
fn transitive_cache_keeps_hierarchy_inverse_paths_and_rebuilds_after_equality() {
    for role in [Role::Named(R), Role::Inv(R), Role::Named(R + 2)] {
        let mut kb = Kb::empty();
        kb.transitive.insert(R);
        kb.role_sub.entry(R).or_default().insert(R + 1);
        kb.inverses.entry(R).or_default().insert(R + 2);
        kb.inverses.entry(R + 2).or_default().insert(R);
        kb.individuals.insert(IND - 1);
        kb.abox_roles
            .extend([(IND, R + 1, IND + 1), (IND + 1, R, IND + 2)]);
        kb.finalize().unwrap();
        let g = Graph::new(&kb, u64::MAX);
        g.begin_changes(Journal::default());
        let mut state = g.init_state(&Assumptions::of_kb());
        let original = state.root_of[&IND];
        let extra = state.root_of[&(IND - 1)];
        let target = state.root_of[&(IND + 2)];
        let inverse = role != Role::Named(R);
        let (from, to) = if inverse {
            (target, original)
        } else {
            (original, target)
        };
        g.select_reads(Some(ReadOwner::Clash { clause: 0 }));
        assert!(g.is_neighbour(&state, from, role, to));
        g.select_reads(None);
        g.select_reads(Some(ReadOwner::Application {
            index: 0,
            clause: 1,
        }));
        g.observe_read(ReadFact::Node(original));
        g.observe_read(ReadFact::Node(extra));
        g.select_reads(None);
        g.change_cause(Origin::Application, 0);
        g.merge_nodes(&mut state, original, extra);
        g.change_cause(Origin::Unattributed, 0);
        let (from, to) = (find(&state, from), find(&state, to));
        g.select_reads(Some(ReadOwner::Clash { clause: 2 }));
        assert!(g.is_neighbour(&state, from, role, to));
        g.select_reads(None);
        let slice = g.finish_changes().unwrap().normalized_slice(&kb).unwrap();
        assert!(slice.owners.contains(&ReadOwner::Application {
            index: 0,
            clause: 1
        }));
        assert_eq!(slice.transitive.len(), 1);
        assert!(slice.transitive.iter().all(|step| step.role == role));
        assert_eq!(
            steps(&slice).map(|step| step.edge).collect::<Vec<_>>(),
            if inverse { vec![1, 0] } else { vec![0, 1] }
        );
        assert!(steps(&slice).any(|step| step.letter
            == if inverse {
                Role::Inv(R + 1)
            } else {
                Role::Named(R + 1)
            }));
    }
}

#[test]
fn transitive_cache_preserves_original_refusal_in_capture_and_cached_publication() {
    use purrdf_lex::allocation::StorageError;
    for warm in [false, true] {
        let mut kb = Kb::empty();
        kb.transitive.insert(R);
        kb.abox_roles
            .extend([(IND, R, IND + 1), (IND + 1, R, IND + 2)]);
        kb.finalize().unwrap();
        let g = Graph::new(&kb, u64::MAX);
        g.begin_changes(Journal::default());
        let state = g.init_state(&Assumptions::of_kb());
        let root = state.root_of[&IND];
        if warm {
            drop(g.neighbors(&state, root, Role::Named(R)));
        }
        let mut journal = g.finish_changes().unwrap();
        journal.refuse_next_publication(StorageError::AllocationFailed);
        g.begin_changes(journal);
        g.select_reads(Some(ReadOwner::Clash { clause: 0 }));
        assert!(!g.is_neighbour(&state, root, Role::Named(R), state.root_of[&(IND + 2)]));
        assert_eq!(g.storage_refusal(), Some(StorageError::AllocationFailed));
        assert!(matches!(
            g.finish_changes().unwrap().normalized_slice(&kb),
            Err(crate::owl_dl::graph::changes::slice::Error::MissingClash)
        ));
    }
}

/// Composition can share a valid closed walk many times without expanding it.
#[test]
fn transitive_cache_slice_preserves_shared_join_graph() {
    use purrdf_lex::allocation::Memory;
    let mut kb = Kb::empty();
    kb.transitive.insert(R);
    kb.abox_roles.extend([(IND, R, IND + 1), (IND + 1, R, IND)]);
    kb.finalize().unwrap();
    let g = Graph::new(&kb, u64::MAX);
    g.begin_changes(Journal::default());
    let state = g.init_state(&Assumptions::of_kb());
    let root = state.root_of[&IND];
    assert!(g.is_neighbour(&state, root, Role::Named(R), root));
    let index = g.patterns().index((R, true)).unwrap();
    let mut journal = g.finish_changes().unwrap();
    let mut path = journal.reach_path(index, root, root);
    let base = journal.reach.paths.len();
    // Use the same admitted join primitive as appended-edge composition. Each
    // joined operand is the original native closed walk, so concatenation is valid.
    let mut memory = Memory::resume(&mut journal.storage, journal.live);
    for _ in 0..24 {
        path = journal.reach.join(Some(path), path, &mut memory).unwrap();
    }
    journal.live = memory.admitted_bytes();
    journal.read_owner = Some(ReadOwner::Clash { clause: 0 });
    journal
        .read(ReadFact::Role {
            from: root,
            to: root,
            role: Role::Named(R),
            witness: crate::owl_dl::graph::changes::RoleWitness::Transitive {
                via: Role::Named(R),
                path,
            },
        })
        .unwrap();
    let slice = journal.normalized_slice(&kb).unwrap();
    assert_eq!(
        steps(&slice).count(),
        2,
        "shared steps are not flattened repeatedly"
    );
    assert_eq!(slice.transitive_nodes.len(), base + 24);
    assert!(slice.transitive_nodes.iter().any(|node| node.id == path));
    assert!(slice.transitive_nodes.iter().all(|node| node.root == path));
    assert!(slice.transitive.iter().all(|step| step.root == path));
    for node in &slice.transitive_nodes {
        match node.path {
            super::Path::Join { first, second } => {
                assert!(first < node.id);
                assert!(second < node.id);
            }
            super::Path::Step(step) => assert!(step.edge < 2),
        }
    }
}
