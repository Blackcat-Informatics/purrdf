// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Proof-local dependency construction over one native replay's ordered observations.
//! This private qualification seam does not resolve original RDF/schema licenses.

use super::reach;
use super::{
    Cause, Event, Journal, Mutation, Origin, PathKind, PathStep, ReadFact, ReadOwner, RoleWitness,
};
use crate::owl_dl::concept::Role;
use purrdf_lex::allocation::{Memory, OrderedTable, Resident, StorageError};
use std::ops::Range;

/// One normalized transitivity/hierarchy license for a selected immutable path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TransitiveRead {
    pub(crate) root: usize,
    pub(crate) from: usize,
    pub(crate) to: usize,
    pub(crate) role: Role,
    pub(crate) via: Role,
}

/// Preserve shared joins instead of exponentially expanding a composed walk.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TransitiveNode {
    pub(crate) root: usize,
    pub(crate) id: usize,
    pub(crate) path: reach::Path,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Fact {
    Node(usize),
    Domain(usize),
    Label(usize, u32),
    Name(usize, u32),
    Edge(usize),
    Distinct(usize, usize),
    Value(usize, u32),
}

struct Dependency {
    cause: Option<Cause>,
    owner: Option<ReadOwner>,
    parents: Range<usize>,
    paths: Range<usize>,
    transitive: Range<usize>,
    transitive_nodes: Range<usize>,
    incomplete: bool,
}

/// Missing native derivation is a refusal, never a successful opaque leaf.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Error {
    Storage(StorageError),
    MissingFact,
    MissingPath,
    MissingEffect,
    MissingClash,
}

impl From<StorageError> for Error {
    fn from(error: StorageError) -> Self {
        Self::Storage(error)
    }
}

/// Native ordinals only. These are not yet source-bound RDF premises.
#[derive(Debug)]
pub(crate) struct Slice {
    pub(crate) causes: Vec<Cause>,
    pub(crate) owners: Vec<ReadOwner>,
    pub(crate) paths: Vec<PathStep>,
    pub(crate) transitive: Vec<TransitiveRead>,
    pub(crate) transitive_nodes: Vec<TransitiveNode>,
}

#[derive(Default)]
struct Builder {
    facts: OrderedTable<Fact, usize>,
    applications: OrderedTable<usize, usize>,
    aliases: OrderedTable<usize, (usize, usize)>,
    edges: OrderedTable<usize, (usize, usize, u32)>,
    nodes: Vec<Dependency>,
    parents: Vec<usize>,
    paths: Vec<PathStep>,
    transitive: Vec<TransitiveRead>,
    transitive_nodes: Vec<TransitiveNode>,
}

impl Builder {
    fn require(&self, fact: Fact) -> Result<usize, Error> {
        self.facts.get(&fact).copied().ok_or(Error::MissingFact)
    }

    fn parent(&mut self, fact: Fact, memory: &mut Memory<'_, Resident>) -> Result<(), Error> {
        let version = self.require(fact)?;
        memory.push(&mut self.parents, version)?;
        Ok(())
    }

    fn alias(
        &mut self,
        mut node: usize,
        target: usize,
        memory: &mut Memory<'_, Resident>,
    ) -> Result<(), Error> {
        while node != target {
            let &(keep, cause) = self.aliases.get(&node).ok_or(Error::MissingFact)?;
            memory.push(&mut self.parents, cause)?;
            node = keep;
        }
        Ok(())
    }

    fn edge(
        &mut self,
        index: usize,
        role: Role,
        from: usize,
        to: usize,
        memory: &mut Memory<'_, Resident>,
    ) -> Result<(), Error> {
        let &(raw_from, raw_to, property) = self.edges.get(&index).ok_or(Error::MissingFact)?;
        let (left, right) = match role {
            Role::Named(wanted) if wanted == property => (raw_from, raw_to),
            Role::Inv(wanted) if wanted == property => (raw_to, raw_from),
            _ => return Err(Error::MissingPath),
        };
        self.parent(Fact::Edge(index), memory)?;
        self.alias(left, from, memory)?;
        self.alias(right, to, memory)
    }

    fn read(
        &mut self,
        fact: ReadFact,
        paths: &[PathStep],
        reach: &[reach::Path],
        memory: &mut Memory<'_, Resident>,
    ) -> Result<(), Error> {
        match fact {
            ReadFact::Node(node) | ReadFact::Top(node) => self.parent(Fact::Node(node), memory),
            ReadFact::ObjectDomain(node) => self.parent(Fact::Domain(node), memory),
            ReadFact::Label { node, concept } => self.parent(Fact::Label(node, concept), memory),
            ReadFact::Denotes { node, individual } => {
                self.parent(Fact::Name(node, individual), memory)
            }
            ReadFact::Role {
                from,
                to,
                role,
                witness: RoleWitness::Edge(index),
            } => self.edge(index, role, from, to, memory),
            ReadFact::Role {
                witness: RoleWitness::Language { start, end },
                ..
            } => {
                for step in paths.get(start..end).ok_or(Error::MissingPath)? {
                    match step.kind {
                        PathKind::Edge { index, letter } => {
                            self.edge(index, letter, step.from.0, step.to.0, memory)?;
                        }
                        PathKind::Epsilon => self.parent(Fact::Node(step.from.0), memory)?,
                        PathKind::Universal(_) => return Err(Error::MissingPath),
                    }
                    memory.push(&mut self.paths, *step)?;
                }
                Ok(())
            }
            ReadFact::Role {
                from,
                to,
                role,
                witness: RoleWitness::Transitive { via, path },
            } => {
                let mut pending = Vec::new();
                let mut visited = OrderedTable::default();
                memory.push(
                    &mut self.transitive,
                    TransitiveRead {
                        root: path,
                        from,
                        to,
                        role,
                        via,
                    },
                )?;
                memory.push(&mut pending, path)?;
                while let Some(id) = pending.pop() {
                    if visited.insert(id, (), memory)?.is_some() {
                        continue;
                    }
                    let node = *reach.get(id).ok_or(Error::MissingPath)?;
                    memory.push(
                        &mut self.transitive_nodes,
                        TransitiveNode {
                            root: path,
                            id,
                            path: node,
                        },
                    )?;
                    match node {
                        reach::Path::Step(step) => {
                            self.edge(step.edge, step.letter, step.from, step.to, memory)?;
                        }
                        reach::Path::Join { first, second } => {
                            memory.push(&mut pending, second)?;
                            memory.push(&mut pending, first)?;
                        }
                    }
                }
                memory.release_vec(pending)?;
                visited.release(memory)?;
                Ok(())
            }
            ReadFact::Role { .. } => Err(Error::MissingPath),
        }
    }

    fn event(&mut self, event: Event, memory: &mut Memory<'_, Resident>) -> Result<(), Error> {
        let start = self.parents.len();
        let cause = event.cause;
        if cause.origin == Origin::Application {
            let &application = self
                .applications
                .get(&cause.index)
                .ok_or(Error::MissingFact)?;
            memory.push(&mut self.parents, application)?;
        }
        let id = self.nodes.len();
        let mut incomplete = false;
        match event.mutation {
            Mutation::Node {
                node,
                named,
                parent,
                reserved,
                concrete,
            } => {
                incomplete = parent.is_some() || reserved;
                self.facts.insert(Fact::Node(node), id, memory)?;
                if !concrete {
                    self.facts.insert(Fact::Domain(node), id, memory)?;
                }
                if let Some(name) = named {
                    self.facts.insert(Fact::Name(node, name), id, memory)?;
                }
            }
            Mutation::Label { node, concept } => {
                self.parent(Fact::Node(node), memory)?;
                self.facts.insert(Fact::Label(node, concept), id, memory)?;
            }
            Mutation::Edge {
                index,
                from,
                to,
                property,
            } => {
                self.parent(Fact::Node(from), memory)?;
                self.parent(Fact::Node(to), memory)?;
                self.edges.insert(index, (from, to, property), memory)?;
                self.facts.insert(Fact::Edge(index), id, memory)?;
            }
            Mutation::Merge { keep, discard } => {
                self.parent(Fact::Node(keep), memory)?;
                self.parent(Fact::Node(discard), memory)?;
                self.aliases.insert(discard, (keep, id), memory)?;
                incomplete = cause.origin == Origin::PreparedEquality;
            }
            Mutation::LabelTransfer {
                keep,
                discard,
                concept,
            } => {
                self.transfer(
                    Fact::Label(discard, concept),
                    Fact::Label(keep, concept),
                    discard,
                    id,
                    memory,
                )?;
            }
            Mutation::NameTransfer {
                keep,
                discard,
                individual,
            } => {
                self.transfer(
                    Fact::Name(discard, individual),
                    Fact::Name(keep, individual),
                    discard,
                    id,
                    memory,
                )?;
            }
            Mutation::Distinct { left, right } => {
                incomplete = cause.origin == Origin::Application;
                self.facts.insert(distinct(left, right), id, memory)?;
            }
            Mutation::DistinctTransfer {
                keep,
                discard,
                previous_other,
                other,
            } => {
                self.alias(previous_other, other, memory)?;
                self.transfer(
                    distinct(discard, previous_other),
                    distinct(keep, other),
                    discard,
                    id,
                    memory,
                )?;
            }
            Mutation::DomainTransfer { keep, .. } => {
                self.facts.remove(&Fact::Domain(keep));
            }
            Mutation::ValueClass {
                node,
                class,
                inherited_from,
            } => {
                if let Some(discard) = inherited_from {
                    self.transfer(
                        Fact::Value(discard, class),
                        Fact::Value(node, class),
                        discard,
                        id,
                        memory,
                    )?;
                } else {
                    self.facts.insert(Fact::Value(node, class), id, memory)?;
                }
            }
            Mutation::WithdrawLabel { node, concept } => {
                self.facts.remove(&Fact::Label(node, concept));
            }
        }
        memory.push(
            &mut self.nodes,
            Dependency {
                cause: Some(cause),
                owner: None,
                parents: start..self.parents.len(),
                paths: 0..0,
                transitive: 0..0,
                transitive_nodes: 0..0,
                incomplete,
            },
        )?;
        Ok(())
    }

    fn transfer(
        &mut self,
        old: Fact,
        new: Fact,
        discard: usize,
        id: usize,
        memory: &mut Memory<'_, Resident>,
    ) -> Result<(), Error> {
        self.parent(old, memory)?;
        let &(_, merge) = self.aliases.get(&discard).ok_or(Error::MissingFact)?;
        memory.push(&mut self.parents, merge)?;
        self.facts.insert(new, id, memory)?;
        Ok(())
    }
}

const fn distinct(left: usize, right: usize) -> Fact {
    if left < right {
        Fact::Distinct(left, right)
    } else {
        Fact::Distinct(right, left)
    }
}

impl Journal {
    /// Consume observations once in replay order, then walk immutable parents.
    /// Unresolved role/effect support is rejected if selected by the terminal slice.
    pub(crate) fn normalized_slice(&self) -> Result<Slice, Error> {
        let mut storage = Resident;
        let mut memory = Memory::new(&mut storage);
        let mut graph = Builder::default();
        let mut event = 0;
        let mut read = 0;
        let mut root = None;
        while read < self.reads.len() {
            let selected = self.reads[read];
            while event < selected.checkpoint {
                graph.event(self.events[event], &mut memory)?;
                event += 1;
            }
            let start = graph.parents.len();
            let path_start = graph.paths.len();
            let transitive_start = graph.transitive.len();
            let transitive_nodes_start = graph.transitive_nodes.len();
            let mut incomplete = false;
            while let Some(next) = self.reads.get(read)
                && next.owner == selected.owner
                && next.checkpoint == selected.checkpoint
            {
                match graph.read(next.fact, &self.paths, &self.reach.paths, &mut memory) {
                    Ok(()) => {}
                    Err(Error::MissingPath) => incomplete = true,
                    Err(error) => return Err(error),
                }
                read += 1;
            }
            let id = graph.nodes.len();
            memory.push(
                &mut graph.nodes,
                Dependency {
                    cause: None,
                    owner: Some(selected.owner),
                    parents: start..graph.parents.len(),
                    paths: path_start..graph.paths.len(),
                    transitive: transitive_start..graph.transitive.len(),
                    transitive_nodes: transitive_nodes_start..graph.transitive_nodes.len(),
                    incomplete,
                },
            )?;
            match selected.owner {
                ReadOwner::Application { index, .. } => {
                    graph.applications.insert(index, id, &mut memory)?;
                }
                ReadOwner::Clash { .. } => root = Some(id),
            }
        }
        let mut pending = Vec::new();
        let mut visited = Vec::new();
        memory.reserve(&mut visited, graph.nodes.len())?;
        visited.resize(graph.nodes.len(), false);
        memory.push(&mut pending, root.ok_or(Error::MissingClash)?)?;
        let mut result = Slice {
            causes: Vec::new(),
            owners: Vec::new(),
            paths: Vec::new(),
            transitive: Vec::new(),
            transitive_nodes: Vec::new(),
        };
        while let Some(id) = pending.pop() {
            if visited[id] {
                continue;
            }
            visited[id] = true;
            let node = &graph.nodes[id];
            if node.incomplete {
                return Err(if node.owner.is_some() {
                    Error::MissingPath
                } else {
                    Error::MissingEffect
                });
            }
            if let Some(cause) = node.cause {
                memory.push(&mut result.causes, cause)?;
            }
            if let Some(owner) = node.owner {
                memory.push(&mut result.owners, owner)?;
            }
            memory.extend(
                &mut result.paths,
                graph.paths[node.paths.clone()].iter().copied(),
            )?;
            memory.extend(
                &mut result.transitive,
                graph.transitive[node.transitive.clone()].iter().copied(),
            )?;
            memory.extend(
                &mut result.transitive_nodes,
                graph.transitive_nodes[node.transitive_nodes.clone()]
                    .iter()
                    .copied(),
            )?;
            for &parent in &graph.parents[node.parents.clone()] {
                memory.push(&mut pending, parent)?;
            }
        }
        Ok(result)
    }
}
