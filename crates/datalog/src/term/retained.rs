// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Owned reachable term data for a diagnostic that outlives its source arena.

use core::hash::BuildHasher;
use purrdf_core::hash::table::HashTable;
use purrdf_hash::error::{ErrorValue, ErrorVisitor, StructuredError};
use purrdf_hash::fixed::FixedState;
use purrdf_lex::allocation::{Memory, Resident, StorageError};
use purrdf_lex::walk::WorkList;

use super::{NodeData, TermDag, TermRead};
use crate::id::{NodeId, SymId};

/// A self-contained reachable subgraph, including the original symbol text.
///
/// Node numbers retain the source arena's local labels, resolved through each
/// record's `id` field. They can have gaps and are not offsets into `nodes` or
/// persistent term identities. Every reachable node is retained once, even for a shared DAG.
/// Clone and destruction operate on flat buffers, independent of term depth.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RetainedTerm {
    root: NodeId,
    graph: RetainedGraph,
}

/// A complete reachable graph retained once across any number of roots.
/// Local node, symbol and metavariable labels retain the source graph's sharing.
/// No copied node is minted into a different arena. Strings are owned once per
/// original symbol; RDF atoms retain their original immutable native owner.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RetainedGraph {
    nodes: Vec<RetainedNode>,
    symbols: Vec<(SymId, String)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RetainedNode {
    id: NodeId,
    data: NodeData,
}

impl RetainedTerm {
    /// Retain only nodes reachable from `root`, without rendering or recursion.
    ///
    /// # Errors
    /// Returns the original storage refusal before publishing a partial graph.
    ///
    /// # Panics
    /// Panics if `root` was not minted by `dag`.
    pub fn capture(dag: &TermDag, root: NodeId) -> Result<Self, StorageError> {
        Ok(Self {
            root,
            graph: RetainedGraph::capture(dag, &[root])?,
        })
    }

    /// The original local root handle, resolved through [`Self::graph`].
    #[must_use]
    pub fn root(&self) -> NodeId {
        self.root
    }

    /// The retained graph, independent of the source arena's lifetime.
    #[must_use]
    pub fn graph(&self) -> &RetainedGraph {
        &self.graph
    }
}

impl RetainedGraph {
    /// Retain every node reachable from the original roots, exactly once.
    ///
    /// # Errors
    /// Returns storage refusal before publishing a partial graph.
    /// # Panics
    /// Panics if a root does not belong to `dag`.
    pub fn capture(dag: &TermDag, roots: &[NodeId]) -> Result<Self, StorageError> {
        let mut resident = Resident;
        let mut memory = Memory::new(&mut resident);
        let mut pending = WorkList::<NodeId, 16>::default();
        for &root in roots {
            pending.try_push_admitted(root, &mut memory)?;
        }
        let mut seen = HashTable::<NodeId>::new();
        let mut nodes = Vec::new();
        while let Some(id) = pending.pop() {
            let hash = FixedState::new().hash_one(id);
            if seen.find(hash, |&previous| previous == id).is_some() {
                continue;
            }
            seen.try_reserve(1, |&previous| FixedState::new().hash_one(previous))
                .map_err(|_| StorageError::AllocationFailed)?;
            seen.insert_unique(hash, id, |&previous| FixedState::new().hash_one(previous));
            let data = match dag.data(id) {
                NodeData::Rdf(value) => NodeData::Rdf(value.clone()),
                NodeData::Leaf(symbol) => NodeData::Leaf(*symbol),
                NodeData::Free(symbol) => NodeData::Free(*symbol),
                NodeData::Meta(meta) => NodeData::Meta(*meta),
                NodeData::Bound { debruijn, slot } => NodeData::Bound {
                    debruijn: *debruijn,
                    slot: *slot,
                },
                NodeData::App { op, args } => {
                    pending.try_push_admitted(*op, &mut memory)?;
                    for &arg in args {
                        pending.try_push_admitted(arg, &mut memory)?;
                    }
                    NodeData::App {
                        op: *op,
                        args: memory.collect(args.iter().copied())?,
                    }
                }
                NodeData::Binder { op, sorts, body } => {
                    pending.try_push_admitted(*op, &mut memory)?;
                    pending.try_push_admitted(*body, &mut memory)?;
                    for &sort in sorts {
                        pending.try_push_admitted(sort, &mut memory)?;
                    }
                    NodeData::Binder {
                        op: *op,
                        sorts: memory.collect(sorts.iter().copied())?,
                        body: *body,
                    }
                }
            };
            memory.push(&mut nodes, RetainedNode { id, data })?;
        }
        nodes.sort_unstable_by_key(|node| node.id);
        let mut ids: Vec<SymId> =
            memory.collect(nodes.iter().filter_map(|node| match node.data {
                NodeData::Leaf(id) | NodeData::Free(id) => Some(id),
                _ => None,
            }))?;
        ids.sort_unstable();
        ids.dedup();
        let mut symbols = Vec::new();
        for id in ids {
            let text = memory.string(dag.symbol(id))?;
            memory.push(&mut symbols, (id, text))?;
        }
        Ok(Self { nodes, symbols })
    }

    /// Inspect an original local node, if it is present in the retained graph.
    #[must_use]
    pub fn get(&self, id: NodeId) -> Option<&NodeData> {
        self.nodes
            .binary_search_by_key(&id, |node| node.id)
            .ok()
            .map(|index| &self.nodes[index].data)
    }

    /// Number of distinct retained nodes, including all shared roots.
    #[must_use]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Whether the graph has no reachable nodes.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    fn visit_nodes<'error>(&'error self, visitor: &mut dyn ErrorVisitor<'error>) {
        visitor.begin_list(Some("nodes"), self.nodes.len());
        for node in &self.nodes {
            visitor.begin_record(None, "datalog-retained-node");
            self.visit_node(node, visitor);
            visitor.end_record();
        }
        visitor.end_list();
    }
}

impl TermRead for RetainedGraph {
    fn data(&self, node: NodeId) -> &NodeData {
        self.get(node)
            .expect("the term belongs to this retained graph")
    }
    fn symbol(&self, id: SymId) -> &str {
        let index = self
            .symbols
            .binary_search_by_key(&id, |(symbol, _)| *symbol)
            .expect("the symbol belongs to this retained graph");
        &self.symbols[index].1
    }
}

impl StructuredError for RetainedTerm {
    fn visit<'error>(&'error self, visitor: &mut dyn ErrorVisitor<'error>) {
        let Self { root, graph } = self;
        visitor.variant("datalog-retained-term");
        visitor.value(Some("root"), ErrorValue::Unsigned(u128::from(root.index())));
        graph.visit_nodes(visitor);
    }
}

impl RetainedGraph {
    fn visit_node<'error>(
        &'error self,
        node: &'error RetainedNode,
        visitor: &mut dyn ErrorVisitor<'error>,
    ) {
        let RetainedNode { id, data } = node;
        visitor.value(Some("id"), ErrorValue::Unsigned(u128::from(id.index())));
        visitor.value(
            Some("symbol"),
            match data {
                NodeData::Leaf(id) | NodeData::Free(id) => ErrorValue::Text(self.symbol(*id)),
                _ => ErrorValue::Absent,
            },
        );
        visitor.begin_record(Some("data"), "datalog-node-data");
        match data {
            NodeData::Rdf(value) => {
                visitor.value(Some("kind"), ErrorValue::Code("rdf"));
                visitor.value(Some("value"), ErrorValue::Term(value.value()));
            }
            NodeData::Leaf(value) | NodeData::Free(value) => {
                visitor.value(
                    Some("kind"),
                    ErrorValue::Code(if matches!(data, NodeData::Leaf(_)) {
                        "leaf"
                    } else {
                        "free"
                    }),
                );
                visitor.value(
                    Some("symbol_id"),
                    ErrorValue::Unsigned(u128::from(value.index())),
                );
            }
            NodeData::Meta(value) => {
                visitor.value(Some("kind"), ErrorValue::Code("meta"));
                visitor.value(
                    Some("meta"),
                    ErrorValue::Unsigned(u128::from(value.index())),
                );
            }
            NodeData::Bound { debruijn, slot } => {
                visitor.value(Some("kind"), ErrorValue::Code("bound"));
                visitor.value(
                    Some("debruijn"),
                    ErrorValue::Unsigned(u128::from(*debruijn)),
                );
                visitor.value(Some("slot"), ErrorValue::Unsigned(u128::from(*slot)));
            }
            NodeData::App { op, args } => {
                visitor.value(Some("kind"), ErrorValue::Code("app"));
                visitor.value(Some("op"), ErrorValue::Unsigned(u128::from(op.index())));
                purrdf_hash::error::visit_list(
                    visitor,
                    Some("args"),
                    args.iter()
                        .map(|id| ErrorValue::Unsigned(u128::from(id.index()))),
                );
            }
            NodeData::Binder { op, sorts, body } => {
                visitor.value(Some("kind"), ErrorValue::Code("binder"));
                visitor.value(Some("op"), ErrorValue::Unsigned(u128::from(op.index())));
                visitor.value(Some("body"), ErrorValue::Unsigned(u128::from(body.index())));
                purrdf_hash::error::visit_list(
                    visitor,
                    Some("sorts"),
                    sorts
                        .iter()
                        .map(|id| ErrorValue::Unsigned(u128::from(id.index()))),
                );
            }
        }
        visitor.end_record();
    }
}
