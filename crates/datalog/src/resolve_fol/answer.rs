// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Structural answers over one retained graph for the whole answer set.

use super::{FolAnswerContract, FolProof, canon, render};
use crate::id::NodeId;
use crate::term::{NodeData, RetainedGraph, TermDag, TermRead};
use purrdf_lex::allocation::{Memory, Resident, Shared, StorageError};

/// One owned answer. Every row from the same run shares one complete reachable
/// graph, including original goal variables, answer terms and proof conclusions.
/// Cloning a row retains that graph; dropping its producer arena or outcome does
/// not invalidate inspection. The current execution contract emits ground answers.
///
/// Equality includes owner scope: rows from unrelated runs are not the same
/// scoped answer merely because their local numeric handles overlap. Compare
/// explicit canonical term keys when structural comparison across runs is needed.
#[derive(Debug, Clone)]
pub struct FolBinding {
    graph: Shared<AnswerGraph>,
    bindings: Vec<Assignment>,
    atom: NodeId,
    proof: FolProof,
}

#[derive(Debug)]
struct AnswerGraph {
    terms: RetainedGraph,
    contract: FolAnswerContract,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Assignment {
    pub(super) variable: NodeId,
    pub(super) name: String,
    pub(super) value: NodeId,
}

pub(super) struct Projected {
    pub(super) bindings: Vec<Assignment>,
    pub(super) atom: NodeId,
    pub(super) proof: FolProof,
}

/// A borrowed term whose local identity is scoped by its original retained graph.
#[derive(Debug, Clone, Copy)]
pub struct FolTerm<'a> {
    graph: &'a RetainedGraph,
    node: NodeId,
}

impl<'a> FolTerm<'a> {
    /// Complete original node data. Child handles resolve through [`Self::term`].
    #[must_use]
    pub fn data(self) -> &'a NodeData {
        self.graph.data(self.node)
    }

    /// Original local handle, meaningful only with this graph owner.
    #[must_use]
    pub fn source_id(self) -> NodeId {
        self.node
    }

    /// Resolve another retained node in the same complete answer graph.
    #[must_use]
    pub fn term(self, node: NodeId) -> Option<Self> {
        self.graph.get(node).map(|_| Self {
            graph: self.graph,
            node,
        })
    }

    /// Whether two views name the same node in the same retained owner scope.
    #[must_use]
    pub fn same_node(self, other: FolTerm<'_>) -> bool {
        core::ptr::eq(self.graph, other.graph) && self.node == other.node
    }

    /// The original atomic symbol, when this is an opaque leaf or free variable.
    #[must_use]
    pub fn symbol(self) -> Option<&'a str> {
        match self.data() {
            NodeData::Leaf(id) | NodeData::Free(id) => Some(self.graph.symbol(*id)),
            _ => None,
        }
    }

    /// Present the term explicitly. Rendering is never an answer identity key.
    #[must_use]
    pub fn render(self) -> String {
        render(self.graph, self.node)
    }

    /// Canonical structural key, with the resolver's metavariable renaming law.
    /// Variable scope identity itself is inspected through [`Self::same_node`].
    #[must_use]
    pub fn canonical_key(self) -> String {
        canon(self.graph, self.node)
    }
}

/// A named projection, retaining its original variable and structural value.
#[derive(Debug, Clone, Copy)]
pub struct FolBindingEntry<'a> {
    graph: &'a RetainedGraph,
    assignment: &'a Assignment,
}

impl<'a> FolBindingEntry<'a> {
    /// Original authored presentation label; it is not the variable's identity.
    #[must_use]
    pub fn name(self) -> &'a str {
        &self.assignment.name
    }

    /// Original goal metavariable, scoped by this answer set's graph.
    #[must_use]
    pub fn variable(self) -> FolTerm<'a> {
        FolTerm {
            graph: self.graph,
            node: self.assignment.variable,
        }
    }

    /// Original native answer term in the same graph as every other projection.
    #[must_use]
    pub fn value(self) -> FolTerm<'a> {
        FolTerm {
            graph: self.graph,
            node: self.assignment.value,
        }
    }
}

impl FolBinding {
    /// Inspect projections in authored order, without rendering values.
    pub fn bindings(&self) -> impl ExactSizeIterator<Item = FolBindingEntry<'_>> {
        self.bindings.iter().map(|assignment| FolBindingEntry {
            graph: &self.graph.terms,
            assignment,
        })
    }

    /// Select a named projection; names select columns, not term identities.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<FolTerm<'_>> {
        self.bindings()
            .find(|entry| entry.name() == name)
            .map(FolBindingEntry::value)
    }

    /// The complete ground answer atom, independent of the producer lifetime.
    #[must_use]
    pub fn atom(&self) -> FolTerm<'_> {
        FolTerm {
            graph: &self.graph.terms,
            node: self.atom,
        }
    }

    /// Original selected semantics, retained by the complete answer owner.
    #[must_use]
    pub fn answer_contract(&self) -> FolAnswerContract {
        self.graph.contract
    }

    /// The original proof; every conclusion it names is in this retained graph.
    #[must_use]
    pub fn proof(&self) -> &FolProof {
        &self.proof
    }

    /// Reproduce the original derivation identity without the producer arena.
    #[must_use]
    pub fn derivation_id(&self) -> String {
        super::derivation_id(&self.graph.terms, &self.proof)
    }

    /// Inspect a retained proof conclusion or other original reachable handle.
    #[must_use]
    pub fn term(&self, node: NodeId) -> Option<FolTerm<'_>> {
        self.atom().term(node)
    }

    pub(super) fn retain(
        dag: &TermDag,
        rows: Vec<Projected>,
        contract: FolAnswerContract,
    ) -> Result<Vec<Self>, StorageError> {
        if rows.is_empty() {
            return Ok(Vec::new());
        }
        let mut resident = Resident;
        let mut memory = Memory::new(&mut resident);
        let mut roots = Vec::new();
        for row in &rows {
            memory.push(&mut roots, row.atom)?;
            for binding in &row.bindings {
                memory.push(&mut roots, binding.variable)?;
                memory.push(&mut roots, binding.value)?;
            }
            let mut pending = Vec::new();
            memory.push(&mut pending, &row.proof)?;
            while let Some(proof) = pending.pop() {
                memory.push(&mut roots, proof.goal())?;
                for premise in proof.premises() {
                    memory.push(&mut pending, premise)?;
                }
            }
        }
        let graph = Shared::try_new(AnswerGraph {
            terms: RetainedGraph::capture(dag, &roots)?,
            contract,
        })
        .map_err(|_| StorageError::AllocationFailed)?;
        let mut answers = Vec::new();
        for Projected {
            bindings,
            atom,
            proof,
        } in rows
        {
            memory.push(
                &mut answers,
                Self {
                    graph: graph.clone(),
                    bindings,
                    atom,
                    proof,
                },
            )?;
        }
        Ok(answers)
    }
}

impl PartialEq for FolBinding {
    fn eq(&self, other: &Self) -> bool {
        core::ptr::eq(&raw const *self.graph, &raw const *other.graph)
            && self.bindings == other.bindings
            && self.atom == other.atom
            && self.proof == other.proof
    }
}
impl Eq for FolBinding {}
