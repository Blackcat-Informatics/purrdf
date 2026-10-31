// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Owned reachable term data for a diagnostic that outlives its source arena.

use core::hash::BuildHasher;
use purrdf_core::hash::table::HashTable;
use purrdf_hash::error::{ErrorValue, ErrorVisitor, StructuredError};
use purrdf_hash::fixed::FixedState;
use purrdf_lex::allocation::{Memory, Resident, StorageError};
use purrdf_lex::walk::WorkList;

use super::{NodeData, TermDag};
use crate::id::NodeId;

/// A self-contained reachable subgraph, including the original symbol text.
///
/// Node numbers retain the source arena's local labels, resolved through each
/// record's `id` field. They can have gaps and are not offsets into `nodes` or
/// persistent term identities. Every reachable node is retained once, even for a shared DAG.
/// Clone and destruction operate on flat buffers, independent of term depth.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RetainedTerm {
    root: NodeId,
    nodes: Vec<RetainedNode>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RetainedNode {
    id: NodeId,
    data: NodeData,
    symbol: Option<String>,
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
        let mut resident = Resident;
        let mut memory = Memory::new(&mut resident);
        let mut pending = WorkList::<NodeId, 16>::with(root);
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
            let (data, symbol) = match dag.data(id) {
                NodeData::Leaf(symbol) => (
                    NodeData::Leaf(*symbol),
                    Some(memory.string(dag.symbol(*symbol))?),
                ),
                NodeData::Free(symbol) => (
                    NodeData::Free(*symbol),
                    Some(memory.string(dag.symbol(*symbol))?),
                ),
                NodeData::Meta(meta) => (NodeData::Meta(*meta), None),
                NodeData::Bound { debruijn, slot } => (
                    NodeData::Bound {
                        debruijn: *debruijn,
                        slot: *slot,
                    },
                    None,
                ),
                NodeData::App { op, args } => {
                    pending.try_push_admitted(*op, &mut memory)?;
                    for &arg in args {
                        pending.try_push_admitted(arg, &mut memory)?;
                    }
                    (
                        NodeData::App {
                            op: *op,
                            args: memory.collect(args.iter().copied())?,
                        },
                        None,
                    )
                }
                NodeData::Binder { op, sorts, body } => {
                    pending.try_push_admitted(*op, &mut memory)?;
                    pending.try_push_admitted(*body, &mut memory)?;
                    for &sort in sorts {
                        pending.try_push_admitted(sort, &mut memory)?;
                    }
                    (
                        NodeData::Binder {
                            op: *op,
                            sorts: memory.collect(sorts.iter().copied())?,
                            body: *body,
                        },
                        None,
                    )
                }
            };
            memory.push(&mut nodes, RetainedNode { id, data, symbol })?;
        }
        nodes.sort_unstable_by_key(|node| node.id);
        Ok(Self { root, nodes })
    }
}

impl StructuredError for RetainedTerm {
    fn visit<'error>(&'error self, visitor: &mut dyn ErrorVisitor<'error>) {
        let Self { root, nodes } = self;
        visitor.variant("datalog-retained-term");
        visitor.value(Some("root"), ErrorValue::Unsigned(u128::from(root.index())));
        purrdf_hash::error::visit_list(
            visitor,
            Some("nodes"),
            nodes.iter().map(|node| ErrorValue::Record(node)),
        );
    }
}

impl StructuredError for RetainedNode {
    fn visit<'error>(&'error self, visitor: &mut dyn ErrorVisitor<'error>) {
        let Self { id, data, symbol } = self;
        visitor.variant("datalog-retained-node");
        visitor.value(Some("id"), ErrorValue::Unsigned(u128::from(id.index())));
        visitor.value(
            Some("symbol"),
            symbol
                .as_deref()
                .map_or(ErrorValue::Absent, ErrorValue::Text),
        );
        visitor.begin_record(Some("data"), "datalog-node-data");
        match data {
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
