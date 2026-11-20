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
        let count = graph.terms.len();
        if !graph.terms.iter().any(|term| term.kind == TermKind::Triple) {
            return Self::default();
        }
        let mut first: FastMap<usize, Triple3> = FastMap::default();
        for &(reifier, triple, _) in &graph.reifiers {
            first.entry(reifier).or_insert(triple);
        }
        let components = |id: usize| -> Option<[usize; 3]> {
            graph
                .declared_triple_with(id, |reifier| first.get(&reifier).copied())
                .map(<[usize; 3]>::from)
        };

        const UNVISITED: usize = usize::MAX;
        let mut order = vec![UNVISITED; count];
        let mut low = vec![0usize; count];
        let mut on_stack = vec![false; count];
        let mut reaches_itself = vec![false; count];
        let mut flags = vec![false; count];
        let mut any = false;
        let mut component: Vec<usize> = Vec::new();
        // One frame per node on the search path: the node, its components and
        // how many of them have been followed.
        let mut path: Vec<(usize, Option<[usize; 3]>, usize)> = Vec::new();
        let mut next_order = 0usize;

        for root in 0..count {
            if order[root] != UNVISITED || graph.terms[root].kind != TermKind::Triple {
                continue;
            }
            order[root] = next_order;
            low[root] = next_order;
            next_order += 1;
            on_stack[root] = true;
            component.push(root);
            path.push((root, components(root), 0));
            while let Some(frame) = path.last_mut() {
                let (node, edges, cursor) = *frame;
                if let Some(edges) = edges.filter(|_| cursor < 3) {
                    frame.2 = cursor + 1;
                    let next = edges[cursor];
                    // A component that names no term, or a term that is not a
                    // triple term, has no components to follow.
                    if graph
                        .terms
                        .get(next)
                        .is_none_or(|term| term.kind != TermKind::Triple)
                    {
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
                        path.push((next, components(next), 0));
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
        if any { Self { flags } } else { Self::default() }
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
