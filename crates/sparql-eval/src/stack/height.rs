// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The height admission a plan passes before the evaluator's recursive passes walk it.
//!
//! The parser and the algebra's own walks — copy, comparison, hash, format, drop,
//! serialization, validation — keep what they have yet to visit on heap work lists, so a
//! tree of any height is parsed and handled without recursion. The evaluator's analyses
//! and its evaluation proper do recurse once per level, so a plan is measured here,
//! iteratively, before they run: every level is charged [`LEVEL_WALK_BYTES`] against the
//! stack left above [`purrdf_stack::MARGIN_BYTES`], and a plan too tall for it is refused
//! with [`EvalError::StackExhausted`] naming `"query algebra"`. On `wasm32` the host
//! engine's call stack, which no measurement reaches, bounds the recursion too: a plan
//! past [`WASM_STRUCTURAL_LIMIT`] or [`WASM_VALUE_LIMIT`] is refused with
//! [`EvalError::HostStackExhausted`].
//!
//! A quoted triple term is not a level of anything measured here. Every walk the
//! evaluator makes over one — its interning, matching, instantiation, comparison and
//! conversion — runs over a work list, so a term's nesting costs no stack on any host and
//! is bounded by memory alone; the terms a plan holds are visited so the nodes below them
//! are, and charged nothing.

use purrdf_sparql_algebra::{GraphPattern, NodeRef, Query};

use crate::error::EvalError;

/// The share of the host engine's call stack a request may spend on `wasm32`: 640 KiB.
///
/// A wasm instance runs on two stacks. The evaluator's guard measures the shadow stack in
/// linear memory, where address-taken locals live; every call also takes a frame on the
/// host engine's own call stack, which no wasm code can read. Under V8 (Node.js,
/// Chromium, Cloudflare Workers) that stack is about 984 KiB for the synchronous lane and
/// for an asynchronous job's suspendable stack alike, whatever the shadow region's size:
/// V8 sizes a JSPI stack from process-wide flags, so neither a wasm module nor a job can
/// enlarge it. The budget is 65% of it, which leaves a third to the JavaScript frames
/// below the call. The per-construct costs charged against it were measured on the
/// shipped npm artifact with parsing and evaluation on the same host stack, so they hold
/// evaluation alone with room to spare.
pub(crate) const WASM_HOST_STACK_BUDGET: usize = 640 * 1024;

/// What one level of graph-pattern nesting costs the host engine's call stack on
/// `wasm32`, in bytes: V8's stack divided by the depth at which nested `LATERAL` and
/// `EXISTS` groups, the costliest, exhausted it, rounded up.
const WASM_GROUP_LEVEL_BYTES: usize = 2304;

/// How deeply graph patterns may nest on `wasm32`: as deep as
/// [`WASM_HOST_STACK_BUDGET`] admits groups, 284 levels.
pub(crate) const WASM_GRAPH_PATTERN_DEPTH: usize = WASM_HOST_STACK_BUDGET / WASM_GROUP_LEVEL_BYTES;

/// The tallest run of levels a plan may hold between two nested constructs on `wasm32`
/// (an operator above its operands, `^` or a path modifier, a run of sibling group
/// elements): 2 048.
const WASM_TREE_HEIGHT: usize = 2048;

/// The tallest tree admitted on `wasm32`, in every node kind but the triple terms:
/// [`WASM_TREE_HEIGHT`], with room for what a tree holds that a count of written levels
/// does not see — each nested group may carry up to eight wrapper nodes.
const WASM_STRUCTURAL_LIMIT: usize = WASM_TREE_HEIGHT + 8 * WASM_GRAPH_PATTERN_DEPTH;

/// The tallest run of expression and path nodes admitted on `wasm32`:
/// [`WASM_TREE_HEIGHT`], with room for the second node some written levels build.
const WASM_VALUE_LIMIT: usize = WASM_TREE_HEIGHT + 256;

/// The stack one level of a plan is charged for the evaluator's walks over it, in bytes:
/// 512 natively, 256 on `wasm32`.
///
/// Measured natively (x86_64, the workspace's opt-level-3 profile) by walking trees
/// thousands of levels tall on threads of bisected size, the costliest recursive
/// per-level walk over any node kind is about 400 bytes; the charge is about 1.3 times
/// that. A `wasm32` level costs 0.37 to 0.45 of its native size on the shadow stack the
/// guard measures.
const LEVEL_WALK_BYTES: usize = if cfg!(target_arch = "wasm32") {
    256
} else {
    512
};

/// Whether a walk `height` levels tall fits the stack left above
/// [`purrdf_stack::MARGIN_BYTES`], at [`LEVEL_WALK_BYTES`] a level.
fn walkable(height: usize) -> bool {
    height.saturating_mul(LEVEL_WALK_BYTES)
        <= purrdf_stack::remaining().saturating_sub(purrdf_stack::MARGIN_BYTES)
}

/// How deep a node sits: in the node kinds the evaluator recurses over (`structural`,
/// the height a recursive walk over the tree descends), and in expression and path nodes
/// (`values`).
#[derive(Clone, Copy)]
struct Depth {
    structural: usize,
    values: usize,
}

impl Depth {
    const ROOT: Self = Self {
        structural: 1,
        values: 0,
    };

    /// The depth of `node`'s children. An `ORDER BY` key and an aggregate are counted as
    /// part of the node that holds them, not as a level of their own; a triple pattern
    /// and the terms inside it are walked over work lists, so they are no level either.
    fn below(self, node: NodeRef<'_>) -> Self {
        Self {
            structural: self.structural
                + usize::from(!matches!(
                    node,
                    NodeRef::Order(_)
                        | NodeRef::Aggregate(_)
                        | NodeRef::Triple(_)
                        | NodeRef::Term(_)
                        | NodeRef::Ground(_)
                )),
            values: self.values + usize::from(matches!(node, NodeRef::Expr(_) | NodeRef::Path(_))),
        }
    }

    /// Admit a node this deep, or refuse the tree. `fits` is the tallest level already
    /// found to fit, so the stack is read once per level of height rather than once per
    /// node.
    fn admit(self, fits: &mut usize) -> Result<(), EvalError> {
        if self.structural > *fits {
            if !walkable(self.structural) {
                return Err(EvalError::StackExhausted {
                    construct: "query algebra",
                });
            }
            *fits = self.structural;
        }
        if cfg!(target_arch = "wasm32")
            && (self.structural > WASM_STRUCTURAL_LIMIT || self.values > WASM_VALUE_LIMIT)
        {
            return Err(EvalError::HostStackExhausted {
                construct: "query algebra",
            });
        }
        Ok(())
    }
}

fn admit_with_memory<'a>(
    roots: impl IntoIterator<Item = NodeRef<'a>>,
    memory: &mut purrdf_lex::allocation::Memory<'_, crate::workspace::LexicalFrame>,
) -> Result<(), EvalError> {
    let result = (|| {
        let mut stack = purrdf_lex::walk::WorkList::<(NodeRef<'a>, Depth), 32>::new();
        for root in roots {
            stack.try_push_admitted((root, Depth::ROOT), memory)?;
        }
        let mut fits = 0;
        while let Some((node, depth)) = stack.pop() {
            depth.admit(&mut fits).map_err(HeightFailure::Evaluation)?;
            let below = depth.below(node);
            let mut failure = None;
            node.for_each_child(|child| {
                if failure.is_none()
                    && let Err(error) = stack.try_push_admitted((child, below), memory)
                {
                    failure = Some(error);
                }
            });
            if let Some(error) = failure {
                return Err(HeightFailure::Storage(error));
            }
        }
        stack.release_admitted(memory)?;
        Ok(())
    })();
    match result {
        Ok(()) => Ok(()),
        Err(HeightFailure::Evaluation(error)) => Err(error),
        Err(HeightFailure::Storage(error)) => Err(memory
            .admission_mut()
            .storage_error(error, "query algebra height")),
    }
}

enum HeightFailure {
    Evaluation(EvalError),
    Storage(purrdf_lex::allocation::StorageError),
}
purrdf_lex::variant_from!(HeightFailure { Storage(purrdf_lex::allocation::StorageError) });

/// Admit pattern height under the original physical traversal account.
pub(crate) fn admit_pattern_with_memory(
    pattern: &GraphPattern,
    memory: &mut purrdf_lex::allocation::Memory<'_, crate::workspace::LexicalFrame>,
) -> Result<(), EvalError> {
    admit_with_memory([NodeRef::Pattern(pattern)], memory)
}

/// Admit query pattern and template trees, preserving the original root order.
pub(crate) fn admit_query(query: &Query) -> Result<(), EvalError> {
    let capability = crate::WorkspaceCapability::default();
    let mut frame = crate::workspace::LexicalFrame::new(&capability);
    admit_query_with_memory(query, &mut purrdf_lex::allocation::Memory::new(&mut frame))
}

/// The same query depth law with native before-growth physical admission.
pub(crate) fn admit_query_with_memory(
    query: &Query,
    memory: &mut purrdf_lex::allocation::Memory<'_, crate::workspace::LexicalFrame>,
) -> Result<(), EvalError> {
    match query {
        Query::Construct {
            pattern, template, ..
        } => admit_with_memory(
            std::iter::once(NodeRef::Pattern(pattern))
                .chain(template.iter().map(|quad| NodeRef::Triple(&quad.triple))),
            memory,
        ),
        Query::Select { pattern, .. }
        | Query::Ask { pattern, .. }
        | Query::Describe { pattern, .. } => admit_with_memory([NodeRef::Pattern(pattern)], memory),
    }
}
