// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The termination rule of GTS-SPEC §7.3, in one place.
//!
//! A `reifies` row may name the very triple term that resolves through it, and
//! a triple term's components may lead back to the term by any chain of such
//! rows. A triple term that is reachable from its own components is
//! **self-reaching**: it states no triple. It keeps an identity of its own and
//! renders as the fresh blank node an unbound triple term renders as, while the
//! row stays recorded and still projects. The rule applies to the self-reaching
//! term itself and not to a term that merely contains one.
//!
//! [`SelfReaching::of`] is the only definition of that set. A walk over a
//! folded graph's resolved components calls it once and then reads components
//! through [`Graph::resolved_triple_of`](crate::model::Graph::resolved_triple_of),
//! so the fold, the multi-segment union, the canonical writer and every
//! projection agree on which terms degrade.

use crate::model::{Graph, TermKind};
use crate::{FastMap, model::Triple3};

/// The successors of one node of a term walk. A walk follows at most a triple's
/// three key components, a datatype, a reifier and the three components of a `tt`.
#[derive(Clone, Copy, Default)]
pub(crate) struct Successors {
    ids: [usize; 8],
    len: usize,
}

impl Successors {
    /// Follow `id` from this node.
    pub(crate) fn push(&mut self, id: usize) {
        self.ids[self.len] = id;
        self.len += 1;
    }

    /// Follow every id of `ids` from this node.
    pub(crate) fn extend(&mut self, ids: impl IntoIterator<Item = usize>) {
        for id in ids {
            self.push(id);
        }
    }

    fn get(&self, index: usize) -> Option<usize> {
        self.ids[..self.len].get(index).copied()
    }
}

/// The nodes `0..count` that lie on a cycle of `successors`: the members of a
/// strongly connected component of more than one node, and every node that is its
/// own successor. `None` when no node does. A successor id of `count` or more names
/// no node and closes no cycle.
///
/// One pass of Tarjan's search, iterative, so a deeply nested or cyclic term table
/// cannot overflow the stack.
pub(crate) fn on_cycles(
    count: usize,
    successors: impl Fn(usize) -> Successors,
) -> Option<Vec<bool>> {
    const UNVISITED: usize = usize::MAX;
    let mut order = vec![UNVISITED; count];
    let mut low = vec![0usize; count];
    let mut on_stack = vec![false; count];
    let mut reaches_itself = vec![false; count];
    let mut flags = vec![false; count];
    let mut any = false;
    let mut component: Vec<usize> = Vec::new();
    // One frame per node on the search path: the node, its successors and how
    // many of them have been followed.
    let mut path: Vec<(usize, Successors, usize)> = Vec::new();
    let mut next_order = 0usize;

    for root in 0..count {
        if order[root] != UNVISITED {
            continue;
        }
        order[root] = next_order;
        low[root] = next_order;
        next_order += 1;
        on_stack[root] = true;
        component.push(root);
        path.push((root, successors(root), 0));
        while let Some(frame) = path.last_mut() {
            let (node, edges, cursor) = *frame;
            if let Some(next) = edges.get(cursor) {
                frame.2 = cursor + 1;
                if next >= count {
                    continue;
                }
                if next == node {
                    reaches_itself[node] = true;
                }
                if order[next] == UNVISITED {
                    order[next] = next_order;
                    low[next] = next_order;
                    next_order += 1;
                    on_stack[next] = true;
                    component.push(next);
                    path.push((next, successors(next), 0));
                } else if on_stack[next] {
                    low[node] = low[node].min(order[next]);
                }
                continue;
            }
            path.pop();
            if let Some(&(parent, _, _)) = path.last() {
                low[parent] = low[parent].min(low[node]);
            }
            if low[node] == order[node] {
                let start = component
                    .iter()
                    .rposition(|&member| member == node)
                    .unwrap_or(0);
                let members = &component[start..];
                if members.len() > 1 || reaches_itself[node] {
                    for &member in members {
                        flags[member] = true;
                    }
                    any = true;
                }
                for &member in members {
                    on_stack[member] = false;
                }
                component.truncate(start);
            }
        }
    }
    any.then_some(flags)
}

/// The triple terms of one graph that reach themselves.
///
/// Every triple term is a node, and its declared components
/// ([`Graph::triple_of`]) are its successors. A node is self-reaching when it
/// lies on a cycle, which is when it belongs to a strongly connected component
/// of more than one node or has itself as a component. A component id that
/// names no term closes no cycle.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SelfReaching {
    /// One flag per term id, or empty when no term reaches itself.
    flags: Vec<bool>,
}

impl SelfReaching {
    /// The self-reaching triple terms of `graph`.
    ///
    /// One pass over the terms and the reifier rows. The search is iterative,
    /// so a deeply nested or cyclic term table cannot overflow the stack.
    #[must_use]
    pub fn of(graph: &Graph) -> Self {
        if !graph.terms.iter().any(|term| term.kind == TermKind::Triple) {
            return Self::default();
        }
        let mut first: FastMap<usize, Triple3> = FastMap::default();
        for &(reifier, triple, _) in &graph.reifiers {
            first.entry(reifier).or_insert(triple);
        }
        let flags = on_cycles(graph.terms.len(), |id| {
            let mut successors = Successors::default();
            if let Some(triple) =
                graph.declared_triple_with(id, |reifier| first.get(&reifier).copied())
            {
                successors.extend(<[usize; 3]>::from(triple));
            }
            successors
        });
        flags.map_or_else(Self::default, |flags| Self { flags })
    }

    /// Whether the triple term `term_id` reaches itself.
    #[must_use]
    pub fn contains(&self, term_id: usize) -> bool {
        self.flags.get(term_id).copied().unwrap_or(false)
    }

    /// Whether no term of the graph reaches itself.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.flags.is_empty()
    }
}
