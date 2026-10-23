// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Effect dependency controls over actual native graph writes.

use super::*;
use crate::owl_dl::graph::{Assumptions, Graph};

/// The native clique's chosen reused witnesses and inequalities license the new pair.
#[test]
fn selected_counting_retains_reused_clique_and_excludes_other_neighbours() {
    let mut kb = Kb::empty();
    let filler = kb.table.intern(crate::owl_dl::concept::Concept::Named(40));
    kb.abox_types
        .extend([(30, kb.top), (31, filler), (32, filler)]);
    kb.abox_roles
        .extend([(30, 20, 31), (30, 20, 32), (30, 20, 33)]);
    kb.different_from.push((31, 32));
    kb.finalize().unwrap();
    let graph = Graph::new(&kb, u64::MAX);
    graph.begin_changes(Journal::default());
    let mut state = graph.init_state(&Assumptions::of_kb());
    let root = state.root_of[&30];
    let first = state.root_of[&31];
    let second = state.root_of[&32];
    let unrelated = state.root_of[&33];
    graph.select_reads(Some(ReadOwner::Application {
        index: 0,
        clause: 0,
        branch: None,
    }));
    graph.observe_read(ReadFact::Node(root));
    graph.select_reads(Some(ReadOwner::Counting {
        application: 0,
        atom: 0,
    }));
    graph.observe_read(ReadFact::Node(root));
    graph.change_cause(Origin::Application, 0);
    let fresh = state.nodes.len();
    assert!(graph.ensure_at_least(&mut state, root, 3, Role::Named(20), filler));
    assert_eq!(state.nodes.len(), fresh + 1);
    graph.select_reads(Some(ReadOwner::Clash { clause: 1 }));
    graph.observe_read(ReadFact::Distinct {
        left: first,
        right: fresh,
    });
    graph.select_reads(None);
    let mut journal = graph.finish_changes().unwrap();
    let slice = journal.normalized_slice(&kb).unwrap();
    assert_derivation(&slice, &journal);
    let reads = &slice.derivation.reads;
    assert!(reads.iter().any(|read| read.fact
        == ReadFact::Distinct {
            left: first,
            right: second
        }));
    for selected in [first, second] {
        assert!(
            reads
                .iter()
                .any(|read| matches!(read.fact, ReadFact::Role { to, .. } if to == selected))
        );
        assert!(reads.iter().any(|read| read.fact
            == ReadFact::Label {
                node: selected,
                concept: filler
            }));
    }
    assert!(
        !reads
            .iter()
            .any(|read| matches!(read.fact, ReadFact::Role { to, .. } if to == unrelated))
    );
    assert!(slice.effects.iter().any(|(_, event)| event.mutation
        == Mutation::Distinct {
            left: first,
            right: fresh
        }));
    assert!(
        !slice
            .effects
            .iter()
            .any(|(_, event)| event.cause.origin == Origin::PreparedRole && event.cause.index == 2)
    );
    drop(slice);
    journal
        .reads
        .retain(|read| !matches!(read.owner, ReadOwner::Counting { .. }));
    assert!(matches!(
        journal.normalized_slice(&kb),
        Err(Error::MissingEffect)
    ));
}

/// Compact spans retain original observations; every vertex belongs to the root's dependency closure.
pub(crate) fn assert_derivation(slice: &Slice<'_>, journal: &Journal) {
    let graph = &slice.derivation;
    assert!(matches!(
        graph.terminal(&slice.owners),
        Ok(ReadOwner::Clash { .. })
    ));
    assert!(
        graph
            .vertices
            .windows(2)
            .all(|pair| pair[0].id < pair[1].id)
    );
    let mut pending = vec![graph.root];
    let mut visited = std::collections::BTreeSet::new();
    while let Some(id) = pending.pop() {
        if !visited.insert(id) {
            continue;
        }
        let at = graph
            .vertices
            .binary_search_by_key(&id, |vertex| vertex.id)
            .unwrap();
        let vertex = &graph.vertices[at];
        for &parent in &graph.parents[vertex.parents.clone()] {
            assert!(parent < id);
            pending.push(parent);
        }
        match vertex.observation {
            Observation::Effect(index) => {
                let (ordinal, effect) = slice.effects[index];
                assert_eq!(effect, journal.events[ordinal]);
                assert!(vertex.reads.is_empty());
            }
            Observation::Reads(index) => {
                let owner = slice.owners[index];
                assert!(!vertex.reads.is_empty());
                for read in &graph.reads[vertex.reads.clone()] {
                    assert_eq!(read.owner, owner);
                    assert!(journal.reads.contains(read));
                    match read.fact {
                        ReadFact::Tbox(index) => {
                            assert!(slice.tbox[vertex.tbox.clone()].contains(&index));
                        }
                        ReadFact::SchemaGuard(index) => {
                            assert!(slice.guards[vertex.guards.clone()].contains(&index));
                        }
                        ReadFact::RoleConstraint(input) => {
                            assert!(slice.rules[vertex.rules.clone()].contains(&input));
                        }
                        ReadFact::RoleClause(input) => assert!(
                            slice.role_clauses[vertex.role_clauses.clone()].contains(&input)
                        ),
                        _ => {}
                    }
                }
            }
        }
        assert!(vertex.paths.end <= slice.paths.len());
        assert!(vertex.transitive.end <= slice.transitive.len());
        assert!(vertex.transitive_nodes.end <= slice.transitive_nodes.len());
        assert!(vertex.rules.end <= slice.rules.len());
        assert!(vertex.tbox.end <= slice.tbox.len());
        assert!(vertex.guards.end <= slice.guards.len());
        assert!(vertex.role_clauses.end <= slice.role_clauses.len());
    }
    assert_eq!(visited.len(), graph.vertices.len());
    assert_eq!(
        graph.vertices.len(),
        slice.effects.len() + slice.owners.len()
    );
}

/// The native compiler's universal component retains its original equivalence inputs.
#[test]
fn selected_universal_component_preserves_original_license_and_refuses_missing_program() {
    use crate::reasoner::schema_tests::Fixture;
    use purrdf_iri::vocab::owl;
    let mut f = Fixture::default();
    let top = f.builder.intern_iri(owl::TOP_OBJECT_PROPERTY).unwrap();
    let property = f.name("p");
    let class = f.name("A");
    f.instance("left", class);
    f.instance("right", class);
    f.edge(property, owl::EQUIVALENT_PROPERTY, top);
    let first = f.name("first");
    let second = f.name("second");
    let unrelated = f.name("unrelated");
    let list = f.list(&[first, second]);
    f.edge(unrelated, owl::PROPERTY_CHAIN_AXIOM, list);
    let mut kb = Kb::from_dataset(&f.freeze()).unwrap();
    kb.finalize().unwrap();
    let left = kb.interner.id_of_iri("https://example.org/left").unwrap();
    let right = kb.interner.id_of_iri("https://example.org/right").unwrap();
    let property = Role::Named(kb.interner.id_of_iri("https://example.org/p").unwrap());
    let mut graph = Graph::new(&kb, u64::MAX);
    // Exercise the existing compiler-only decision with its real retained source
    // component, rather than the earlier legacy hierarchy short circuit.
    graph.top_role = None;
    graph.begin_changes(Journal::default());
    let state = graph.init_state(&Assumptions::of_kb());
    graph.select_reads(Some(ReadOwner::Clash { clause: 0 }));
    assert!(graph.is_neighbour(
        &state,
        state.root_of[&left],
        property,
        state.root_of[&right]
    ));
    graph.select_reads(None);
    let journal = graph.finish_changes().unwrap();
    drop(graph);
    let slice = journal.normalized_slice(&kb).unwrap();
    assert_derivation(&slice, &journal);
    assert!(slice.derivation.reads.iter().any(|read| matches!(
        read.fact,
        ReadFact::Role {
            witness: RoleWitness::Universal(UniversalLicence::Component { .. }),
            ..
        }
    )));
    let inputs = slice.role_inputs().unwrap();
    assert_ne!(inputs, []);
    assert!(!inputs.contains(&crate::owl_dl::roles::source::Input::Chain(0)));
    drop(slice);
    let program = kb.role_program.take();
    assert!(matches!(
        journal.normalized_slice(&kb).unwrap().role_inputs(),
        Err(Error::MissingPath)
    ));
    kb.role_program = program;
    assert_ne!(
        journal
            .normalized_slice(&kb)
            .unwrap()
            .role_inputs()
            .unwrap(),
        []
    );
}

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
            branch: None,
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
    let mut slice = journal.normalized_slice(&kb).unwrap();
    assert_derivation(&slice, &journal);
    assert!(slice.owners.contains(&ReadOwner::Application {
        index: 1,
        clause: 1,
        branch: None
    }));
    assert!(!slice.owners.contains(&ReadOwner::Application {
        index: 0,
        clause: 0,
        branch: None
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
    let born = slice.derivation.vertices.iter().find(|vertex| {
        matches!(vertex.observation, Observation::Effect(index) if slice.effects[index].0 == ordinal)
    }).unwrap();
    let parents = &slice.derivation.parents[born.parents.clone()];
    assert_eq!(
        parents.len(),
        2,
        "the checked application and original parent birth"
    );
    assert!(parents.iter().any(|id| {
        let vertex = &slice.derivation.vertices[slice.derivation.vertices.binary_search_by_key(id, |v| v.id).unwrap()];
        matches!(vertex.observation, Observation::Reads(index) if slice.owners[index] == ReadOwner::Application { index: 1, clause: 1, branch: None })
    }));
    let original_root = slice.derivation.root;
    slice.derivation.root = born.id;
    assert!(matches!(
        slice.derivation.terminal(&slice.owners),
        Err(Error::MissingClash)
    ));
    slice.derivation.root = usize::MAX;
    assert!(matches!(
        slice.derivation.terminal(&slice.owners),
        Err(Error::MissingClash)
    ));
    slice.derivation.root = original_root;
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
