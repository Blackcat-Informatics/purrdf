// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native mutation observations for independent replay, not a completed causal slice.
//!
//! Source ordinals refer to the consumer's prepared arrays. They are not asserted
//! RDF premises until their original parser/normalization licenses are resolved.

use super::super::concept::Role;
use super::super::input_source::Input;
pub(crate) mod reach;
pub(crate) mod slice;
use purrdf_lex::allocation::{Memory, OrderedTable, Resident, StorageError};
use std::fmt;

#[derive(Default)]
struct RoleRules(OrderedTable<(Role, Role), (Role, Input)>);

impl fmt::Debug for RoleRules {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.0.iter()).finish()
    }
}

/// Only the independently checked binding owns these reads, never search candidates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ReadOwner {
    Application { index: usize, clause: usize },
    Clash { clause: usize },
}

/// The native fact selected by a successful body predicate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ReadFact {
    Tbox(usize),
    SchemaGuard(usize),
    RoleClause(crate::owl_dl::roles::RoleClause),
    RoleConstraint(Input),
    Node(usize),
    ObjectDomain(usize),
    Label {
        node: usize,
        concept: u32,
    },
    Top(usize),
    Denotes {
        node: usize,
        individual: u32,
    },
    Role {
        from: usize,
        to: usize,
        role: Role,
        witness: RoleWitness,
    },
}

/// Direct edges retain raw endpoints in their mutation; other paths require their
/// own native path licenses before a causal graph may be published.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RoleWitness {
    Edge { index: usize, letter: Role },
    Language { start: usize, end: usize },
    Transitive { via: Role, path: usize },
    Universal,
}

/// An actual transition selected in the native product traversal. State numbers
/// are local to the requested role's compiled machine, not authored RDF terms.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PathStep {
    pub(crate) role: Role,
    pub(crate) from: (usize, usize),
    pub(crate) to: (usize, usize),
    pub(crate) transition: usize,
    pub(crate) kind: PathKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PathKind {
    Edge { index: usize, letter: Role },
    Epsilon,
    Universal(Role),
}

/// Reads precede the write at `checkpoint`. Later identification cannot change
/// which version of a fact established this checked body.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Read {
    pub(crate) owner: ReadOwner,
    pub(crate) checkpoint: usize,
    pub(crate) fact: ReadFact,
}

/// The native owner of a write. No producer label supplies a source fact.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Origin {
    #[default]
    Unattributed,
    ObjectDomain,
    TermDenotation,
    PreparedType,
    PreparedRole,
    PreparedEquality,
    PreparedDistinct,
    AssumedType,
    AssumedRole,
    RefutationRoot,
    AssumedAnonymousType,
    Internalized,
    Application,
}

/// An original prepared-array ordinal, or the checked application and head atom.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Cause {
    pub(crate) origin: Origin,
    pub(crate) index: usize,
    pub(crate) atom: usize,
}

/// An observed primitive, before it is linked to selected body/path dependencies.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mutation {
    Node {
        node: usize,
        named: Option<u32>,
        parent: Option<usize>,
        reserved: bool,
        concrete: bool,
    },
    Label {
        node: usize,
        concept: u32,
    },
    Edge {
        index: usize,
        from: usize,
        to: usize,
        property: u32,
    },
    Distinct {
        left: usize,
        right: usize,
    },
    Merge {
        keep: usize,
        discard: usize,
    },
    LabelTransfer {
        keep: usize,
        discard: usize,
        concept: u32,
    },
    NameTransfer {
        keep: usize,
        discard: usize,
        individual: u32,
    },
    DistinctTransfer {
        keep: usize,
        discard: usize,
        previous_other: usize,
        other: usize,
    },
    DomainTransfer {
        keep: usize,
        discard: usize,
    },
    ValueClass {
        node: usize,
        class: u32,
        inherited_from: Option<usize>,
    },
    WithdrawLabel {
        node: usize,
        concept: u32,
    },
}

impl fmt::Debug for Mutation {
    /// Keep diagnostics structural: these are native identities, not RDF names.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Node {
                node,
                named,
                parent,
                reserved,
                concrete,
            } => f
                .debug_struct("Node")
                .field("node", node)
                .field("named", named)
                .field("parent", parent)
                .field("reserved", reserved)
                .field("concrete", concrete)
                .finish(),
            Self::Label { node, concept } => {
                f.debug_tuple("Label").field(&(node, concept)).finish()
            }
            Self::Edge {
                index,
                from,
                to,
                property,
            } => f
                .debug_tuple("Edge")
                .field(&(index, from, to, property))
                .finish(),
            Self::Distinct { left, right } => {
                f.debug_tuple("Distinct").field(&(left, right)).finish()
            }
            Self::Merge { keep, discard } => {
                f.debug_tuple("Merge").field(&(keep, discard)).finish()
            }
            Self::LabelTransfer {
                keep,
                discard,
                concept,
            } => f
                .debug_tuple("LabelTransfer")
                .field(&(keep, discard, concept))
                .finish(),
            Self::NameTransfer {
                keep,
                discard,
                individual,
            } => f
                .debug_tuple("NameTransfer")
                .field(&(keep, discard, individual))
                .finish(),
            Self::DistinctTransfer {
                keep,
                discard,
                previous_other,
                other,
            } => f
                .debug_tuple("DistinctTransfer")
                .field(&(keep, discard, previous_other, other))
                .finish(),
            Self::WithdrawLabel { node, concept } => f
                .debug_tuple("WithdrawLabel")
                .field(&(node, concept))
                .finish(),
            Self::DomainTransfer { keep, discard } => f
                .debug_tuple("DomainTransfer")
                .field(&(keep, discard))
                .finish(),
            Self::ValueClass {
                node,
                class,
                inherited_from,
            } => f
                .debug_tuple("ValueClass")
                .field(&(node, class, inherited_from))
                .finish(),
        }
    }
}

/// The owner and operation stay together through aliases and withdrawals.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct Event {
    pub(crate) cause: Cause,
    pub(crate) mutation: Mutation,
}

impl fmt::Debug for Event {
    /// Show all cause components alongside the exact native mutation.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Cause {
            origin,
            index,
            atom,
        } = self.cause;
        f.debug_struct("Event")
            .field("cause", &(origin, index, atom))
            .field("mutation", &self.mutation)
            .finish()
    }
}

/// One replay owns its fallible observation buffers and original resident account.
#[derive(Debug, Default)]
pub(crate) struct Journal {
    pub(crate) cause: Cause,
    pub(super) stopped: bool,
    events: Vec<Event>,
    reads: Vec<Read>,
    paths: Vec<PathStep>,
    reach: reach::Witnesses,
    role_rules: RoleRules,
    pub(super) read_owner: Option<ReadOwner>,
    storage: Resident,
    live: usize,
    #[cfg(test)]
    injected_refusal: Option<StorageError>,
}

impl Journal {
    /// The first native hierarchy discovery owns one immutable predecessor.
    pub(super) fn role_rule(
        &mut self,
        requested: Role,
        parent: Role,
        child: Role,
        input: Input,
    ) -> Result<(), StorageError> {
        if requested == child || self.role_rules.0.get(&(requested, child)).is_some() {
            return Ok(());
        }
        let mut memory = Memory::resume(&mut self.storage, self.live);
        let result = self
            .role_rules
            .0
            .insert((requested, child), (parent, input), &mut memory);
        self.live = memory.admitted_bytes();
        result.map(|_| ())
    }
    pub(super) fn reach_step(
        &mut self,
        pattern: usize,
        root: usize,
        step: reach::Step,
    ) -> Result<(), StorageError> {
        #[cfg(test)]
        self.publication_refusal()?;
        let mut memory = Memory::resume(&mut self.storage, self.live);
        let result = self.reach.step(pattern, root, step, &mut memory);
        self.live = memory.admitted_bytes();
        result
    }

    pub(super) fn reach_extend(
        &mut self,
        pattern: usize,
        owner: usize,
        step: reach::Step,
        target: usize,
    ) -> Result<(), StorageError> {
        #[cfg(test)]
        self.publication_refusal()?;
        let mut memory = Memory::resume(&mut self.storage, self.live);
        let result = self.reach.extend(pattern, owner, step, target, &mut memory);
        self.live = memory.admitted_bytes();
        result
    }

    pub(super) fn reach_path(&self, pattern: usize, root: usize, target: usize) -> usize {
        self.reach.path(pattern, root, target)
    }

    pub(super) fn path_len(&self) -> usize {
        self.paths.len()
    }

    /// Publish only a selected predecessor, retaining its original native identity.
    pub(super) fn path_step(&mut self, step: PathStep) -> Result<(), StorageError> {
        #[cfg(test)]
        self.publication_refusal()?;
        let mut memory = Memory::resume(&mut self.storage, self.live);
        let result = memory.push(&mut self.paths, step);
        self.live = memory.admitted_bytes();
        result
    }

    /// Admit selected reads into the same resident account as their write history.
    pub(crate) fn read(&mut self, fact: ReadFact) -> Result<(), StorageError> {
        let Some(owner) = self.read_owner else {
            return Ok(());
        };
        #[cfg(test)]
        self.publication_refusal()?;
        let mut memory = Memory::resume(&mut self.storage, self.live);
        let result = memory.push(
            &mut self.reads,
            Read {
                owner,
                checkpoint: self.events.len(),
                fact,
            },
        );
        self.live = memory.admitted_bytes();
        result
    }

    #[cfg(test)]
    pub(crate) fn reads(&self) -> &[Read] {
        &self.reads
    }

    /// Inject one typed refusal at the same publication boundary as allocation.
    #[cfg(test)]
    pub(crate) fn with_injected_refusal(original: StorageError) -> Self {
        Self {
            injected_refusal: Some(original),
            ..Self::default()
        }
    }

    /// A deterministic fault at publication, not a claim of physical exhaustion.
    #[cfg(test)]
    pub(crate) fn refuse_next_publication(&mut self, original: StorageError) {
        self.injected_refusal = Some(original);
    }

    #[cfg(test)]
    fn publication_refusal(&mut self) -> Result<(), StorageError> {
        self.injected_refusal.take().map_or(Ok(()), Err)
    }

    /// Reserve under the same original owner before publishing an observation.
    pub(crate) fn push(
        &mut self,
        mutation: Mutation,
        cause: Option<Cause>,
    ) -> Result<(), StorageError> {
        let cause = cause.unwrap_or(self.cause);
        assert_ne!(
            cause.origin,
            Origin::Unattributed,
            "a native mutation requires a source or checked application owner"
        );
        #[cfg(test)]
        self.publication_refusal()?;
        let mut memory = Memory::resume(&mut self.storage, self.live);
        let result = memory.push(&mut self.events, Event { cause, mutation });
        self.live = memory.admitted_bytes();
        if result.is_ok() && matches!(mutation, Mutation::Merge { .. }) {
            self.reach.clear();
        }
        result
    }

    /// Inspect the actual recorder in synthetic native replay controls.
    #[cfg(test)]
    pub(crate) fn events(&self) -> &[Event] {
        &self.events
    }
}
