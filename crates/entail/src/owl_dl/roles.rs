// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The shared OWL object-property hierarchy and regular role-language compiler.
//!
//! Authored expressions remain signed role identities through the syntactic
//! regularity decision. In particular, an equivalent property is not silently
//! substituted for a chain's recursive endpoint before that decision.

use purrdf_core::TermValue;
use purrdf_core::collections::{ListErrorKind, RdfListWalk, SoleObject};
use purrdf_core::graph::{GraphError, scc_component_index_with_memory};
use purrdf_datalog::StopSignal;
use purrdf_lex::allocation::{Admission, Memory, StorageError};

use crate::interner::Interner;
use crate::owl_dl::concept::{ConceptTable, Decomp, Role};
use crate::owl_dl::parser::{TripleIndex, Vocab};

/// Original term Debug script with fallible native scratch and its first cause.
/// The output destination is admitted separately by `Memory::format`.
struct SourceTermDisplay<'a> {
    term: &'a TermValue,
    refusal: std::cell::Cell<Option<StorageError>>,
}

impl std::fmt::Display for SourceTermDisplay<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut resident = purrdf_lex::allocation::Resident;
        let mut memory = Memory::new(&mut resident);
        self.term
            .write_debug_with_memory(f, &mut memory)
            .map_err(|error| {
                if let purrdf_lex::walk::DebugWriteError::Storage(original) = error {
                    self.refusal.set(Some(original));
                }
                std::fmt::Error
            })
    }
}

/// A refused property hierarchy, before any reasoning certificate is produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoleHierarchyError {
    /// A semantic refusal together with original source-term spelling.
    Witness {
        /// Original typed classification and local signed-role identities.
        original: Box<Self>,
        /// Admitted diagnostic arguments, retained after the source interner dies.
        presentation: purrdf_lex::diagnostic::DiagnosticPresentation,
    },
    /// A chain collection is not a well-formed ordered RDF list.
    MalformedList {
        /// The broken collection's interned node.
        node: u32,
        /// The strict collection walker's original classification.
        kind: ListErrorKind,
    },
    /// A property expression is neither an IRI nor one anonymous inverse of an IRI.
    InvalidProperty {
        /// The offending expression's interned node.
        node: u32,
    },
    /// OWL role-inclusion chains have at least two operands.
    ShortChain {
        /// The property whose chain is malformed.
        head: u32,
    },
    /// A composite role occurs where OWL requires a simple object property.
    NonSimpleProperty {
        /// The restricted property's interned IRI.
        property: u32,
    },
    /// The required strict role order is cyclic or opposes a simple inclusion.
    OrderConflict {
        /// The lower property's interned IRI.
        lower: u32,
        /// Whether the lower expression is inverse.
        lower_inverse: bool,
        /// The upper property's interned IRI.
        upper: u32,
        /// Whether the upper expression is inverse.
        upper_inverse: bool,
    },
    /// Simple and complex dependencies cannot be stratified into role languages.
    RecursiveDependency {
        /// A composite property's interned IRI on the dependency cycle.
        property: u32,
    },
    /// A concrete destination or original admission refused before growth.
    Storage(StorageError),
    /// A malformed internal dense graph.
    Graph(GraphError),
    /// The caller's original cancellation signal was fired.
    Stopped,
}

purrdf_lex::variant_from!(RoleHierarchyError { Storage(StorageError) });

impl From<GraphError> for RoleHierarchyError {
    fn from(error: GraphError) -> Self {
        match error {
            GraphError::Storage(error) => Self::Storage(error),
            error @ GraphError::InvalidEdge { .. } => Self::Graph(error),
        }
    }
}

impl std::fmt::Display for RoleHierarchyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Witness { presentation, .. } => f.write_str(presentation.english()),
            Self::MalformedList { node, kind } => {
                write!(f, "malformed role-chain collection at term {node}: {kind}")
            }
            Self::InvalidProperty { node } => write!(f, "invalid object property at term {node}"),
            Self::ShortChain { head } => {
                write!(f, "role chain of term {head} has fewer than two operands")
            }
            Self::NonSimpleProperty { property } => write!(
                f,
                "non-simple object property at term {property} occurs in a simple-property axiom"
            ),
            Self::OrderConflict {
                lower,
                lower_inverse,
                upper,
                upper_inverse,
            } => write!(
                f,
                "role order requires {}term {lower} below {}term {upper}, contrary to the hierarchy",
                if *lower_inverse { "inverse " } else { "" },
                if *upper_inverse { "inverse " } else { "" },
            ),
            Self::RecursiveDependency { property } => write!(
                f,
                "role-language dependency cycle through composite term {property}"
            ),
            Self::Storage(error) => error.fmt(f),
            Self::Graph(error) => error.fmt(f),
            Self::Stopped => f.write_str("role-hierarchy construction cancelled"),
        }
    }
}

impl std::error::Error for RoleHierarchyError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Witness { original, .. } => Some(original.as_ref()),
            Self::Storage(error) => Some(error),
            Self::Graph(error) => Some(error),
            _ => None,
        }
    }
}

impl RoleHierarchyError {
    /// The original refusal kind, independent of its retained human presentation.
    #[must_use]
    pub fn classification(&self) -> &Self {
        match self {
            Self::Witness { original, .. } => original,
            original => original,
        }
    }

    /// Typed source spellings usable without access to the private term interner.
    #[must_use]
    pub fn presentation(&self) -> Option<&purrdf_lex::diagnostic::DiagnosticPresentation> {
        match self {
            Self::Witness { presentation, .. } => Some(presentation),
            _ => None,
        }
    }

    /// Retain a semantic witness through the original fallible diagnostic home.
    pub(crate) fn with_source(self, interner: &Interner) -> Self {
        use purrdf_lex::diagnostic::{
            DiagnosticParameter, DiagnosticPresentation, DiagnosticValue,
        };
        let terms = match &self {
            Self::MalformedList { node, .. } | Self::InvalidProperty { node } => {
                [Some(*node), None]
            }
            Self::ShortChain { head } => [Some(*head), None],
            Self::NonSimpleProperty { property } | Self::RecursiveDependency { property } => {
                [Some(*property), None]
            }
            Self::OrderConflict { lower, upper, .. } => [Some(*lower), Some(*upper)],
            Self::Witness { .. } | Self::Storage(_) | Self::Graph(_) | Self::Stopped => {
                return self;
            }
        };
        let mut resident = purrdf_lex::allocation::Resident;
        let mut memory = Memory::new(&mut resident);
        let result = memory.try_scope(|memory| {
            let mut parameters = Vec::new();
            let classification = DiagnosticValue::Text(memory.format(&self)?);
            let parameter =
                DiagnosticParameter::try_new_with_memory("classification", classification, memory)?;
            memory.push(&mut parameters, parameter)?;
            for (name, term) in ["first", "second"].into_iter().zip(terms) {
                if let Some(term) = term {
                    let display = SourceTermDisplay {
                        term: interner.value(term),
                        refusal: std::cell::Cell::new(None),
                    };
                    let spelling = memory
                        .format(&display)
                        .map_err(|error| display.refusal.get().unwrap_or(error))?;
                    let parameter = DiagnosticParameter::try_new_with_memory(
                        name,
                        DiagnosticValue::Text(spelling),
                        memory,
                    )?;
                    memory.push(&mut parameters, parameter)?;
                }
            }
            // These placeholders belong to the validated diagnostic template language.
            #[allow(clippy::literal_string_with_formatting_args)]
            let template = if terms[1].is_some() {
                "{classification}; original lower {first}, upper {second}"
            } else {
                "{classification}; original source {first}"
            };
            let presentation = DiagnosticPresentation::try_new_with_memory(
                "entail.role-hierarchy.source",
                template,
                parameters,
                memory,
            )
            .map_err(|error| error.fixed_template_storage())?;
            let original = memory.boxed(self)?;
            Ok::<_, StorageError>(Self::Witness {
                original,
                presentation,
            })
        });
        result.unwrap_or_else(Self::Storage)
    }
}

/// One authored inclusion, in collection order, without equivalence substitution.
#[derive(Debug)]
pub(crate) struct Chain {
    pub(crate) head: Role,
    pub(crate) body: Vec<Role>,
}

/// Source RBox syntax, retained in the same admission as the eventual automata.
#[derive(Debug, Default)]
pub(crate) struct Hierarchy {
    /// Ordered, authored complex inclusions, before equivalent-role normalization.
    pub(crate) chains: Vec<Chain>,
    /// Simple inclusions, including both directions of equivalence and inverses.
    pub(crate) inclusions: Vec<(Role, Role)>,
    /// Global reflexivity is allowed for composite roles too; it is not hasSelf.
    pub(crate) reflexive: Vec<Role>,
}

/// The source hierarchy and its least required strict-order witness.
///
/// These are separate relations: a legal equivalent-property component is cyclic
/// in `simple` but cannot contain a comparison in `strict`. Authored chain
/// endpoints have already been tested before either relation is closed.
#[derive(Debug)]
pub(crate) struct RoleOrder {
    pub(crate) roles: Vec<Role>,
    simple: RoleRelation,
    strict: RoleRelation,
}

impl RoleOrder {
    fn representatives<S: Admission + ?Sized>(
        &self,
        memory: &mut Memory<'_, S>,
    ) -> Result<Vec<usize>, StorageError> {
        memory.scope(|memory| {
            let mut roots =
                memory.collect(std::iter::repeat_n(usize::MAX, self.simple.reachable.len()))?;
            let mut representatives =
                memory.collect(std::iter::repeat_n(0usize, self.roles.len()))?;
            for position in 0..self.roles.len() {
                let component = self.simple.component[position];
                if roots[component] == usize::MAX {
                    roots[component] = position;
                }
                representatives[position] = roots[component];
            }
            memory.release_vec(roots)?;
            Ok(representatives)
        })
    }
    fn position(&self, role: Role) -> usize {
        self.roles
            .binary_search(&role)
            .expect("role belongs to the source hierarchy")
    }

    pub(crate) fn includes(&self, sub: Role, sup: Role) -> bool {
        self.simple
            .reaches(self.position(sub), self.position(sup), true)
    }

    pub(crate) fn release<S: Admission + ?Sized>(
        self,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), StorageError> {
        self.strict.release(memory)?;
        self.simple.release(memory)?;
        memory.release_vec(self.roles)
    }
}

impl Hierarchy {
    /// Least Horn closure on a *finite, supplied* graph, for proof checking and
    /// consequence saturation. This does not decide role regularity and is not
    /// the completion reader: it checks original inclusions directly, independent
    /// of the language compiler's state graph or traversal.
    pub(crate) fn close_edges<S, E>(
        &self,
        edges: &[(usize, usize, u32)],
        domain: (usize, Option<Role>),
        is_object: impl Fn(usize) -> bool,
        mut poll: impl FnMut() -> Result<(), E>,
        memory: &mut Memory<'_, S>,
    ) -> Result<Vec<(Role, usize, usize)>, E>
    where
        S: Admission + ?Sized,
        E: From<StorageError>,
    {
        memory.try_scope(|memory| {
            let (nodes, top) = domain;
            let mut relation = Vec::new();
            if let Some(top) = top {
                for from in 0..nodes {
                    for to in 0..nodes {
                        poll()?;
                        if is_object(from) && is_object(to) {
                            insert_edge(&mut relation, (top, from, to), memory)?;
                            insert_edge(&mut relation, (top.inverse(), from, to), memory)?;
                        }
                    }
                }
            }
            for &(from, to, property) in edges {
                poll()?;
                insert_edge(&mut relation, (Role::Named(property), from, to), memory)?;
                insert_edge(&mut relation, (Role::Inv(property), to, from), memory)?;
            }
            for &role in &self.reflexive {
                for node in 0..nodes {
                    poll()?;
                    if is_object(node) {
                        insert_edge(&mut relation, (role, node, node), memory)?;
                    }
                }
            }
            loop {
                let before = relation.len();
                for &(sub, sup) in &self.inclusions {
                    poll()?;
                    // Collect the original matching row before mutating the
                    // relation; newly discovered rows are handled by fixpoint.
                    let selected = memory.collect(
                        relation
                            .iter()
                            .filter(|&&(role, _, _)| role == sub)
                            .map(|&(_, from, to)| (from, to)),
                    )?;
                    for &(from, to) in &selected {
                        poll()?;
                        insert_edge(&mut relation, (sup, from, to), memory)?;
                    }
                    memory.release_vec(selected)?;
                }
                for chain in &self.chains {
                    for inverse in [false, true] {
                        let head = if inverse {
                            chain.head.inverse()
                        } else {
                            chain.head
                        };
                        for start in 0..nodes {
                            poll()?;
                            let mut current = Vec::new();
                            memory.push(&mut current, start)?;
                            for position in 0..chain.body.len() {
                                let member = if inverse {
                                    chain.body[chain.body.len() - 1 - position].inverse()
                                } else {
                                    chain.body[position]
                                };
                                let mut next = Vec::new();
                                for &from in &current {
                                    poll()?;
                                    let first = relation.partition_point(|&(role, subject, _)| {
                                        (role, subject) < (member, from)
                                    });
                                    for &(role, subject, target) in &relation[first..] {
                                        poll()?;
                                        if (role, subject) != (member, from) {
                                            break;
                                        }
                                        memory.push(&mut next, target)?;
                                    }
                                }
                                next.sort_unstable();
                                next.dedup();
                                memory.release_vec(current)?;
                                current = next;
                                if current.is_empty() {
                                    break;
                                }
                            }
                            for &target in &current {
                                poll()?;
                                insert_edge(&mut relation, (head, start, target), memory)?;
                                insert_edge(
                                    &mut relation,
                                    (head.inverse(), target, start),
                                    memory,
                                )?;
                            }
                            memory.release_vec(current)?;
                        }
                    }
                }
                if relation.len() == before {
                    return Ok(relation);
                }
            }
        })
    }
    /// Decide both the printed source-order condition and the theorem-backed
    /// mixed dependency condition, without constructing potentially much larger
    /// language machines. Profile certification and compilation share this law.
    pub(crate) fn validate<S: Admission + ?Sized>(
        &self,
        top: Option<Role>,
        stop: Option<&dyn StopSignal>,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), RoleHierarchyError> {
        memory.try_scope(|memory| {
            let order = self.source_order(top, stop, memory)?;
            let representatives = order.representatives(memory)?;
            self.check_dependencies(&order, &representatives, top, stop, memory)?;
            memory.release_vec(representatives)?;
            order.release(memory)?;
            Ok(())
        })
    }
    /// Compile the source-order witness and the actual regular role languages
    /// under one original memory scope. Witness matrices die before their scratch
    /// admission is released; only the returned program's buffers survive.
    pub(crate) fn compile<S: Admission + ?Sized>(
        &self,
        top: Option<Role>,
        stop: Option<&dyn StopSignal>,
        memory: &mut Memory<'_, S>,
    ) -> Result<RoleProgram, RoleHierarchyError> {
        let baseline = memory.admitted_bytes();
        memory.try_scope(|memory| {
            let order = self.source_order(top, stop, memory)?;
            let mut program = self.automata(&order, top, stop, memory)?;
            order.release(memory)?;
            program.admitted_bytes = memory
                .admitted_bytes()
                .checked_sub(baseline)
                .ok_or(StorageError::SizeOverflow)?;
            Ok(program)
        })
    }

    pub(crate) fn release<S: Admission + ?Sized>(
        self,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), StorageError> {
        let Self {
            mut chains,
            inclusions,
            reflexive,
        } = self;
        while let Some(chain) = chains.pop() {
            memory.release_vec(chain.body)?;
        }
        memory.release_vec(chains)?;
        memory.release_vec(inclusions)?;
        memory.release_vec(reflexive)
    }

    /// Decide the published OWL 2 Structural Specification §11.2 order condition.
    /// This source-order witness alone is not a role-language regularity theorem:
    /// the interaction of simple inclusions and chains is checked separately before
    /// the automaton producer is entered.
    pub(crate) fn source_order<S: Admission + ?Sized>(
        &self,
        top: Option<Role>,
        stop: Option<&dyn StopSignal>,
        memory: &mut Memory<'_, S>,
    ) -> Result<RoleOrder, RoleHierarchyError> {
        poll(stop)?;
        memory.try_scope(|memory| {
            let mut roles = Vec::new();
            for &(sub, sup) in &self.inclusions {
                memory.push(&mut roles, sub)?;
                memory.push(&mut roles, sup)?;
            }
            for chain in &self.chains {
                memory.push(&mut roles, chain.head)?;
                memory.push(&mut roles, chain.head.inverse())?;
                for &member in &chain.body {
                    memory.push(&mut roles, member)?;
                    memory.push(&mut roles, member.inverse())?;
                }
            }
            for &role in &self.reflexive {
                memory.push(&mut roles, role)?;
                memory.push(&mut roles, role.inverse())?;
            }
            roles.sort_unstable();
            roles.dedup();
            let count = roles.len();
            let mut simple = memory.collect(std::iter::repeat_with(Vec::new).take(count))?;
            let mut strict = memory.collect(std::iter::repeat_with(Vec::new).take(count))?;
            let position = |role| roles.binary_search(&role).expect("collected role");
            for &(sub, sup) in &self.inclusions {
                memory.push(&mut simple[position(sub)], position(sup))?;
            }
            if let Some(top) = top
                && roles.binary_search(&top).is_ok()
                && roles.binary_search(&top.inverse()).is_ok()
            {
                // The fixed universal relation is its own inverse. Its machine
                // is a distinguished graph-reader case, not an edge-language
                // approximation to universal connectivity.
                memory.push(&mut simple[position(top)], position(top.inverse()))?;
                memory.push(&mut simple[position(top.inverse())], position(top))?;
            }
            let simple = RoleRelation::close(simple, stop, memory)?;
            for chain in &self.chains {
                poll(stop)?;
                if Some(chain.head) == top || chain.body.as_slice() == [chain.head, chain.head] {
                    continue;
                }
                let first = usize::from(chain.body.first() == Some(&chain.head));
                let end = chain.body.len() - usize::from(chain.body.last() == Some(&chain.head));
                // Only one recursive endpoint can be omitted. The two-head
                // transitivity case was handled above; every other repeated head
                // induces the forbidden strict self comparison.
                let (first, end) = if first != 0 && end != chain.body.len() {
                    (0, chain.body.len())
                } else {
                    (first, end)
                };
                for &member in &chain.body[first..end] {
                    let upper = position(chain.head);
                    memory.push(&mut strict[position(member)], upper)?;
                    memory.push(&mut strict[position(member.inverse())], upper)?;
                }
            }
            let strict = RoleRelation::close(strict, stop, memory)?;
            for lower in 0..count {
                poll(stop)?;
                for upper in 0..count {
                    if strict.reaches(lower, upper, false) && simple.reaches(upper, lower, true) {
                        return Err(order_conflict(roles[lower], roles[upper]));
                    }
                }
            }
            Ok(RoleOrder {
                roles,
                simple,
                strict,
            })
        })
    }
}

fn insert_edge<S: Admission + ?Sized>(
    relation: &mut Vec<(Role, usize, usize)>,
    edge: (Role, usize, usize),
    memory: &mut Memory<'_, S>,
) -> Result<(), StorageError> {
    if let Err(position) = relation.binary_search(&edge) {
        memory.reserve_for_push(relation)?;
        relation.insert(position, edge);
    }
    Ok(())
}

fn poll(stop: Option<&dyn StopSignal>) -> Result<(), RoleHierarchyError> {
    if stop.is_some_and(StopSignal::stopped) {
        Err(RoleHierarchyError::Stopped)
    } else {
        Ok(())
    }
}

/// Role reachability over the shared Tarjan condensation. Sparse hierarchies
/// retain actual reachable component pairs, without a dense matrix or a cubic
/// closure over unrelated roles.
#[derive(Debug)]
struct RoleRelation {
    component: Vec<usize>,
    reachable: Vec<Vec<usize>>,
    cyclic: Vec<bool>,
}

impl RoleRelation {
    fn close<S: Admission + ?Sized>(
        adjacency: Vec<Vec<usize>>,
        stop: Option<&dyn StopSignal>,
        memory: &mut Memory<'_, S>,
    ) -> Result<Self, RoleHierarchyError> {
        poll(stop)?;
        let component = scc_component_index_with_memory(&adjacency, memory)?;
        poll(stop)?;
        let count = component.iter().max().map_or(0, |n| n + 1);
        let mut reachable = memory.collect(std::iter::repeat_with(Vec::new).take(count))?;
        let mut sizes = memory.collect(std::iter::repeat_n(0usize, count))?;
        let mut cyclic = memory.collect(std::iter::repeat_n(false, count))?;
        for (source, targets) in adjacency.iter().enumerate() {
            poll(stop)?;
            let from = component[source];
            sizes[from] += 1;
            for &target in targets {
                let to = component[target];
                if from == to {
                    cyclic[from] = true;
                } else {
                    memory.push(&mut reachable[from], to)?;
                }
            }
        }
        for (id, &size) in sizes.iter().enumerate() {
            cyclic[id] |= size > 1;
        }
        memory.release_vec(sizes)?;
        // Every outgoing target has a lower Tarjan component id. Its closure
        // is complete before the source, and the two rows are disjoint borrows.
        for id in 0..count {
            poll(stop)?;
            reachable[id].sort_unstable();
            reachable[id].dedup();
            let direct = reachable[id].len();
            for edge in 0..direct {
                let target = reachable[id][edge];
                let (lower, upper) = reachable.split_at_mut(id);
                for &ancestor in &lower[target] {
                    poll(stop)?;
                    memory.push(&mut upper[0], ancestor)?;
                }
            }
            reachable[id].sort_unstable();
            reachable[id].dedup();
        }
        release_rows(adjacency, memory)?;
        Ok(Self {
            component,
            reachable,
            cyclic,
        })
    }

    fn reaches(&self, from: usize, to: usize, reflexive: bool) -> bool {
        let source = self.component[from];
        let target = self.component[to];
        if source == target {
            reflexive || from != to || self.cyclic[source]
        } else {
            self.reachable[source].binary_search(&target).is_ok()
        }
    }

    fn release<S: Admission + ?Sized>(
        self,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), StorageError> {
        release_rows(self.reachable, memory)?;
        memory.release_vec(self.cyclic)?;
        memory.release_vec(self.component)
    }
}

fn release_rows<S: Admission + ?Sized>(
    mut rows: Vec<Vec<usize>>,
    memory: &mut Memory<'_, S>,
) -> Result<(), StorageError> {
    while let Some(row) = rows.pop() {
        memory.release_vec(row)?;
    }
    memory.release_vec(rows)
}

fn order_conflict(lower: Role, upper: Role) -> RoleHierarchyError {
    let (lower, lower_inverse) = match lower {
        Role::Named(id) => (id, false),
        Role::Inv(id) => (id, true),
    };
    let (upper, upper_inverse) = match upper {
        Role::Named(id) => (id, false),
        Role::Inv(id) => (id, true),
    };
    RoleHierarchyError::OrderConflict {
        lower,
        lower_inverse,
        upper,
        upper_inverse,
    }
}

/// A transition reads one original graph edge, or moves without reading an edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Transition {
    pub(crate) from: usize,
    pub(crate) to: usize,
    pub(crate) letter: Option<Role>,
}

/// One finite role-language machine. States are local dense indices; the final
/// program assigns their internal concept identities without minting RDF names.
#[derive(Debug)]
pub(crate) struct Automaton {
    pub(crate) states: usize,
    pub(crate) initial: usize,
    pub(crate) accepting: usize,
    pub(crate) transitions: Vec<Transition>,
}

impl Automaton {
    fn empty() -> Self {
        Self {
            states: 2,
            initial: 0,
            accepting: 1,
            transitions: Vec::new(),
        }
    }

    fn state(&mut self) -> Result<usize, StorageError> {
        let state = self.states;
        self.states = self
            .states
            .checked_add(1)
            .ok_or(StorageError::SizeOverflow)?;
        Ok(state)
    }

    fn transition<S: Admission + ?Sized>(
        &mut self,
        from: usize,
        to: usize,
        letter: Option<Role>,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), StorageError> {
        memory.push(&mut self.transitions, Transition { from, to, letter })
    }

    /// Add exactly this word between the supplied continuation states. Every
    /// interior state is fresh, so another chain cannot return into this word.
    fn word<S: Admission + ?Sized>(
        &mut self,
        from: usize,
        to: usize,
        word: &[Role],
        memory: &mut Memory<'_, S>,
    ) -> Result<(), StorageError> {
        if word.is_empty() {
            return self.transition(from, to, None, memory);
        }
        let mut previous = from;
        for (index, &role) in word.iter().enumerate() {
            let next = if index + 1 == word.len() {
                to
            } else {
                self.state()?
            };
            self.transition(previous, next, Some(role), memory)?;
            previous = next;
        }
        Ok(())
    }

    /// Substitute a disjoint copy at a lower-role transition. Sharing one copy
    /// among unrelated continuations would accept cross-return words, so the
    /// physical destination includes every copy before any transition is written.
    fn substitute<S: Admission + ?Sized>(
        &mut self,
        edge: Transition,
        lower: &Self,
        stop: Option<&dyn StopSignal>,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), RoleHierarchyError> {
        let offset = self.states;
        self.states = offset
            .checked_add(lower.states)
            .ok_or(StorageError::SizeOverflow)?;
        let required = self
            .transitions
            .len()
            .checked_add(lower.transitions.len())
            .and_then(|n| n.checked_add(2))
            .ok_or(StorageError::SizeOverflow)?;
        memory.reserve(&mut self.transitions, required)?;
        for transition in &lower.transitions {
            poll(stop)?;
            self.transitions.push(Transition {
                from: offset + transition.from,
                to: offset + transition.to,
                letter: transition.letter,
            });
        }
        self.transitions.push(Transition {
            from: edge.from,
            to: offset + lower.initial,
            letter: None,
        });
        self.transitions.push(Transition {
            from: offset + lower.accepting,
            to: edge.to,
            letter: None,
        });
        Ok(())
    }

    #[cfg(test)]
    fn release<S: Admission + ?Sized>(
        self,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), StorageError> {
        memory.release_vec(self.transitions)
    }
}

/// The compiled machines and the original signed-role lookup. Equivalent roles
/// share a machine, but distinct transition continuations never share a copy.
#[derive(Debug, Default)]
pub(crate) struct RoleProgram {
    /// Original authored inclusions, retained for the independent finite proof
    /// replay and consequence saturation rather than reconstructed from an NFA.
    pub(crate) source: Hierarchy,
    pub(crate) roles: Vec<Role>,
    pub(crate) machine_for: Vec<usize>,
    pub(crate) machines: Vec<Automaton>,
    pub(crate) top: Option<Role>,
    /// Composite roles and their inverse/super-role closure, from the original
    /// signed simple hierarchy. These are forbidden in simple-property axioms.
    pub(crate) non_simple: Vec<Role>,
    /// Every state/filler obligation introduced before search begins.
    pub(crate) obligations: Vec<Obligation>,
    /// Original dense concept IDs already inspected for universal restrictions.
    scanned_concepts: usize,
    /// Original native producer metadata for resuming this resident program.
    pub(crate) admitted_bytes: usize,
}

/// The initial and transition clauses for one authored universal restriction.
#[derive(Debug)]
pub(crate) struct Obligation {
    pub(crate) source: u32,
    pub(crate) machine: usize,
    pub(crate) filler: u32,
    pub(crate) states: Vec<u32>,
}

impl RoleProgram {
    /// Extend the finite blocking closure after query/negation-cache concept
    /// interning. Every state label is published before its obligation is visible;
    /// a refusal cannot publish a partly initialized state vector.
    pub(crate) fn extend_obligations<S: Admission + ?Sized>(
        &mut self,
        table: &mut ConceptTable,
        stop: Option<&dyn StopSignal>,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), RoleHierarchyError> {
        while self.scanned_concepts < table.len() {
            poll(stop)?;
            let source =
                u32::try_from(self.scanned_concepts).map_err(|_| StorageError::SizeOverflow)?;
            if let Decomp::All(role, filler) = *table.decomp(source)
                && let Ok(position) = self.roles.binary_search(&role)
            {
                let machine = self.machine_for[position];
                let states = memory.try_scope(|memory| {
                    let mut states = Vec::new();
                    memory.reserve(&mut states, self.machines[machine].states)?;
                    for state in 0..self.machines[machine].states {
                        poll(stop)?;
                        states.push(table.try_role_state(machine, state, filler)?);
                    }
                    Ok::<_, RoleHierarchyError>(states)
                })?;
                let bytes = states
                    .capacity()
                    .checked_mul(size_of::<u32>())
                    .ok_or(StorageError::SizeOverflow)?;
                if let Err(original) = memory.push(
                    &mut self.obligations,
                    Obligation {
                        source,
                        machine,
                        filler,
                        states,
                    },
                ) {
                    // The unpublished record died in push before this refund.
                    // Any replacement of the retained outer array stays covered.
                    let _ = memory.release_bytes(bytes);
                    return Err(original.into());
                }
            }
            self.scanned_concepts += 1;
        }
        Ok(())
    }

    pub(crate) fn machine(&self, role: Role) -> Option<&Automaton> {
        let position = self.roles.binary_search(&role).ok()?;
        self.machines.get(self.machine_for[position])
    }

    #[cfg(test)]
    pub(crate) fn release<S: Admission + ?Sized>(
        self,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), StorageError> {
        let Self {
            source,
            roles,
            machine_for,
            mut machines,
            top: _,
            non_simple,
            mut obligations,
            scanned_concepts: _,
            admitted_bytes: _,
        } = self;
        source.release(memory)?;
        while let Some(obligation) = obligations.pop() {
            memory.release_vec(obligation.states)?;
        }
        memory.release_vec(obligations)?;
        memory.release_vec(non_simple)?;
        while let Some(machine) = machines.pop() {
            machine.release(memory)?;
        }
        memory.release_vec(machines)?;
        memory.release_vec(machine_for)?;
        memory.release_vec(roles)
    }
}

impl Hierarchy {
    /// Produce role automata only after a theorem-backed dependency check. The
    /// source-order relation remains separate and available to profile reporting.
    fn automata<S: Admission + ?Sized>(
        &self,
        order: &RoleOrder,
        top: Option<Role>,
        stop: Option<&dyn StopSignal>,
        memory: &mut Memory<'_, S>,
    ) -> Result<RoleProgram, RoleHierarchyError> {
        memory.try_scope(|memory| {
            let count = order.roles.len();
            let representative = order.representatives(memory)?;
            self.check_dependencies(order, &representative, top, stop, memory)?;
            let mut program = RoleProgram {
                source: Self::default(),
                roles: memory.collect(order.roles.iter().copied())?,
                machine_for: memory.collect(std::iter::repeat_n(usize::MAX, count))?,
                machines: Vec::new(),
                top,
                non_simple: memory.collect(order.roles.iter().copied().filter(|&role| {
                    top.is_some_and(|top| {
                        (order.roles.binary_search(&top).is_ok() && order.includes(top, role))
                            || (order.roles.binary_search(&top.inverse()).is_ok()
                                && order.includes(top.inverse(), role))
                    }) || self.chains.iter().any(|chain| {
                        order.includes(chain.head, role)
                            || order.includes(chain.head.inverse(), role)
                    })
                }))?,
                obligations: Vec::new(),
                scanned_concepts: 0,
                admitted_bytes: 0,
            };
            let mut remaining = representative
                .iter()
                .enumerate()
                .filter(|&(position, &root)| position == root)
                .count();
            while remaining != 0 {
                poll(stop)?;
                let before = remaining;
                for root in 0..count {
                    if representative[root] != root || program.machine_for[root] != usize::MAX {
                        continue;
                    }
                    if !self.dependencies_ready(root, &representative, &program, order, top) {
                        continue;
                    }
                    let machine = self.component_automaton(
                        root,
                        &program,
                        (order, &representative),
                        top,
                        stop,
                        memory,
                    )?;
                    let id = program.machines.len();
                    memory.push(&mut program.machines, machine)?;
                    for (position, &component) in representative.iter().enumerate() {
                        if component == root {
                            program.machine_for[position] = id;
                        }
                    }
                    remaining -= 1;
                }
                if remaining == before {
                    let role = order.roles[program
                        .machine_for
                        .iter()
                        .position(|&id| id == usize::MAX)
                        .expect("unbuilt dependency")];
                    let property = match role {
                        Role::Named(id) | Role::Inv(id) => id,
                    };
                    return Err(RoleHierarchyError::RecursiveDependency { property });
                }
            }
            memory.release_vec(representative)?;
            Ok(program)
        })
    }

    fn dependencies_ready(
        &self,
        root: usize,
        representatives: &[usize],
        program: &RoleProgram,
        order: &RoleOrder,
        top: Option<Role>,
    ) -> bool {
        let head = order.roles[root];
        if Some(head) == top {
            return true;
        }
        for (position, &sub) in order.roles.iter().enumerate() {
            if representatives[position] != root
                && order.includes(sub, head)
                && program.machine_for[position] == usize::MAX
            {
                return false;
            }
        }
        for chain in &self.chains {
            if Some(chain.head) == top {
                continue;
            }
            for inverse in [false, true] {
                let signed = if inverse {
                    chain.head.inverse()
                } else {
                    chain.head
                };
                if representatives[order.position(signed)] != root {
                    continue;
                }
                for &member in &chain.body {
                    let member = if inverse { member.inverse() } else { member };
                    let position = order.position(member);
                    if representatives[position] != root
                        && program.machine_for[position] == usize::MAX
                    {
                        return false;
                    }
                }
            }
        }
        true
    }

    /// The original regular-calculus dependency order, after genuine simple
    /// equivalences have been quotiented. Inverse operands contribute the same
    /// dependency as their originals. A simple cycle by itself is not a reason to
    /// refuse an ontology; a remaining cycle through a complex head is.
    /// This is the normalized mixed dependency condition of Stefanoni (2015),
    /// Chapter 12, Theorem 12.4, not a claim that the printed source order alone
    /// guarantees a regular language (Example 12.2 disproves that implication).
    fn check_dependencies<S: Admission + ?Sized>(
        &self,
        order: &RoleOrder,
        representatives: &[usize],
        top: Option<Role>,
        stop: Option<&dyn StopSignal>,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), RoleHierarchyError> {
        memory.try_scope(|memory| {
            let count = order.roles.len();
            let mut dependency = memory.collect(std::iter::repeat_with(Vec::new).take(count))?;
            let mut add =
                |sub: Role, sup: Role, memory: &mut Memory<'_, S>| -> Result<(), StorageError> {
                    let target = representatives[order.position(sup)];
                    for source in [sub, sub.inverse()] {
                        let source = representatives[order.position(source)];
                        if source != target {
                            memory.push(&mut dependency[source], target)?;
                        }
                    }
                    Ok(())
                };
            for &(sub, sup) in &self.inclusions {
                let target = representatives[order.position(sup)];
                if Some(sup) != top
                    && representatives[order.position(sub)] != target
                    && representatives[order.position(sub.inverse())] != target
                {
                    add(sub, sup, memory)?;
                }
            }
            for chain in &self.chains {
                if Some(chain.head) == top {
                    continue;
                }
                for &member in &chain.body {
                    if member != chain.head && member.inverse() != chain.head {
                        add(member, chain.head, memory)?;
                        add(member.inverse(), chain.head.inverse(), memory)?;
                    }
                }
            }
            let dependency = RoleRelation::close(dependency, stop, memory)?;
            for chain in &self.chains {
                poll(stop)?;
                if Some(chain.head) == top || chain.body.as_slice() == [chain.head, chain.head] {
                    continue;
                }
                for role in [chain.head, chain.head.inverse()] {
                    let root = representatives[order.position(role)];
                    if dependency.reaches(root, root, false) {
                        let property = match role {
                            Role::Named(id) | Role::Inv(id) => id,
                        };
                        return Err(RoleHierarchyError::RecursiveDependency { property });
                    }
                }
            }
            dependency.release(memory)?;
            Ok(())
        })
    }

    fn component_automaton<S: Admission + ?Sized>(
        &self,
        root: usize,
        lower: &RoleProgram,
        component: (&RoleOrder, &[usize]),
        top: Option<Role>,
        stop: Option<&dyn StopSignal>,
        memory: &mut Memory<'_, S>,
    ) -> Result<Automaton, RoleHierarchyError> {
        let (order, representatives) = component;
        let head = order.roles[root];
        let mut machine = Automaton::empty();
        if Some(head) == top {
            return Ok(machine);
        }
        for &sub in &order.roles {
            if order.includes(sub, head) {
                machine.transition(machine.initial, machine.accepting, Some(sub), memory)?;
            }
        }
        if self
            .reflexive
            .iter()
            .any(|&role| order.includes(role, head))
        {
            machine.transition(machine.initial, machine.accepting, None, memory)?;
        }
        for chain in &self.chains {
            if Some(chain.head) == top {
                continue;
            }
            for inverse in [false, true] {
                poll(stop)?;
                let signed = if inverse {
                    chain.head.inverse()
                } else {
                    chain.head
                };
                if representatives[order.position(signed)] != root {
                    continue;
                }
                if inverse {
                    let body = memory.collect(chain.body.iter().rev().map(|r| r.inverse()))?;
                    machine.inclusion(signed, &body, memory)?;
                    memory.release_vec(body)?;
                } else {
                    machine.inclusion(signed, &chain.body, memory)?;
                }
            }
        }
        // Copies already contain their own complete lower substitutions. Iterate
        // only the original transitions, not the freshly copied graph.
        let original = machine.transitions.len();
        for index in 0..original {
            poll(stop)?;
            let edge = machine.transitions[index];
            if let Some(letter) = edge.letter
                && representatives[order.position(letter)] != root
            {
                machine.substitute(
                    edge,
                    lower.machine(letter).expect("ready lower role"),
                    stop,
                    memory,
                )?;
            }
        }
        machine
            .transitions
            .sort_unstable_by_key(|e| (e.from, e.letter, e.to));
        machine.transitions.dedup();
        Ok(machine)
    }
}

impl Automaton {
    /// Endpoint recursion has already passed the literal source check. Leading
    /// recursion loops at the final state; trailing recursion loops at the initial
    /// state. The transitivity loop joins the final and initial states by epsilon.
    fn inclusion<S: Admission + ?Sized>(
        &mut self,
        head: Role,
        body: &[Role],
        memory: &mut Memory<'_, S>,
    ) -> Result<(), StorageError> {
        if body == [head, head] {
            self.transition(self.accepting, self.initial, None, memory)
        } else if body.first() == Some(&head) {
            self.word(self.accepting, self.accepting, &body[1..], memory)
        } else if body.last() == Some(&head) {
            self.word(self.initial, self.initial, &body[..body.len() - 1], memory)
        } else {
            self.word(self.initial, self.accepting, body, memory)
        }
    }
}

/// Decode role expressions without treating a collection node as an individual.
pub(crate) struct RoleReader<'a> {
    pub(crate) index: &'a TripleIndex,
    pub(crate) interner: &'a Interner,
    pub(crate) vocab: &'a Vocab,
    pub(crate) stop: Option<&'a dyn StopSignal>,
}

impl RoleReader<'_> {
    fn poll(&self) -> Result<(), RoleHierarchyError> {
        poll(self.stop)
    }

    fn non_object(&self, node: u32) -> bool {
        self.index
            .get(&node)
            .and_then(|predicates| predicates.get(&self.vocab.ty))
            .is_some_and(|types| {
                types.contains(&self.vocab.datatype_property)
                    || types.contains(&self.vocab.annotation_property)
            })
    }

    /// The exact two object-property-expression forms of the OWL RDF mapping.
    pub(crate) fn role(&self, node: u32) -> Result<Role, RoleHierarchyError> {
        self.poll()?;
        match self.interner.value(node) {
            TermValue::Iri(_) if !self.non_object(node) => Ok(Role::Named(node)),
            TermValue::Blank { .. } => {
                let targets = self
                    .index
                    .get(&node)
                    .and_then(|predicates| predicates.get(&self.vocab.inverse_of));
                match SoleObject::of(targets.into_iter().flatten().copied()) {
                    SoleObject::One(target)
                        if matches!(self.interner.value(target), TermValue::Iri(_))
                            && !self.non_object(target) =>
                    {
                        Ok(Role::Inv(target))
                    }
                    _ => Err(RoleHierarchyError::InvalidProperty { node }),
                }
            }
            _ => Err(RoleHierarchyError::InvalidProperty { node }),
        }
    }

    /// Read all RBox syntax through the same signed-property and strict-list doors.
    pub(crate) fn hierarchy<S: Admission + ?Sized>(
        &self,
        memory: &mut Memory<'_, S>,
    ) -> Result<Hierarchy, RoleHierarchyError> {
        memory.try_scope(|memory| {
            let mut out = Hierarchy::default();
            for (&subject, predicates) in self.index {
                self.poll()?;
                if let Some(lists) = self.vocab.property_chain.and_then(|p| predicates.get(&p)) {
                    for &list in lists {
                        let chain = self.chain(subject, list, memory)?;
                        memory.push(&mut out.chains, chain)?;
                    }
                }
                for (predicate, equivalent, inverse) in [
                    (self.vocab.sub_prop, false, false),
                    (self.vocab.equiv_prop, true, false),
                    (self.vocab.inverse_of, true, true),
                ] {
                    let Some(objects) = predicates.get(&predicate) else {
                        continue;
                    };
                    // An anonymous inverse expression is structural syntax, not
                    // an additional global inverse-property axiom.
                    if inverse && matches!(self.interner.value(subject), TermValue::Blank { .. }) {
                        continue;
                    }
                    if !inverse && self.non_object(subject) {
                        continue;
                    }
                    let sub = self.role(subject)?;
                    for &object in objects {
                        self.poll()?;
                        if !inverse && self.non_object(object) {
                            continue;
                        }
                        let mut sup = self.role(object)?;
                        if inverse {
                            sup = sup.inverse();
                        }
                        add_inclusion(&mut out.inclusions, sub, sup, memory)?;
                        if equivalent {
                            add_inclusion(&mut out.inclusions, sup, sub, memory)?;
                        }
                    }
                }
                if let Some(types) = predicates.get(&self.vocab.ty) {
                    if types.contains(&self.vocab.transitive) {
                        let head = self.role(subject)?;
                        let body = memory.collect([head, head])?;
                        memory.push(&mut out.chains, Chain { head, body })?;
                    }
                    if types.contains(&self.vocab.symmetric) {
                        let role = self.role(subject)?;
                        add_inclusion(&mut out.inclusions, role, role.inverse(), memory)?;
                    }
                    if types.contains(&self.vocab.reflexive) {
                        let role = self.role(subject)?;
                        memory.push(&mut out.reflexive, role)?;
                        memory.push(&mut out.reflexive, role.inverse())?;
                    }
                }
            }
            out.inclusions.sort_unstable();
            out.inclusions.dedup();
            out.reflexive.sort_unstable();
            out.reflexive.dedup();
            Ok(out)
        })
    }

    /// Map the original strict list iterator directly into the admitted role array.
    pub(crate) fn chain<S: Admission + ?Sized>(
        &self,
        head: u32,
        list: u32,
        memory: &mut Memory<'_, S>,
    ) -> Result<Chain, RoleHierarchyError> {
        memory.try_scope(|memory| {
            let role = self.role(head)?;
            let objects = |cell: u32, predicate: u32| {
                SoleObject::of(
                    self.index
                        .get(&cell)
                        .and_then(|predicates| predicates.get(&predicate))
                        .into_iter()
                        .flatten()
                        .copied(),
                )
            };
            let members = RdfListWalk::new(
                list,
                Some(self.vocab.nil),
                |cell| objects(cell, self.vocab.first),
                |cell| objects(cell, self.vocab.rest),
            );
            let mut body = Vec::new();
            for member in members {
                self.poll()?;
                let member = member.map_err(|error| RoleHierarchyError::MalformedList {
                    node: error.node,
                    kind: error.kind,
                })?;
                memory.push(&mut body, self.role(member)?)?;
            }
            if body.len() < 2 {
                return Err(RoleHierarchyError::ShortChain { head });
            }
            Ok(Chain { head: role, body })
        })
    }
}

/// Every simple inclusion entails its inverse inclusion; no alias changes orientation.
fn add_inclusion<S: Admission + ?Sized>(
    inclusions: &mut Vec<(Role, Role)>,
    sub: Role,
    sup: Role,
    memory: &mut Memory<'_, S>,
) -> Result<(), StorageError> {
    memory.push(inclusions, (sub, sup))?;
    memory.push(inclusions, (sub.inverse(), sup.inverse()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use purrdf_lex::allocation::Resident;

    fn hierarchy() -> Hierarchy {
        Hierarchy {
            chains: vec![
                Chain {
                    head: Role::Named(3),
                    body: vec![Role::Named(1), Role::Named(2)],
                },
                Chain {
                    head: Role::Named(3),
                    body: vec![Role::Named(3), Role::Named(2)],
                },
            ],
            ..Hierarchy::default()
        }
    }

    /// Enumerate strict partial orders independently of SCC condensation and
    /// compare the printed source-order decision over all small simple graphs.
    #[test]
    fn source_order_matches_an_independent_partial_order_enumerator() {
        let pairs = [(0usize, 1usize), (0, 2), (1, 0), (1, 2), (2, 0), (2, 1)];
        for hierarchy_bits in 0..64 {
            let mut simple = [[false; 3]; 3];
            let mut inclusions = Vec::new();
            for (bit, &(from, to)) in pairs.iter().enumerate() {
                if hierarchy_bits & (1 << bit) != 0 {
                    simple[from][to] = true;
                    inclusions.push((Role::Named(from as u32), Role::Named(to as u32)));
                    inclusions.push((Role::Inv(from as u32), Role::Inv(to as u32)));
                }
            }
            for middle in 0..3 {
                for from in 0..3 {
                    for to in 0..3 {
                        simple[from][to] |= simple[from][middle] && simple[middle][to];
                    }
                }
            }
            for left in 0..3 {
                for right in 0..3 {
                    let head = 2;
                    let body = [left, right];
                    let required: Vec<_> = if body == [head, head] {
                        Vec::new()
                    } else {
                        body.into_iter().filter(|&role| role != head).collect()
                    };
                    let expected = (0..64).any(|order_bits| {
                        let mut order = [[false; 3]; 3];
                        for (bit, &(from, to)) in pairs.iter().enumerate() {
                            order[from][to] = order_bits & (1 << bit) != 0;
                        }
                        required.iter().all(|&role| order[role][head])
                            && (0..3).all(|from| {
                                (0..3).all(|to| {
                                    (!order[from][to] || !simple[to][from])
                                        && (0..3).all(|middle| {
                                            !order[from][middle]
                                                || !order[middle][to]
                                                || order[from][to]
                                        })
                                })
                            })
                    });
                    let source = Hierarchy {
                        chains: vec![Chain {
                            head: Role::Named(head as u32),
                            body: body
                                .into_iter()
                                .map(|role| Role::Named(role as u32))
                                .collect(),
                        }],
                        inclusions: inclusions.clone(),
                        ..Hierarchy::default()
                    };
                    let mut resident = Resident;
                    let mut memory = Memory::new(&mut resident);
                    let actual = source.source_order(None, None, &mut memory);
                    assert_eq!(
                        actual.is_ok(),
                        expected,
                        "hierarchy={hierarchy_bits} body={body:?}"
                    );
                    if let Ok(order) = actual {
                        order.release(&mut memory).expect("original order release");
                    }
                    assert_eq!(memory.admitted_bytes(), 0);
                }
            }
        }
    }

    /// The oracle is deliberately a finite relation rewrite, with no NFA states
    /// or dependency-compiler helpers. It tests compiled languages, not whether
    /// an infinite completion graph has a model.
    fn oracle(edges: &[(usize, usize, u32)]) -> [[[bool; 3]; 3]; 4] {
        let mut relation = [[[false; 3]; 3]; 4];
        for &(from, to, role) in edges {
            relation[role as usize][from][to] = true;
        }
        loop {
            let previous = relation;
            for from in 0..3 {
                for to in 0..3 {
                    for middle in 0..3 {
                        relation[3][from][to] |= (previous[1][from][middle]
                            || previous[3][from][middle])
                            && previous[2][middle][to];
                    }
                }
            }
            if relation == previous {
                return relation;
            }
        }
    }

    fn language(
        machine: &Automaton,
        edges: &[(usize, usize, u32)],
        from: usize,
        to: usize,
    ) -> bool {
        let mut seen = vec![false; 3 * machine.states];
        let mut work = vec![(from, machine.initial)];
        while let Some((node, state)) = work.pop() {
            if std::mem::replace(&mut seen[node * machine.states + state], true) {
                continue;
            }
            if node == to && state == machine.accepting {
                return true;
            }
            for transition in &machine.transitions {
                if transition.from != state {
                    continue;
                }
                match transition.letter {
                    None => work.push((node, transition.to)),
                    Some(Role::Named(role)) => {
                        for &(subject, object, predicate) in edges {
                            if predicate == role && subject == node {
                                work.push((object, transition.to));
                            }
                        }
                    }
                    Some(Role::Inv(role)) => {
                        for &(subject, object, predicate) in edges {
                            if predicate == role && object == node {
                                work.push((subject, transition.to));
                            }
                        }
                    }
                }
            }
        }
        false
    }

    #[test]
    fn compiled_signed_languages_match_independent_exhaustive_finite_closure() {
        let source = hierarchy();
        let mut resident = Resident;
        let mut memory = Memory::new(&mut resident);
        let program = source
            .compile(None, None, &mut memory)
            .expect("regular hierarchy");
        // Enumerate every graph over nine selected directed labelled edges,
        // including self edges and cycles. Every endpoint pair is checked.
        let candidates = [
            (0, 0, 1),
            (0, 1, 1),
            (1, 2, 1),
            (2, 0, 1),
            (0, 1, 2),
            (1, 1, 2),
            (1, 2, 2),
            (2, 0, 2),
            (2, 2, 2),
        ];
        for mask in 0..512 {
            let edges: Vec<_> = candidates
                .iter()
                .copied()
                .enumerate()
                .filter_map(|(bit, edge)| ((mask & (1 << bit)) != 0).then_some(edge))
                .collect();
            let expected = oracle(&edges);
            for (from, row) in expected[3].iter().enumerate() {
                for (to, &expected) in row.iter().enumerate() {
                    assert_eq!(
                        language(program.machine(Role::Named(3)).unwrap(), &edges, from, to),
                        expected,
                        "mask={mask}, {from}->{to}"
                    );
                    assert_eq!(
                        language(program.machine(Role::Inv(3)).unwrap(), &edges, to, from),
                        expected,
                        "inverse mask={mask}, {to}->{from}"
                    );
                }
            }
        }
        program.release(&mut memory).unwrap();
        assert_eq!(memory.admitted_bytes(), 0);
    }

    #[derive(Default)]
    struct Account {
        live: usize,
        peak: usize,
        grows: usize,
        refuse: Option<usize>,
    }
    impl Admission for Account {
        fn resize(&mut self, bytes: usize) -> Result<(), StorageError> {
            if bytes > self.live {
                self.grows += 1;
                if self.refuse == Some(self.grows) {
                    return Err(StorageError::AdmissionFailed);
                }
            }
            self.live = bytes;
            self.peak = self.peak.max(bytes);
            Ok(())
        }
    }

    #[test]
    fn every_compiler_growth_refusal_releases_the_original_scratch() {
        let source = hierarchy();
        let mut healthy = Account::default();
        let measured;
        {
            let mut memory = Memory::new(&mut healthy);
            let window = purrdf_alloc_probe::CurrentThreadWindow::open();
            let program = source
                .compile(None, None, &mut memory)
                .expect("healthy compiler");
            measured = window.close();
            assert_eq!(
                measured.retained_bytes,
                i64::try_from(memory.admitted_bytes()).unwrap()
            );
            program
                .release(&mut memory)
                .expect("destruction before refund");
        }
        assert!(measured.peak_working_bytes <= i64::try_from(healthy.peak).unwrap());
        assert_eq!(healthy.live, 0);
        for growth in 1..=healthy.grows {
            let mut account = Account {
                refuse: Some(growth),
                ..Account::default()
            };
            let mut memory = Memory::new(&mut account);
            assert!(matches!(
                source.compile(None, None, &mut memory),
                Err(RoleHierarchyError::Storage(StorageError::AdmissionFailed))
            ));
            assert_eq!(memory.admitted_bytes(), 0, "growth {growth}");
            assert_eq!(account.live, 0, "growth {growth}");
        }
        let mut account = Account {
            refuse: Some(1),
            ..Account::default()
        };
        let mut memory = Memory::new(&mut account);
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        let result = source.compile(None, None, &mut memory);
        let measured = window.close();
        assert!(matches!(
            result,
            Err(RoleHierarchyError::Storage(StorageError::AdmissionFailed))
        ));
        assert_eq!(measured.allocations, 0);
        assert_eq!(account.live, 0);
    }

    #[test]
    fn long_regular_words_use_iterative_admitted_construction_without_a_chain_limit() {
        for length in [16, 256, 4096] {
            let source = Hierarchy {
                chains: vec![Chain {
                    head: Role::Named(2),
                    body: vec![Role::Named(1); length],
                }],
                ..Hierarchy::default()
            };
            let mut account = Account::default();
            let mut memory = Memory::new(&mut account);
            let program = source
                .compile(None, None, &mut memory)
                .expect("regular long word");
            let machine = program.machine(Role::Named(2)).unwrap();
            assert!(machine.states >= length);
            assert!(machine.states <= 8 * length + 2, "linear word construction");
            program.release(&mut memory).unwrap();
            assert_eq!(memory.admitted_bytes(), 0);
            assert_eq!(account.live, 0);
        }
    }

    #[test]
    fn state_overflow_and_original_cancellation_are_distinct_from_regularity() {
        let mut machine = Automaton::empty();
        machine.states = usize::MAX;
        assert_eq!(machine.state(), Err(StorageError::SizeOverflow));
        assert_eq!(machine.states, usize::MAX);
        #[derive(Debug)]
        struct Cancel;
        impl StopSignal for Cancel {
            fn stopped(&self) -> bool {
                true
            }
        }
        let source = hierarchy();
        let mut account = Account::default();
        let mut memory = Memory::new(&mut account);
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        let result = source.compile(None, Some(&Cancel), &mut memory);
        let measured = window.close();
        assert!(matches!(result, Err(RoleHierarchyError::Stopped)));
        assert_eq!(measured.allocations, 0);
        assert_eq!(memory.admitted_bytes(), 0);
        assert_eq!(account.live, 0);
    }
}
