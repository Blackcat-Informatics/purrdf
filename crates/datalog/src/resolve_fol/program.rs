// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Validated construction for the structured Horn program carrier.

use super::{BTreeMap, FolClause, FolLit, FolProgram, MetaId, NodeData, NodeId, TermDag};

/// The authored position of a term in a structured program.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum FolPosition {
    /// A clause's head.
    Head,
    /// A clause's body literal, in authored order.
    Body {
        /// Zero-based body position.
        literal: usize,
    },
    /// The query goal.
    Goal,
    /// A projected variable, in authored order.
    GoalVariable {
        /// Zero-based projection position.
        index: usize,
        /// Original authored projection name.
        name: String,
    },
    /// A declared sort of the original metavariable.
    Sort {
        /// Original declared metavariable handle.
        variable: MetaId,
    },
}

/// An invalid authored construction, retaining original handles, names and positions.
///
/// Handles remain local to the caller's arena, as they do in a valid program.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum FolConstructionError {
    /// A handle does not address a node in the supplied arena.
    UnknownNode {
        /// Original term handle.
        node: NodeId,
        /// Original authored position.
        position: FolPosition,
        /// Program clause index, absent when validating a standalone clause.
        clause: Option<usize>,
        /// Number of nodes actually resident in the arena.
        nodes: usize,
    },
    /// A projected named variable is not a metavariable.
    NotMetavariable {
        /// Original position in the projection.
        index: usize,
        /// Original node.
        node: NodeId,
        /// Original authored name.
        name: String,
    },
    /// A projected metavariable does not occur in the goal.
    OutsideGoal {
        /// Original position in the projection.
        index: usize,
        /// Original variable node.
        node: NodeId,
        /// Original authored name.
        name: String,
        /// Original query goal.
        goal: NodeId,
    },
    /// Two projected names would overwrite one another in an answer.
    DuplicateName {
        /// Original name shared by the two projection entries.
        name: String,
        /// First authored projection position.
        first: usize,
        /// Later authored projection position.
        second: usize,
        /// Original node at the first position.
        first_node: NodeId,
        /// Original node at the later position.
        second_node: NodeId,
    },
    /// A sort declaration names a metavariable the arena has not minted.
    UnknownMetavariable {
        /// Original authored metavariable handle.
        variable: MetaId,
        /// Original declared sort node.
        sort: NodeId,
    },
}

impl purrdf_hash::error::StructuredError for FolPosition {
    purrdf_hash::error_visit! {
        Head => "fol-head" {},
        Body { literal } => "fol-body" { "literal" => Unsigned(*literal as u128) },
        Goal => "fol-goal" {},
        GoalVariable { index, name } => "fol-goal-variable" {
            "index" => Unsigned(*index as u128),
            "name" => Text(name)
        },
        Sort { variable } => "fol-sort" { "variable" => Unsigned(u128::from(variable.index())) },
    }
}

impl purrdf_hash::error::StructuredError for FolConstructionError {
    purrdf_hash::error_visit! {
        UnknownNode { node, position, clause, nodes } => "fol-construction-node" {
            "node" => Unsigned(u128::from(node.index())),
            "position" => Record(position),
            "clause" => Value(clause.map_or(purrdf_hash::error::ErrorValue::Absent, |index| purrdf_hash::error::ErrorValue::Unsigned(index as u128))),
            "nodes" => Unsigned(*nodes as u128)
        },
        NotMetavariable { index, node, name } => "fol-construction-not-metavariable" {
            "index" => Unsigned(*index as u128),
            "node" => Unsigned(u128::from(node.index())),
            "name" => Text(name)
        },
        OutsideGoal { index, node, name, goal } => "fol-construction-outside-goal" {
            "index" => Unsigned(*index as u128),
            "node" => Unsigned(u128::from(node.index())),
            "name" => Text(name),
            "goal" => Unsigned(u128::from(goal.index()))
        },
        DuplicateName { name, first, second, first_node, second_node } => "fol-construction-duplicate-name" {
            "name" => Text(name),
            "first" => Unsigned(*first as u128),
            "second" => Unsigned(*second as u128),
            "first_node" => Unsigned(u128::from(first_node.index())),
            "second_node" => Unsigned(u128::from(second_node.index()))
        },
        UnknownMetavariable { variable, sort } => "fol-construction-metavariable" {
            "variable" => Unsigned(u128::from(variable.index())),
            "sort" => Unsigned(u128::from(sort.index()))
        },
    }
}

impl core::fmt::Display for FolConstructionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::UnknownNode {
                node,
                position,
                clause,
                nodes,
            } => write!(
                f,
                "term {node:?} at {position:?} in clause {clause:?} is outside the arena's {nodes} nodes"
            ),
            Self::NotMetavariable { index, node, name } => write!(
                f,
                "goal projection {index} ({name:?}, {node:?}) must name a metavariable"
            ),
            Self::OutsideGoal {
                index,
                node,
                name,
                goal,
            } => write!(
                f,
                "goal projection {index} ({name:?}, {node:?}) does not occur in goal {goal:?}"
            ),
            Self::DuplicateName {
                name,
                first,
                second,
                ..
            } => write!(
                f,
                "goal projection name {name:?} is repeated at positions {first} and {second}"
            ),
            Self::UnknownMetavariable { variable, sort } => write!(
                f,
                "sort {sort:?} is declared for unminted metavariable {variable:?}"
            ),
        }
    }
}

impl std::error::Error for FolConstructionError {}

fn validate_node(
    dag: &TermDag,
    node: NodeId,
    position: impl FnOnce() -> FolPosition,
    clause: Option<usize>,
) -> Result<(), FolConstructionError> {
    if node.index() >= dag.len() as u64 {
        return Err(FolConstructionError::UnknownNode {
            node,
            position: position(),
            clause,
            nodes: dag.len(),
        });
    }
    Ok(())
}

impl FolClause {
    /// Construct a clause over resident nodes in `dag`, preserving body order.
    ///
    /// All handles must originate in this arena. Head and body terms may use
    /// every current term form; execution support is decided by the resolver.
    /// In particular, body width is an execution refusal, not a construction limit.
    ///
    /// # Errors
    /// Returns the original unknown handle and authored head/body position.
    pub fn new(
        dag: &TermDag,
        head: NodeId,
        body: Vec<FolLit>,
    ) -> Result<Self, FolConstructionError> {
        let clause = Self { head, body };
        clause.validate(dag, None)?;
        Ok(clause)
    }

    fn validate(&self, dag: &TermDag, clause: Option<usize>) -> Result<(), FolConstructionError> {
        validate_node(dag, self.head, || FolPosition::Head, clause)?;
        for (literal, lit) in self.body.iter().enumerate() {
            validate_node(dag, lit.atom(), || FolPosition::Body { literal }, clause)?;
        }
        Ok(())
    }

    /// The original head term handle.
    #[must_use]
    pub fn head(&self) -> NodeId {
        self.head
    }

    /// The complete body, in authored order.
    #[must_use]
    pub fn body(&self) -> &[FolLit] {
        &self.body
    }
}

impl FolProgram {
    /// Construct a program over a single caller-owned term arena.
    ///
    /// Names are presentation labels, never computational term keys. Projection
    /// entries must name metavariables present in the goal and use distinct
    /// names. A subset of goal variables, or distinct names for the same
    /// variable, is permitted. Sort declarations may include other metavariables
    /// minted by the same arena; unused declarations do not change execution.
    ///
    /// Clause positions define authored rule indices. Construction does not
    /// rename variables, reorder terms or alter structural identity.
    ///
    /// # Errors
    /// Returns an original invalid handle, projection operand or conflicting
    /// name with its authored position. Checks every clause against `dag`, even
    /// when it was previously constructed against another arena.
    pub fn new(
        dag: &TermDag,
        clauses: Vec<FolClause>,
        goal: NodeId,
        goal_vars: Vec<(NodeId, String)>,
        meta_sorts: BTreeMap<MetaId, NodeId>,
    ) -> Result<Self, FolConstructionError> {
        for (index, clause) in clauses.iter().enumerate() {
            clause.validate(dag, Some(index))?;
        }
        validate_node(dag, goal, || FolPosition::Goal, None)?;
        let mut names = BTreeMap::new();
        for (index, (node, name)) in goal_vars.iter().enumerate() {
            validate_node(
                dag,
                *node,
                || FolPosition::GoalVariable {
                    index,
                    name: name.clone(),
                },
                None,
            )?;
            let NodeData::Meta(variable) = dag.data(*node) else {
                return Err(FolConstructionError::NotMetavariable {
                    index,
                    node: *node,
                    name: name.clone(),
                });
            };
            if dag.free_meta(goal).binary_search(variable).is_err() {
                return Err(FolConstructionError::OutsideGoal {
                    index,
                    node: *node,
                    name: name.clone(),
                    goal,
                });
            }
            if let Some((first, first_node)) = names.insert(name, (index, *node)) {
                return Err(FolConstructionError::DuplicateName {
                    name: name.clone(),
                    first,
                    second: index,
                    first_node,
                    second_node: *node,
                });
            }
        }
        for (&variable, &sort) in &meta_sorts {
            if !dag.has_meta(variable) {
                return Err(FolConstructionError::UnknownMetavariable { variable, sort });
            }
            validate_node(dag, sort, || FolPosition::Sort { variable }, None)?;
        }
        Ok(Self {
            clauses,
            goal,
            goal_vars,
            meta_sorts,
        })
    }

    /// The clauses in authored order; their positions are proof rule indices.
    #[must_use]
    pub fn clauses(&self) -> &[FolClause] {
        &self.clauses
    }

    /// The original query goal.
    #[must_use]
    pub fn goal(&self) -> NodeId {
        self.goal
    }

    /// The original named projection, in authored order.
    #[must_use]
    pub fn goal_vars(&self) -> &[(NodeId, String)] {
        &self.goal_vars
    }

    /// The original metavariable sort declarations.
    #[must_use]
    pub fn meta_sorts(&self) -> &BTreeMap<MetaId, NodeId> {
        &self.meta_sorts
    }
}
