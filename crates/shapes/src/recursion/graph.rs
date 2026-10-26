// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Shared signed-component evidence for source and native model admission.

use purrdf_core::graph::{GraphError, tarjan_scc_with_memory};
use purrdf_lex::allocation::{Memory, OrderedTable, Resident, Shared, StorageError};
use std::sync::{Arc, Weak};

use super::RecursionSemantics;
use crate::error::{
    RecursionDependency, RecursionPolarity, RecursionRefusal, RecursionRefusalReason, ShapesError,
};

pub(crate) type Dependency = (usize, usize, &'static str, RecursionPolarity);

#[derive(Clone, Default)]
pub(crate) struct Analysis {
    state: Option<Shared<State>>,
}

enum State {
    Cycles(Vec<Cycle>),
}

/// Declared source edges may include statically deactivated occurrences that
/// the executable native projection omits. Keep that evidence with its source,
/// rather than installing it as a different result in the native-model cache.
pub(crate) struct Source {
    owner: Weak<crate::shapes::ModelOwner>,
    analysis: Analysis,
}

impl Source {
    pub(crate) fn new(
        owner: &Arc<crate::shapes::ModelOwner>,
        analysis: Analysis,
    ) -> Result<Option<Shared<Self>>, ShapesError> {
        if analysis.state.is_none() {
            return Ok(None);
        }
        Shared::try_new(Self {
            owner: Arc::downgrade(owner),
            analysis,
        })
        .map(Some)
        .map_err(|_| StorageError::AllocationFailed.into())
    }
    pub(crate) fn admit(
        &self,
        owner: &Arc<crate::shapes::ModelOwner>,
        semantics: Option<RecursionSemantics>,
    ) -> Result<(), ShapesError> {
        if self.owner.as_ptr() == Arc::as_ptr(owner) {
            self.analysis.admit(semantics)
        } else {
            Ok(())
        }
    }
}

purrdf_hash::debug_non_exhaustive!(Source {});

struct Cycle {
    definitions: Vec<(usize, super::closed::WeakDefinition)>,
    identities: Vec<(usize, usize)>,
    members: Vec<String>,
    edges: Vec<RecursionDependency>,
    nonmonotone: bool,
    scoped_edges: Vec<crate::error::ScopedRecursionDependency>,
}

impl Analysis {
    pub(crate) fn admit(&self, semantics: Option<RecursionSemantics>) -> Result<(), ShapesError> {
        let Some(state) = self.state.as_deref() else {
            return Ok(());
        };
        let State::Cycles(cycles) = state;
        for cycle in cycles {
            let reason = match semantics {
                None => RecursionRefusalReason::MissingSelection,
                Some(semantics) if cycle.nonmonotone => {
                    RecursionRefusalReason::NonStratified { semantics }
                }
                Some(_) => continue,
            };
            let component = crate::error::RecursionComponent {
                identities: {
                    let mut resident = Resident;
                    Memory::new(&mut resident).collect(cycle.identities.iter().copied())?
                },
                scoped_edges: {
                    let mut resident = Resident;
                    let mut memory = Memory::new(&mut resident);
                    memory.collect(cycle.scoped_edges.iter().copied())?
                },
                definitions: {
                    let mut resident = Resident;
                    let mut memory = Memory::new(&mut resident);
                    let mut definitions = Vec::new();
                    for (_, original) in &cycle.definitions {
                        memory.push(&mut definitions, original.publish()?)?;
                    }
                    definitions
                },
                members: {
                    let mut resident = Resident;
                    let mut memory = Memory::new(&mut resident);
                    let mut members = Vec::new();
                    for member in &cycle.members {
                        let text = memory.string(member)?;
                        memory.push(&mut members, text)?;
                    }
                    members
                },
                edges: {
                    let mut resident = Resident;
                    let mut memory = Memory::new(&mut resident);
                    let mut edges = Vec::new();
                    for edge in &cycle.edges {
                        let source = memory.string(&edge.source)?;
                        let target = memory.string(&edge.target)?;
                        let parameter = memory.string(&edge.parameter)?;
                        memory.push(
                            &mut edges,
                            RecursionDependency {
                                source,
                                target,
                                parameter,
                                polarity: edge.polarity,
                            },
                        )?;
                    }
                    edges
                },
            };
            return Err(ShapesError::Recursion(RecursionRefusal::component(
                reason, component,
            )?));
        }
        Ok(())
    }
}

pub(crate) fn analyze(
    adjacency: &[Vec<usize>],
    dependencies: &[Dependency],
    label: impl Fn(usize) -> Result<String, ShapesError>,
    shape: impl Fn(usize) -> bool,
    definition: impl Fn(usize) -> Result<super::closed::WeakDefinition, ShapesError>,
) -> Result<Analysis, ShapesError> {
    let mut resident = Resident;
    let mut memory = Memory::new(&mut resident);
    let mut components =
        tarjan_scc_with_memory(adjacency, &mut memory).map_err(|error| match error {
            GraphError::Storage(error) => ShapesError::Storage(error),
            error @ GraphError::InvalidEdge { .. } => ShapesError::internal(error.to_string()),
        })?;
    for component in &mut components {
        component.sort_unstable();
    }
    components.sort_unstable();
    let mut cycles = Vec::new();
    for component in components {
        if (component.len() == 1 && !adjacency[component[0]].contains(&component[0]))
            || !component.iter().any(|&node| shape(node))
        {
            continue;
        }
        let mut members = Vec::new();
        for &node in &component {
            let label = label(node)?;
            memory.push(&mut members, label)?;
        }
        let mut definitions = Vec::new();
        for &node in &component {
            memory.push(&mut definitions, (node, definition(node)?))?;
        }
        // Source census terms are unique and sorted; native catalog seeds are
        // sorted too, with local occurrences interned in original ordered AST
        // traversal order. Retain that ordinal only for equal native terms.
        definitions.sort_unstable_by(|(a, left), (b, right)| {
            crate::term::canonical_cmp(left.native(), right.native())
                .then_with(|| left.kind().cmp(&right.kind()))
                .then_with(|| a.cmp(b))
        });
        let mut positions: OrderedTable<usize, usize> = OrderedTable::default();
        let mut domains: OrderedTable<Option<usize>, usize> = OrderedTable::default();
        let mut occurrences: OrderedTable<
            (usize, super::closed::RecursionDefinitionKind, usize),
            usize,
        > = OrderedTable::default();
        let mut identities = Vec::new();
        for (position, (node, original)) in definitions.iter().enumerate() {
            positions.try_insert_with(
                *node,
                &mut memory,
                |a, b| Ok::<_, ShapesError>(a.cmp(b)),
                |_, _, _| Ok::<_, ShapesError>(position),
            )?;
            let key = original.model_key();
            let domain = match domains.get(&key) {
                Some(&value) => value,
                None => {
                    let value = domains.len();
                    domains.try_insert_with(
                        key,
                        &mut memory,
                        |a, b| Ok::<_, ShapesError>(a.cmp(b)),
                        |_, _, _| Ok::<_, ShapesError>(value),
                    )?;
                    value
                }
            };
            let key = (domain, original.kind(), original.occurrence());
            let occurrence = match occurrences.get(&key) {
                Some(&value) => value,
                None => {
                    let value = occurrences.len();
                    occurrences.try_insert_with(
                        key,
                        &mut memory,
                        |a, b| Ok::<_, ShapesError>(a.cmp(b)),
                        |_, _, _| Ok::<_, ShapesError>(value),
                    )?;
                    value
                }
            };
            memory.push(&mut identities, (domain, occurrence))?;
        }
        domains.release(&mut memory)?;
        occurrences.release(&mut memory)?;
        members.sort_unstable();
        members.dedup();
        let mut edges = Vec::new();
        let mut nonmonotone = false;
        let mut scoped_edges = Vec::new();
        for &(source, target, parameter, polarity) in dependencies {
            if component.binary_search(&source).is_err()
                || component.binary_search(&target).is_err()
            {
                continue;
            }
            memory.push(
                &mut scoped_edges,
                crate::error::ScopedRecursionDependency {
                    source: *positions
                        .get(&source)
                        .ok_or_else(|| ShapesError::internal("scoped source left its component"))?,
                    target: *positions
                        .get(&target)
                        .ok_or_else(|| ShapesError::internal("scoped target left its component"))?,
                    parameter,
                    polarity,
                },
            )?;
            nonmonotone |= matches!(
                polarity,
                RecursionPolarity::Negative | RecursionPolarity::Both
            );
            let source = label(source)?;
            let target = label(target)?;
            let parameter = memory.string(parameter)?;
            memory.push(
                &mut edges,
                RecursionDependency {
                    source,
                    target,
                    parameter,
                    polarity,
                },
            )?;
        }
        positions.release(&mut memory)?;
        scoped_edges.sort_unstable();
        edges.sort_unstable();
        edges.dedup();
        memory.push(
            &mut cycles,
            Cycle {
                definitions,
                identities,
                members,
                edges,
                nonmonotone,
                scoped_edges,
            },
        )?;
    }
    Ok(Analysis {
        state: if cycles.is_empty() {
            None
        } else {
            Some(
                Shared::try_new(State::Cycles(cycles))
                    .map_err(|_| StorageError::AllocationFailed)?,
            )
        },
    })
}
