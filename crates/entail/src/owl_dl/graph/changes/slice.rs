// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Proof-local dependency construction over one native replay's ordered observations.
//! Selected input anchors, syntax and source licenses retain their original
//! native owners; unsupported effects refuse.

use super::reach;
use super::{
    Cause, Event, Journal, Mutation, Origin, PathKind, PathStep, Read, ReadFact, ReadOwner,
    RoleWitness, UniversalLicence,
};
use crate::owl_dl::concept::Role;
use crate::owl_dl::{
    AxiomSource, Kb,
    input_source::{Input, Source},
};
use purrdf_lex::allocation::{Memory, OrderedTable, Resident, StorageError};
use std::ops::Range;
mod key;
pub(crate) mod schema;
#[cfg(test)]
pub(crate) mod tests;

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
    event: Option<usize>,
    owner: Option<ReadOwner>,
    parents: Range<usize>,
    reads: Range<usize>,
    paths: Range<usize>,
    transitive: Range<usize>,
    transitive_nodes: Range<usize>,
    rules: Range<usize>,
    tbox: Range<usize>,
    guards: Range<usize>,
    role_clauses: Range<usize>,
    incomplete: bool,
}

/// A selected observation addresses its one canonical effect or read-owner entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Observation {
    Effect(usize),
    Reads(usize),
}

/// Original replay-local vertex and its compact selected dependency/license spans.
/// Parent values are original vertex identities, never RDF term identifiers.
#[derive(Debug)]
pub(crate) struct Vertex {
    pub(crate) id: usize,
    pub(crate) observation: Observation,
    pub(crate) parents: Range<usize>,
    pub(crate) reads: Range<usize>,
    pub(crate) paths: Range<usize>,
    pub(crate) transitive: Range<usize>,
    pub(crate) transitive_nodes: Range<usize>,
    pub(crate) rules: Range<usize>,
    pub(crate) tbox: Range<usize>,
    pub(crate) guards: Range<usize>,
    pub(crate) role_clauses: Range<usize>,
}

/// Only the backwards-reachable checked derivation survives journal disposal.
#[derive(Debug)]
pub(crate) struct Derivation {
    pub(crate) root: usize,
    /// Sorted original vertex identities permit borrowed lookup without another index.
    pub(crate) vertices: Vec<Vertex>,
    pub(crate) parents: Vec<usize>,
    pub(crate) reads: Vec<Read>,
    /// Sparse selected node births address the one canonical effect table.
    pub(crate) births: Vec<(usize, usize)>,
}

impl Derivation {
    /// A retained root must resolve to a selected terminal read, never an effect leaf.
    fn terminal(&self, owners: &[ReadOwner]) -> Result<ReadOwner, Error> {
        let at = self
            .vertices
            .binary_search_by_key(&self.root, |vertex| vertex.id)
            .map_err(|_| Error::MissingClash)?;
        let Observation::Reads(owner) = self.vertices[at].observation else {
            return Err(Error::MissingClash);
        };
        match owners.get(owner) {
            Some(owner @ (ReadOwner::Clash { .. } | ReadOwner::Merge { .. })) => Ok(*owner),
            _ => Err(Error::MissingClash),
        }
    }
}

/// Missing native derivation is a refusal, never a successful opaque leaf.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Error {
    Storage(StorageError),
    MissingFact,
    MissingPath,
    MissingEffect,
    MissingKey(usize),
    MissingMembership { key: usize, individual: u32 },
    MissingClash,
    MissingInput(Input),
    MissingSyntax,
    MissingSchema(usize),
    UnexpectedSchema,
    Stopped,
    RoleLicence(crate::owl_dl::roles::LicenceError),
}

/// Owned dependencies selected while their original source phase is still active.
/// Schema reads address the enclosing captured proof's checked terminal evidence.
pub(crate) struct Support {
    pub(crate) derivation: Derivation,
    /// Original journal ordinals and effects, local to this checked clash's replay.
    pub(crate) effects: Vec<(usize, Event)>,
    pub(crate) owners: Vec<ReadOwner>,
    pub(crate) paths: Vec<PathStep>,
    pub(crate) path_ordinals: Vec<usize>,
    pub(crate) transitive: Vec<TransitiveRead>,
    pub(crate) transitive_nodes: Vec<TransitiveNode>,
    pub(crate) rules: Vec<Input>,
    pub(crate) tbox: Vec<usize>,
    pub(crate) internalized: Vec<(u32, usize)>,
    pub(crate) guards: Vec<usize>,
    pub(crate) role_clauses: Vec<crate::owl_dl::roles::RoleClause>,
    pub(crate) schema_reads: Vec<schema::Read>,
    pub(crate) inputs: Vec<SelectedInput>,
    pub(crate) syntax: Vec<(crate::owl_dl::syntax::Space, usize)>,
    pub(crate) roles: Vec<crate::owl_dl::roles::source::Input>,
    pub(crate) key_equalities: Vec<usize>,
}

impl std::fmt::Debug for Support {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Support")
            .field("derivation", &self.derivation)
            .field("effects", &self.effects)
            .field("owners", &self.owners)
            .field("paths", &self.paths)
            .field("path_ordinals", &self.path_ordinals)
            .field("transitive", &self.transitive)
            .field("transitive_nodes", &self.transitive_nodes)
            .field("rules", &self.rules)
            .field("tbox", &self.tbox)
            .field("internalized", &self.internalized)
            .field("guards", &self.guards)
            .field("role_clauses", &self.role_clauses)
            .field("schema_reads", &self.schema_reads)
            .field("inputs", &self.inputs)
            .field("syntax", &self.syntax)
            .field("roles", &self.roles)
            .field("key_equalities", &self.key_equalities)
            .finish()
    }
}

purrdf_lex::variant_from!(Error { Storage(StorageError) });

fn poll(kb: &Kb) -> Result<(), Error> {
    if kb
        .stop
        .as_deref()
        .is_some_and(purrdf_datalog::StopSignal::stopped)
    {
        Err(Error::Stopped)
    } else {
        Ok(())
    }
}

/// Checked native dependencies, with separately bound original input anchors.
pub(crate) struct Slice<'a> {
    kb: &'a Kb,
    pub(crate) derivation: Derivation,
    pub(crate) effects: Vec<(usize, Event)>,
    pub(crate) owners: Vec<ReadOwner>,
    pub(crate) paths: Vec<PathStep>,
    pub(crate) path_ordinals: Vec<usize>,
    pub(crate) transitive: Vec<TransitiveRead>,
    pub(crate) transitive_nodes: Vec<TransitiveNode>,
    pub(crate) rules: Vec<Input>,
    pub(crate) tbox: Vec<usize>,
    pub(crate) internalized: Vec<(u32, usize)>,
    guards: Vec<usize>,
    pub(crate) role_clauses: Vec<crate::owl_dl::roles::RoleClause>,
    pub(crate) schema: Option<crate::owl_dl::proof::CheckedSchema<'a>>,
    pub(crate) schema_reads: Vec<schema::Read>,
    pub(crate) keys: Vec<key::Derivation<'a>>,
}

impl std::fmt::Debug for Slice<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Slice")
            .field("derivation", &self.derivation)
            .field("effects", &self.effects)
            .field("source_terms", &self.kb.source_terms)
            .field("owners", &self.owners)
            .field("paths", &self.paths)
            .field("path_ordinals", &self.path_ordinals)
            .field("transitive", &self.transitive)
            .field("transitive_nodes", &self.transitive_nodes)
            .field("rules", &self.rules)
            .field("tbox", &self.tbox)
            .field("internalized", &self.internalized)
            .field("guards", &self.guards)
            .field("role_clauses", &self.role_clauses)
            .field("schema", &self.schema)
            .field("schema_reads", &self.schema_reads)
            .field("keys", &self.keys)
            .finish()
    }
}

/// One selected assertion or role-rule input and its original native source owner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SelectedInput {
    pub(crate) cause: Option<Cause>,
    pub(crate) input: Input,
    pub(crate) source: Source,
}

impl Slice<'_> {
    pub(crate) fn into_support(self) -> Result<Support, Error> {
        self.derivation.terminal(&self.owners)?;
        let mut storage = Resident;
        let mut memory = Memory::new(&mut storage);
        let mut inputs = Vec::new();
        for input in self.inputs() {
            poll(self.kb)?;
            memory.push(&mut inputs, input?)?;
        }
        let syntax = self.syntax_roots()?;
        let roles = self.role_inputs()?;
        let key_equalities = memory.collect(self.keys.iter().map(|key| key.equality))?;
        Ok(Support {
            derivation: self.derivation,
            effects: self.effects,
            owners: self.owners,
            paths: self.paths,
            path_ordinals: self.path_ordinals,
            transitive: self.transitive,
            transitive_nodes: self.transitive_nodes,
            rules: self.rules,
            tbox: self.tbox,
            internalized: self.internalized,
            guards: self.guards,
            role_clauses: self.role_clauses,
            schema_reads: self.schema_reads,
            inputs,
            syntax,
            roles,
            key_equalities,
        })
    }
    pub(crate) fn role_inputs(&self) -> Result<Vec<crate::owl_dl::roles::source::Input>, Error> {
        let mut inputs = self.local_role_inputs()?;
        let mut storage = Resident;
        let mut memory = Memory::new(&mut storage);
        for support in self
            .keys
            .iter()
            .flat_map(|key| &key.members)
            .flat_map(key::Member::supports)
        {
            memory.extend(&mut inputs, support.roles.iter().copied())?;
        }
        Ok(inputs)
    }

    fn local_role_inputs(&self) -> Result<Vec<crate::owl_dl::roles::source::Input>, Error> {
        let Some(program) = &self.kb.role_program else {
            let component = self.derivation.reads.iter().any(|read| {
                matches!(
                    read.fact,
                    ReadFact::Role {
                        witness: RoleWitness::Universal(UniversalLicence::Component { .. }),
                        ..
                    }
                )
            });
            return if self.paths.is_empty() && self.role_clauses.is_empty() && !component {
                Ok(Vec::new())
            } else {
                Err(Error::MissingPath)
            };
        };
        let mut storage = Resident;
        let mut memory = Memory::new(&mut storage);
        memory.try_scope(|memory| {
            let mut transitions = Vec::new();
            memory.extend(
                &mut transitions,
                self.paths.iter().map(|step| (step.role, step.transition)),
            )?;
            for source in &self.role_clauses {
                let obligation = program
                    .obligations
                    .get(source.obligation)
                    .ok_or(Error::MissingPath)?;
                let crate::owl_dl::concept::Decomp::All(role, _) =
                    self.kb.table.decomp(obligation.source)
                else {
                    return Err(Error::MissingPath);
                };
                if let crate::owl_dl::roles::RoleClauseKind::Transition(transition) = source.kind {
                    memory.push(&mut transitions, (*role, transition))?;
                }
            }
            let inclusions = self
                .derivation
                .reads
                .iter()
                .filter_map(|read| match read.fact {
                    ReadFact::Role {
                        role,
                        witness: RoleWitness::Universal(UniversalLicence::Component { top }),
                        ..
                    } => Some((top, role)),
                    _ => None,
                });
            let inputs = program
                .path_inputs(
                    transitions.iter().copied(),
                    inclusions,
                    self.kb.stop.as_deref(),
                    memory,
                )
                .map_err(Error::RoleLicence)?;
            memory.release_vec(transitions)?;
            Ok(inputs)
        })
    }
    /// Bind actual checked input/inclusion captures. Multi-license schema guards
    /// still require their checked bound derivations, not an empty syntax set.
    pub(crate) fn syntax_roots(&self) -> Result<Vec<(crate::owl_dl::syntax::Space, usize)>, Error> {
        use crate::owl_dl::syntax::Space;
        let kb = self.kb;
        for &guard in &self.guards {
            if !self.schema.is_some_and(|schema| schema.matches(kb, guard)) {
                return Err(Error::MissingSchema(guard));
            }
        }
        let mut resident = Resident;
        let mut memory = Memory::new(&mut resident);
        let mut roots = Vec::new();
        for input in self.inputs() {
            poll(kb)?;
            let input = input?;
            if matches!(input.source, Source::KeyInference { .. }) {
                continue;
            }
            if let Some(root) = kb
                .inputs
                .syntax(input.input)
                .ok_or(Error::MissingSyntax)?
                .root()
            {
                memory.push(&mut roots, (Space::Source, root))?;
            }
        }
        let internalized = self.internalized.iter().map(|&(_, source)| source);
        let schema_inclusions = self.schema.iter().flat_map(|schema| {
            let bounds = schema.bounds();
            self.schema_reads
                .iter()
                .filter_map(|read| read.step(bounds).inclusion())
        });
        for index in self
            .tbox
            .iter()
            .copied()
            .chain(schema_inclusions)
            .chain(internalized)
        {
            poll(kb)?;
            let source = kb.tbox_sources.get(index).ok_or(Error::MissingSyntax)?;
            let (space, capture) = match source {
                AxiomSource::Unrecorded => return Err(Error::MissingSyntax),
                AxiomSource::EmptyRole(_) => continue,
                AxiomSource::QueryDefinition(_) => (Space::Query, kb.query_syntax.tbox_root(index)),
                _ => (Space::Source, kb.syntax.tbox_root(index)),
            };
            if let Some(root) = capture.ok_or(Error::MissingSyntax)?.root() {
                memory.push(&mut roots, (space, root))?;
            }
        }
        for support in self
            .keys
            .iter()
            .flat_map(|key| &key.members)
            .flat_map(key::Member::supports)
        {
            memory.extend(&mut roots, support.syntax.iter().copied())?;
        }
        Ok(roots)
    }
    /// Bind selected assertions and native role-rule predecessors. Expression,
    /// query and effect licenses remain separate graph obligations.
    pub(crate) fn inputs(&self) -> impl Iterator<Item = Result<SelectedInput, Error>> + '_ {
        let kb = self.kb;
        let facts = self
            .effects
            .iter()
            .map(|(_, event)| &event.cause)
            .filter_map(|&cause| {
                let input = match cause.origin {
                    Origin::PreparedType => Input::Type(cause.index),
                    Origin::PreparedRole => Input::Role(cause.index),
                    Origin::PreparedEquality => Input::Equality(cause.index),
                    Origin::PreparedDistinct => Input::Distinct(cause.index),
                    _ => return None,
                };
                Some((Some(cause), input))
            });
        facts
            .chain(self.rules.iter().map(|&input| (None, input)))
            .chain(
                self.keys
                    .iter()
                    .flat_map(|key| key.inputs().map(|input| (None, input))),
            )
            .chain(self.schema.iter().flat_map(|schema| {
                schema
                    .bounds()
                    .roles
                    .iter()
                    .map(|step| (None, step.input()))
            }))
            .map(|(cause, input)| match kb.inputs.get(input) {
                None | Some(Source::Anchor(AxiomSource::Unrecorded)) => {
                    Err(Error::MissingInput(input))
                }
                Some(source) => Ok(SelectedInput {
                    cause,
                    input,
                    source,
                }),
            })
            .chain(
                self.keys
                    .iter()
                    .flat_map(|key| &key.members)
                    .flat_map(key::Member::supports)
                    .flat_map(|support| support.inputs.iter().copied().map(Ok)),
            )
    }
}

#[derive(Default)]
struct Builder {
    facts: OrderedTable<Fact, usize>,
    applications: OrderedTable<usize, usize>,
    heads: OrderedTable<(usize, usize), usize>,
    aliases: OrderedTable<usize, (usize, usize)>,
    edges: OrderedTable<usize, (usize, usize, u32)>,
    nodes: Vec<Dependency>,
    parents: Vec<usize>,
    paths: Vec<PathStep>,
    path_ordinals: Vec<usize>,
    transitive: Vec<TransitiveRead>,
    transitive_nodes: Vec<TransitiveNode>,
    rules: Vec<Input>,
    tbox: Vec<usize>,
    guards: Vec<usize>,
    role_clauses: Vec<crate::owl_dl::roles::RoleClause>,
}

impl Builder {
    /// Follow the native hierarchy walk's retained predecessor, not another closure.
    fn role_rules(
        &mut self,
        requested: Role,
        mut actual: Role,
        parents: &OrderedTable<(Role, Role), (Role, Input)>,
        memory: &mut Memory<'_, Resident>,
    ) -> Result<(), Error> {
        while actual != requested {
            let &(parent, input) = parents
                .get(&(requested, actual))
                .ok_or(Error::MissingPath)?;
            memory.push(&mut self.rules, input)?;
            actual = parent;
        }
        Ok(())
    }
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
        events: &[Event],
        paths: &[PathStep],
        reach: &[reach::Path],
        rules: &OrderedTable<(Role, Role), (Role, Input)>,
        memory: &mut Memory<'_, Resident>,
    ) -> Result<(), Error> {
        match fact {
            ReadFact::RoleConstraint(input) => {
                memory.push(&mut self.rules, input)?;
                Ok(())
            }
            ReadFact::RoleClause(source) => {
                memory.push(&mut self.role_clauses, source)?;
                Ok(())
            }
            ReadFact::Tbox(index) => {
                memory.push(&mut self.tbox, index)?;
                Ok(())
            }
            ReadFact::SchemaGuard(index) => {
                memory.push(&mut self.guards, index)?;
                Ok(())
            }
            ReadFact::Alias { original, node } => {
                self.parent(Fact::Node(original), memory)?;
                self.parent(Fact::Node(node), memory)?;
                self.alias(original, node, memory)
            }
            ReadFact::Node(node) | ReadFact::Top(node) => self.parent(Fact::Node(node), memory),
            ReadFact::ObjectDomain(node) => self.parent(Fact::Domain(node), memory),
            ReadFact::Distinct { left, right } => self.parent(distinct(left, right), memory),
            ReadFact::Predecessor {
                child,
                parent,
                node,
                predecessor,
                role,
                letter,
            } => {
                let birth = |node| -> Result<Event, Error> {
                    let id = self
                        .facts
                        .get(&Fact::Node(node))
                        .ok_or(Error::MissingFact)?;
                    let ordinal = self.nodes[*id].event.ok_or(Error::MissingFact)?;
                    events.get(ordinal).copied().ok_or(Error::MissingFact)
                };
                let incoming = match letter {
                    Role::Named(p) => Role::Inv(p),
                    Role::Inv(p) => Role::Named(p),
                };
                if !matches!(birth(child)?.mutation, Mutation::Node { parent: Some(p), incoming: Some(r), reserved: false, .. } if p == parent && r == incoming)
                    || !matches!(
                        birth(predecessor)?.mutation,
                        Mutation::Node {
                            parent: Some(_),
                            reserved: false,
                            ..
                        }
                    )
                {
                    return Err(Error::MissingEffect);
                }
                self.parent(Fact::Node(child), memory)?;
                self.parent(Fact::Node(parent), memory)?;
                self.alias(child, node, memory)?;
                self.alias(parent, predecessor, memory)?;
                self.role_rules(role, letter, rules, memory)
            }
            ReadFact::Label { node, concept } => self.parent(Fact::Label(node, concept), memory),
            ReadFact::Denotes { node, individual } => {
                self.parent(Fact::Name(node, individual), memory)
            }
            ReadFact::Role {
                from,
                to,
                role,
                witness: RoleWitness::Edge { index, letter },
            } => {
                self.edge(index, letter, from, to, memory)?;
                self.role_rules(role, letter, rules, memory)
            }
            ReadFact::Role {
                witness: RoleWitness::Language { start, end },
                ..
            } => {
                for (offset, step) in paths
                    .get(start..end)
                    .ok_or(Error::MissingPath)?
                    .iter()
                    .enumerate()
                {
                    match step.kind {
                        PathKind::Edge { index, letter } => {
                            self.edge(index, letter, step.from.0, step.to.0, memory)?;
                        }
                        PathKind::Epsilon => self.parent(Fact::Node(step.from.0), memory)?,
                        PathKind::Universal(_) => {
                            self.parent(Fact::Domain(step.from.0), memory)?;
                            self.parent(Fact::Domain(step.to.0), memory)?;
                        }
                    }
                    memory.push(&mut self.paths, *step)?;
                    memory.push(&mut self.path_ordinals, start + offset)?;
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
                self.role_rules(role, via, rules, memory)?;
                let (Role::Named(property) | Role::Inv(property)) = via;
                memory.push(&mut self.rules, Input::Transitive(property))?;
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
                            self.role_rules(via, step.letter, rules, memory)?;
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
            ReadFact::Role {
                from,
                to,
                role,
                witness: RoleWitness::Universal(licence),
            } => {
                self.parent(Fact::Domain(from), memory)?;
                self.parent(Fact::Domain(to), memory)?;
                if let UniversalLicence::Hierarchy { via } = licence {
                    self.role_rules(role, via, rules, memory)?;
                }
                Ok(())
            }
        }
    }

    fn event(
        &mut self,
        ordinal: usize,
        event: Event,
        kb: &Kb,
        memory: &mut Memory<'_, Resident>,
    ) -> Result<(), Error> {
        let start = self.parents.len();
        let cause = event.cause;
        if cause.origin == Origin::Application {
            let &application = self
                .heads
                .get(&(cause.index, cause.atom))
                .or_else(|| self.applications.get(&cause.index))
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
                incoming,
                reserved,
                concrete,
            } => {
                // The native checker already validated this application and its
                // head. Retain the actual birth and original predecessor rather
                // than treating every generated successor as an opaque leaf.
                incomplete = if reserved {
                    parent.is_some()
                        || incoming.is_some()
                        || named.is_some()
                        || concrete
                        || cause.origin != Origin::Application
                        || !self.applications.get(&cause.index).is_some_and(|&id| {
                            matches!(
                                self.nodes[id].owner,
                                Some(ReadOwner::Application {
                                    branch: Some(_),
                                    ..
                                })
                            )
                        })
                } else {
                    parent.is_some() && cause.origin != Origin::Application
                };
                if let Some(parent) = parent {
                    self.parent(Fact::Node(parent), memory)?;
                }
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
                incomplete = cause.origin == Origin::PreparedEquality
                    && !matches!(
                        kb.inputs.get(Input::Equality(cause.index)),
                        Some(Source::Anchor(AxiomSource::Triple(..)) | Source::KeyInference { .. })
                    );
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
                incomplete = cause.origin == Origin::Application
                    && self.heads.get(&(cause.index, cause.atom)).is_none();
                self.parent(Fact::Node(left), memory)?;
                self.parent(Fact::Node(right), memory)?;
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
                event: Some(ordinal),
                owner: None,
                parents: start..self.parents.len(),
                reads: 0..0,
                paths: 0..0,
                transitive: 0..0,
                transitive_nodes: 0..0,
                rules: 0..0,
                tbox: 0..0,
                guards: 0..0,
                role_clauses: 0..0,
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
    #[cfg(test)]
    pub(crate) fn normalized_slice<'a>(&self, kb: &'a Kb) -> Result<Slice<'a>, Error> {
        self.normalized_slice_with_schema(kb, None)
    }

    pub(crate) fn normalized_slice_with_schema<'a>(
        &self,
        kb: &'a Kb,
        schema: Option<crate::owl_dl::proof::CheckedSchema<'a>>,
    ) -> Result<Slice<'a>, Error> {
        let mut storage = Resident;
        let mut memory = Memory::new(&mut storage);
        let mut graph = Builder::default();
        let mut event = 0;
        let mut read = 0;
        let mut root = None;
        while read < self.reads.len() {
            poll(kb)?;
            let selected = self.reads[read];
            while event < selected.checkpoint {
                poll(kb)?;
                graph.event(event, self.events[event], kb, &mut memory)?;
                event += 1;
            }
            let start = graph.parents.len();
            if let ReadOwner::Counting { application, .. }
            | ReadOwner::Identification { application, .. }
            | ReadOwner::Merge { application, .. } = selected.owner
            {
                let &body = graph
                    .applications
                    .get(&application)
                    .ok_or(Error::MissingFact)?;
                memory.push(&mut graph.parents, body)?;
            }
            let path_start = graph.paths.len();
            let transitive_start = graph.transitive.len();
            let transitive_nodes_start = graph.transitive_nodes.len();
            let rules_start = graph.rules.len();
            let tbox_start = graph.tbox.len();
            let guards_start = graph.guards.len();
            let role_clauses_start = graph.role_clauses.len();
            let mut incomplete = false;
            let read_start = read;
            while let Some(next) = self.reads.get(read)
                && next.owner == selected.owner
                && next.checkpoint == selected.checkpoint
            {
                match graph.read(
                    next.fact,
                    &self.events,
                    &self.paths,
                    &self.reach.paths,
                    &self.role_rules.0,
                    &mut memory,
                ) {
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
                    event: None,
                    owner: Some(selected.owner),
                    parents: start..graph.parents.len(),
                    reads: read_start..read,
                    paths: path_start..graph.paths.len(),
                    transitive: transitive_start..graph.transitive.len(),
                    transitive_nodes: transitive_nodes_start..graph.transitive_nodes.len(),
                    rules: rules_start..graph.rules.len(),
                    tbox: tbox_start..graph.tbox.len(),
                    guards: guards_start..graph.guards.len(),
                    role_clauses: role_clauses_start..graph.role_clauses.len(),
                    incomplete,
                },
            )?;
            match selected.owner {
                ReadOwner::Application { index, .. } => {
                    graph.applications.insert(index, id, &mut memory)?;
                }
                ReadOwner::Counting { application, atom }
                | ReadOwner::Identification { application, atom } => {
                    graph.heads.insert((application, atom), id, &mut memory)?;
                }
                ReadOwner::Clash { .. } => root = Some(id),
                ReadOwner::Merge { .. } => root = Some(id),
            }
        }
        let mut pending = Vec::new();
        let mut visited = Vec::new();
        memory.reserve(&mut visited, graph.nodes.len())?;
        visited.resize(graph.nodes.len(), false);
        let root = root.ok_or(Error::MissingClash)?;
        memory.push(&mut pending, root)?;
        let mut result = Slice {
            kb,
            derivation: Derivation {
                root,
                vertices: Vec::new(),
                parents: Vec::new(),
                reads: Vec::new(),
                births: Vec::new(),
            },
            effects: Vec::new(),
            owners: Vec::new(),
            paths: Vec::new(),
            path_ordinals: Vec::new(),
            transitive: Vec::new(),
            transitive_nodes: Vec::new(),
            rules: Vec::new(),
            tbox: Vec::new(),
            internalized: Vec::new(),
            guards: Vec::new(),
            role_clauses: Vec::new(),
            schema,
            schema_reads: Vec::new(),
            keys: Vec::new(),
        };
        while let Some(id) = pending.pop() {
            poll(kb)?;
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
            let observation = if let Some(ordinal) = node.event {
                let event = self.events[ordinal];
                let selected = result.effects.len();
                if let Mutation::Node { node, .. } = event.mutation {
                    memory.push(&mut result.derivation.births, (node, selected))?;
                }
                if event.cause.origin == Origin::Internalized {
                    let at = kb
                        .meta
                        .iter()
                        .position(|&concept| concept as usize == event.cause.index)
                        .ok_or(Error::MissingSyntax)?;
                    let source = kb
                        .meta_sources
                        .get(at)
                        .copied()
                        .ok_or(Error::MissingSyntax)?;
                    memory.push(&mut result.internalized, (kb.meta[at], source))?;
                }
                memory.push(&mut result.effects, (ordinal, event))?;
                Observation::Effect(selected)
            } else if let Some(owner) = node.owner {
                let selected = result.owners.len();
                memory.push(&mut result.owners, owner)?;
                Observation::Reads(selected)
            } else {
                return Err(Error::MissingFact);
            };
            let mut vertex = Vertex {
                id,
                observation,
                parents: result.derivation.parents.len()..0,
                reads: result.derivation.reads.len()..0,
                paths: result.paths.len()..0,
                transitive: result.transitive.len()..0,
                transitive_nodes: result.transitive_nodes.len()..0,
                rules: result.rules.len()..0,
                tbox: result.tbox.len()..0,
                guards: result.guards.len()..0,
                role_clauses: result.role_clauses.len()..0,
            };
            memory.extend(
                &mut result.derivation.reads,
                self.reads[node.reads.clone()].iter().copied(),
            )?;
            memory.extend(
                &mut result.paths,
                graph.paths[node.paths.clone()].iter().copied(),
            )?;
            memory.extend(
                &mut result.path_ordinals,
                graph.path_ordinals[node.paths.clone()].iter().copied(),
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
            memory.extend(
                &mut result.rules,
                graph.rules[node.rules.clone()].iter().copied(),
            )?;
            memory.extend(
                &mut result.tbox,
                graph.tbox[node.tbox.clone()].iter().copied(),
            )?;
            memory.extend(
                &mut result.guards,
                graph.guards[node.guards.clone()].iter().copied(),
            )?;
            memory.extend(
                &mut result.role_clauses,
                graph.role_clauses[node.role_clauses.clone()]
                    .iter()
                    .copied(),
            )?;
            for &parent in &graph.parents[node.parents.clone()] {
                if parent >= id {
                    return Err(Error::MissingFact);
                }
                memory.push(&mut pending, parent)?;
                memory.push(&mut result.derivation.parents, parent)?;
            }
            vertex.parents.end = result.derivation.parents.len();
            vertex.reads.end = result.derivation.reads.len();
            vertex.paths.end = result.paths.len();
            vertex.transitive.end = result.transitive.len();
            vertex.transitive_nodes.end = result.transitive_nodes.len();
            vertex.rules.end = result.rules.len();
            vertex.tbox.end = result.tbox.len();
            vertex.guards.end = result.guards.len();
            vertex.role_clauses.end = result.role_clauses.len();
            memory.push(&mut result.derivation.vertices, vertex)?;
        }
        result
            .derivation
            .vertices
            .sort_unstable_by_key(|vertex| vertex.id);
        result
            .derivation
            .births
            .sort_unstable_by_key(|&(node, _)| node);
        if let Some(schema) = result.schema
            && (result.guards.is_empty()
                || result
                    .guards
                    .iter()
                    .any(|&guard| !schema.matches(kb, guard)))
        {
            return Err(Error::UnexpectedSchema);
        }
        if let Some(schema) = result.schema {
            result.schema_reads = schema::select(kb, schema.bounds(), &mut memory)?;
        }
        let mut selected_keys = OrderedTable::default();
        for (_, event) in &result.effects {
            let cause = event.cause;
            poll(kb)?;
            if cause.origin == Origin::PreparedEquality
                && matches!(
                    kb.inputs.get(Input::Equality(cause.index)),
                    Some(Source::KeyInference { .. })
                )
                && selected_keys
                    .insert(cause.index, (), &mut memory)?
                    .is_none()
            {
                memory.push(&mut result.keys, key::select(kb, cause.index)?)?;
            }
        }
        Ok(result)
    }
}
