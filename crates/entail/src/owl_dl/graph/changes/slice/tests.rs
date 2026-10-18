// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Effect dependency controls over actual native graph writes.

use super::*;
use crate::owl_dl::graph::{Assumptions, Graph};

/// A birth requires its recorded application and original parent, never an opaque leaf.
#[test]
fn selected_effects_require_birth_parent_and_application_and_refuse_reserved_identity() {
    let mut kb = Kb::empty();
    kb.abox_types.push((30, kb.top));
    kb.finalize().unwrap();
    let graph = Graph::new(&kb, u64::MAX);
    graph.begin_changes(Journal::default());
    let mut state = graph.init_state(&Assumptions::of_kb());
    let parent = state.root_of[&30];
    let mut selected = None;
    for index in 0..2 {
        graph.select_reads(Some(ReadOwner::Application {
            index,
            clause: index,
        }));
        graph.observe_read(ReadFact::Node(parent));
        graph.select_reads(None);
        graph.change_cause(Origin::Application, index);
        selected = Some(graph.new_successor(&mut state, parent, Role::Named(20), &[kb.top]));
    }
    let selected = selected.unwrap();
    graph.select_reads(Some(ReadOwner::Clash { clause: 2 }));
    graph.observe_read(ReadFact::Node(selected));
    graph.select_reads(None);
    let mut journal = graph.finish_changes().unwrap();
    let slice = journal.normalized_slice(&kb).unwrap();
    assert!(slice.owners.contains(&ReadOwner::Application {
        index: 1,
        clause: 1
    }));
    assert!(!slice.owners.contains(&ReadOwner::Application {
        index: 0,
        clause: 0
    }));
    assert!(
        slice.effects.iter().all(
            |(_, effect)| effect.cause.origin != Origin::Application || effect.cause.index == 1
        )
    );
    let ordinal = slice
        .effects
        .iter()
        .find_map(|&(ordinal, effect)| {
            matches!(effect.mutation,
        Mutation::Node { node, parent: Some(_), .. } if node == selected)
            .then_some(ordinal)
        })
        .unwrap();
    let original = journal.events[ordinal];
    let Mutation::Node { parent, .. } = &mut journal.events[ordinal].mutation else {
        unreachable!()
    };
    *parent = Some(usize::MAX);
    assert!(matches!(
        journal.normalized_slice(&kb),
        Err(Error::MissingFact)
    ));
    journal.events[ordinal] = original;
    journal.events[ordinal].cause.index = usize::MAX;
    assert!(matches!(
        journal.normalized_slice(&kb),
        Err(Error::MissingFact)
    ));
    journal.events[ordinal] = original;
    let Mutation::Node { reserved, .. } = &mut journal.events[ordinal].mutation else {
        unreachable!()
    };
    *reserved = true;
    assert!(matches!(
        journal.normalized_slice(&kb),
        Err(Error::MissingEffect)
    ));
    journal.events[ordinal] = original;
    assert!(journal.normalized_slice(&kb).is_ok());
}
