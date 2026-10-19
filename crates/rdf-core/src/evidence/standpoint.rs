// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! One declared standpoint order, with explicit sharper-to-broader direction.

use purrdf_lex::allocation::{Admission, Memory, StorageError};

use crate::graph::{GraphError, scc_component_index_with_memory};

/// A malformed declared order or refusal of its native workspace.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StandpointError<K> {
    /// A relation or request references a standpoint outside the declaration.
    Undeclared(K),
    /// The explicitly unscoped marker cannot participate in the order.
    Unscoped(K),
    /// A sharpening edge closes a directed cycle, including a self-edge.
    Cycle {
        /// Sharper endpoint of the offending edge.
        sharper: K,
        /// Broader endpoint of the offending edge.
        broader: K,
    },
    /// Invalid dense graph construction; remains a hard invariant failure.
    Graph(GraphError),
    /// Native admission, layout or allocation refusal.
    Storage(StorageError),
}

impl<K: purrdf_hash::error::StructuredError> purrdf_hash::error::StructuredError
    for StandpointError<K>
{
    purrdf_hash::error_visit! {
        Undeclared(key) => "standpoint-undeclared" { "key" => Record(key) },
        Unscoped(key) => "standpoint-unscoped" { "key" => Record(key) },
        Cycle { sharper, broader } => "standpoint-cycle" {
            "sharper" => Record(sharper), "broader" => Record(broader),
        },
        Graph(cause) => "standpoint-graph" { "cause" => Cause(cause) },
        Storage(cause) => "standpoint-storage" { "cause" => Cause(cause) },
    }
}

purrdf_lex::variant_from!(impl<K> StandpointError<K> {
    Storage(StorageError),
    Graph(GraphError),
});

impl<K: core::fmt::Debug> core::fmt::Display for StandpointError<K> {
    fn fmt(&self, out: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Undeclared(node) => write!(out, "undeclared standpoint {node:?}"),
            Self::Unscoped(node) => write!(
                out,
                "unscoped marker {node:?} cannot participate in sharpening"
            ),
            Self::Cycle { sharper, broader } => write!(
                out,
                "standpoint sharpening cycle through {sharper:?} -> {broader:?}"
            ),
            Self::Graph(error) => error.fmt(out),
            Self::Storage(error) => error.fmt(out),
        }
    }
}

impl<K: core::fmt::Debug> std::error::Error for StandpointError<K> {}

/// A query's declared order, independent of any ontology vocabulary.
///
/// Edges are stored as sharper -> broader. A selected sharper standpoint
/// inherits a broader claim; the reverse relation does not follow. `K` is the
/// caller's existing identity, never a fabricated namespace or global row id.
/// The result owner retains the construction grant through destruction.
#[derive(Debug)]
pub struct StandpointOrder<K> {
    nodes: Vec<K>,
    adjacency: Vec<Vec<usize>>,
    inheritance_order: Vec<usize>,
    unscoped: Option<K>,
}

impl<K: Copy + Ord> StandpointOrder<K> {
    /// Borrow the canonical declared universe, excluding its unscoped marker.
    #[must_use]
    pub fn declared(&self) -> &[K] {
        &self.nodes
    }

    /// Construct and validate the entire declared poset before publication.
    ///
    /// Duplicate declarations and edges are normalized. The optional unscoped
    /// marker is omitted from the node universe and refused as an edge endpoint.
    /// There is no chosen node, edge or traversal-depth ceiling.
    ///
    /// # Errors
    /// Returns undeclared/unscoped references, a directed cycle or storage refusal.
    pub fn new<S: Admission + ?Sized>(
        declared: &[K],
        sharpening: &[(K, K)],
        unscoped: Option<K>,
        memory: &mut Memory<'_, S>,
    ) -> Result<Self, StandpointError<K>> {
        memory.try_scope(|memory| {
            let mut nodes = memory.collect(
                declared
                    .iter()
                    .copied()
                    .filter(|node| Some(*node) != unscoped),
            )?;
            nodes.sort_unstable();
            nodes.dedup();
            let mut order = Self {
                adjacency: memory.collect(core::iter::repeat_with(Vec::new).take(nodes.len()))?,
                nodes,
                inheritance_order: Vec::new(),
                unscoped,
            };
            for &(sharper, broader) in sharpening {
                let from = order.index(sharper)?;
                let to = order.index(broader)?;
                memory.push(&mut order.adjacency[from], to)?;
            }
            for row in &mut order.adjacency {
                row.sort_unstable();
                row.dedup();
            }
            // Cycle judging stays in the workspace's original Tarjan home.
            let components = scc_component_index_with_memory(&order.adjacency, memory)?;
            for (from, row) in order.adjacency.iter().enumerate() {
                for &to in row {
                    if components[from] == components[to] {
                        return Err(StandpointError::Cycle {
                            sharper: order.nodes[from],
                            broader: order.nodes[to],
                        });
                    }
                }
            }
            // Tarjan's component ids are reverse topological: every broader
            // endpoint precedes its sharper source. The validated DAG has one
            // node per component, so retain that order for single-pass claims.
            order.inheritance_order = memory.collect(0..order.nodes.len())?;
            order
                .inheritance_order
                .sort_unstable_by_key(|&node| components[node]);
            memory.release_vec(components)?;
            Ok(order)
        })
    }

    fn index(&self, node: K) -> Result<usize, StandpointError<K>> {
        if self.unscoped == Some(node) {
            return Err(StandpointError::Unscoped(node));
        }
        self.nodes
            .binary_search(&node)
            .map_err(|_| StandpointError::Undeclared(node))
    }

    /// Propagate declared seed values once along each original order edge.
    /// The returned canonical-node array retains its native construction grant.
    pub(super) fn inherit_values<V: Copy, S: Admission + ?Sized>(
        &self,
        mut seed: impl FnMut(K) -> V,
        mut join: impl FnMut(V, V) -> V,
        memory: &mut Memory<'_, S>,
    ) -> Result<Vec<V>, StorageError> {
        memory.scope(|memory| {
            let mut values = memory.collect(self.nodes.iter().copied().map(&mut seed))?;
            for &sharper in &self.inheritance_order {
                for &broader in &self.adjacency[sharper] {
                    values[sharper] = join(values[sharper], values[broader]);
                }
            }
            Ok(values)
        })
    }

    /// Test only an explicitly declared edge, without transitive inheritance.
    ///
    /// This lets a consumer of materialized direct memberships retain that law.
    ///
    /// # Errors
    /// Refuses an undeclared standpoint or the explicitly unscoped marker.
    pub fn directly_sharpens(&self, sharper: K, broader: K) -> Result<bool, StandpointError<K>> {
        let from = self.index(sharper)?;
        let to = self.index(broader)?;
        Ok(self.adjacency[from].binary_search(&to).is_ok())
    }

    /// Test the reflexive, transitive sharpening relation.
    ///
    /// A claim according to `broader` is inherited by the selected `sharper`.
    /// Deep orders use admitted heap storage rather than recursive call frames.
    ///
    /// # Errors
    /// Refuses an undeclared/unscoped reference or native traversal storage.
    pub fn sharpens<S: Admission + ?Sized>(
        &self,
        sharper: K,
        broader: K,
        memory: &mut Memory<'_, S>,
    ) -> Result<bool, StandpointError<K>> {
        let from = self.index(sharper)?;
        let to = self.index(broader)?;
        if from == to || self.adjacency[from].binary_search(&to).is_ok() {
            return Ok(true);
        }
        memory.try_scope(|memory| {
            let mut seen = memory.collect(core::iter::repeat_n(false, self.nodes.len()))?;
            let mut work = Vec::new();
            memory.push(&mut work, from)?;
            seen[from] = true;
            let mut found = false;
            while let Some(node) = work.pop() {
                for &child in &self.adjacency[node] {
                    if child == to {
                        found = true;
                        break;
                    }
                    if !seen[child] {
                        memory.push(&mut work, child)?;
                        seen[child] = true;
                    }
                }
                if found {
                    break;
                }
            }
            memory.release_vec(work)?;
            memory.release_vec(seen)?;
            Ok(found)
        })
    }

    /// Destroy original nodes and edge arrays before refunding their capacity.
    ///
    /// # Errors
    /// Propagates the original native admission callback's refund refusal.
    pub fn release<S: Admission + ?Sized>(
        self,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), StorageError> {
        let Self {
            nodes,
            mut adjacency,
            inheritance_order,
            ..
        } = self;
        memory.release_vec(nodes)?;
        memory.release_vec(inheritance_order)?;
        for row in &mut adjacency {
            memory.release_vec(core::mem::take(row))?;
        }
        memory.release_vec(adjacency)
    }
}

#[cfg(test)]
mod tests {
    use super::{StandpointError, StandpointOrder};
    use purrdf_lex::allocation::{Memory, Resident};

    #[test]
    fn inheritance_and_direct_membership_keep_their_declared_directions() {
        let mut resident = Resident;
        let mut memory = Memory::new(&mut resident);
        let order = StandpointOrder::new(
            &[3, 2, 1, 0, 2],
            &[(3, 2), (2, 1), (3, 2)],
            Some(0),
            &mut memory,
        )
        .unwrap();
        assert!(order.sharpens(3, 1, &mut memory).unwrap());
        assert!(!order.sharpens(1, 3, &mut memory).unwrap());
        assert!(order.sharpens(2, 2, &mut memory).unwrap());
        assert!(order.directly_sharpens(3, 2).unwrap());
        assert!(!order.directly_sharpens(3, 1).unwrap());
        assert_eq!(
            order.sharpens(0, 1, &mut memory),
            Err(StandpointError::Unscoped(0))
        );
        assert_eq!(
            order.sharpens(4, 1, &mut memory),
            Err(StandpointError::Undeclared(4))
        );
        order.release(&mut memory).unwrap();
        assert_eq!(memory.admitted_bytes(), 0);
    }

    #[test]
    fn malformed_and_cyclic_orders_fail_without_publishing_or_retaining_capacity() {
        let mut resident = Resident;
        let mut memory = Memory::new(&mut resident);
        for edges in [&[(1, 2), (2, 1)][..], &[(1, 1)][..]] {
            assert!(matches!(
                StandpointOrder::new(&[1, 2], edges, None, &mut memory),
                Err(StandpointError::Cycle { .. })
            ));
            assert_eq!(memory.admitted_bytes(), 0);
        }
        assert!(matches!(
            StandpointOrder::new(&[1], &[(1, 2)], None, &mut memory),
            Err(StandpointError::Undeclared(2))
        ));
        assert_eq!(memory.admitted_bytes(), 0);
        assert!(matches!(
            StandpointOrder::new(&[0, 1], &[(1, 0)], Some(0), &mut memory),
            Err(StandpointError::Unscoped(0))
        ));
        assert_eq!(memory.admitted_bytes(), 0);
    }

    #[test]
    fn deep_inheritance_walks_iteratively_and_destroys_all_scratch() {
        let nodes: Vec<usize> = (0..8192).collect();
        let edges: Vec<(usize, usize)> = (1..nodes.len()).map(|node| (node, node - 1)).collect();
        let mut resident = Resident;
        let mut memory = Memory::new(&mut resident);
        let order = StandpointOrder::new(&nodes, &edges, None, &mut memory).unwrap();
        let retained = memory.admitted_bytes();
        assert!(order.sharpens(nodes.len() - 1, 0, &mut memory).unwrap());
        assert_eq!(memory.admitted_bytes(), retained);
        assert!(!order.sharpens(0, nodes.len() - 1, &mut memory).unwrap());
        assert_eq!(memory.admitted_bytes(), retained);
        order.release(&mut memory).unwrap();
        assert_eq!(memory.admitted_bytes(), 0);
    }
}
