// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The plan arena: one evaluated algebra tree, numbered.
//!
//! A [`Tree`] numbers every node of the pattern it is built over with a [`NodeId`], and
//! that number is the node's ordinal in [`walk_spine`]'s pre-order: direct children first, in [`visit_classified_children`] order, then the
//! `EXISTS` bodies reached through the node's expressions. The per-node charge ledger
//! prints exactly that ordinal, so a ledger line, a pushdown ceiling and a prepared
//! `EXISTS` site all name a node by the same dense index. A subtree is the contiguous
//! id range `[id, subtree_end(id))`, and reverse id order is a valid bottom-up order.
//!
//! The numbering lives in a [`PlanShape`]: dense per-node tables with no lifetime, shared
//! by [`Arc`] with every forked worker and with the ledger. A [`Tree`] pairs a shape with
//! the source it numbers ([`Src`]) and with a [`TreeKey`] no other tree in the process
//! ever carries, which is what lets a table keyed by `(TreeKey, NodeId)` tell two trees
//! apart even when their shapes are identical.
//!
//! [`PlanShape::node_of`] maps a node's address back to its id. It is the bridge for
//! code that still reaches a node as a `&GraphPattern`: the tree's nodes are alive and
//! unmoved for as long as the tree borrows them, so an address names at most one of them.
//!
//! A [`PlanCache`] keeps one tree's key and shape for every evaluation of an algebra
//! tree its owner holds unchanged — a prepared plan, or a prepared execution's retained
//! substituted tree — so those evaluations number the tree, and compile its attached
//! expressions and prepare its `EXISTS` bodies, once rather than once each. Every node
//! but the root sits behind a heap allocation the owner never frees or replaces while
//! it holds the tree, so its address is the same on every evaluation; the root is held
//! inline by its owner and moves with it, so the shape keeps the root's address in a
//! cell each evaluation sets and resolves the root through that cell alone.
//!
//! [`visit_classified_children`]: crate::governor::soundness::visit_classified_children

use std::ops::Range;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use purrdf_sparql_algebra::{Expression, GraphPattern};

use crate::eval::PreparedExists;
use crate::governor::soundness::{
    ChildEdge, PATTERN_LABELS, PatternPart, pattern_label_index, visit_exists_patterns,
    visit_pattern_parts, walk_spine,
};
use crate::service_endpoints::EndpointScan;
use crate::vm::ExprProgram;

/// A node of one tree: its ordinal in the tree's pre-order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct NodeId(u32);

impl NodeId {
    /// The tree's root, which the pre-order visits first.
    pub(crate) const ROOT: Self = Self(0);

    /// The id as a table index.
    pub(crate) const fn index(self) -> usize {
        self.0 as usize
    }

    /// The id at table index `index`.
    ///
    /// # Panics
    ///
    /// When `index` does not fit the `u32` id space — a tree of more than four billion
    /// nodes, which no allocation that holds one can describe.
    pub(crate) fn from_index(index: usize) -> Self {
        Self(u32::try_from(index).expect("a plan's node count fits the u32 id space"))
    }
}

/// An expression attached directly to a node (a `FILTER`, a `BIND`, an `OPTIONAL`
/// condition, an `ORDER BY` key, an aggregate argument), numbered in the order
/// [`visit_pattern_parts`] yields them across the pre-order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct ExprId(u32);

/// A property-path node, numbered in pre-order among the tree's path nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the pre-order test asserts it; no evaluator path reads it"
    )
)]
pub(crate) struct PathId(u32);

/// An `EXISTS` body reached through an expression, numbered in pre-order among the
/// tree's `EXISTS` bodies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct SiteId(u32);

/// A node's algebra variant, as its index in the one label table
/// ([`PATTERN_LABELS`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NodeKind(u8);

impl NodeKind {
    /// The kind of `pattern`.
    pub(crate) const fn of(pattern: &GraphPattern) -> Self {
        Self(pattern_label_index(pattern) as u8)
    }

    /// The variant's stable label.
    pub(crate) const fn label(self) -> &'static str {
        PATTERN_LABELS[self.0 as usize]
    }
}

/// A process-unique tree identity. Issued from one monotonic counter and never reused,
/// so a `(TreeKey, NodeId)` pair recorded against one tree can never be read back as a
/// node of another.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct TreeKey(u64);

impl TreeKey {
    /// A key no tree has carried before.
    pub(crate) fn fresh() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        Self(NEXT.fetch_add(1, Ordering::Relaxed))
    }
}

/// One `EXISTS` body of the tree, and its preparation.
pub(crate) struct ExistsSite {
    /// The body's root node.
    body: NodeId,
    /// The attached expression the body is reached through.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "the pre-order test asserts it; no evaluator path reads it"
        )
    )]
    expr: ExprId,
    /// The body's prepare-time analysis, built on first use and kept for the life of the
    /// shape — one evaluation, or every evaluation of a tree a [`PlanCache`] keeps.
    prepared: OnceLock<Arc<PreparedExists>>,
}

/// One node's row of the shape: every per-node table the evaluator reads, and the ones
/// only the tests read, in one record so a tree's nodes cost one allocation.
#[derive(Clone)]
struct NodeRow {
    /// The node's algebra variant.
    kind: NodeKind,
    /// The node's depth below the root.
    depth: u32,
    /// The first id past the node's subtree.
    subtree_end: NodeId,
    /// The node's attached expressions, as a range of [`ExprId`]s.
    exprs: Range<u32>,
    /// The node's parent; `None` for the root.
    parent: Option<NodeId>,
    /// The address the node lived at when the shape was built.
    address: usize,
    /// The id of the node whose address ranks at this row's index among every node's
    /// address, ascending: read across the rows, the address index [`AddressIndex`]
    /// binary-searches.
    by_address: NodeId,
    /// The edge from the node's parent to it; [`ChildEdge::MONOTONE`] for the root.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "the pre-order test asserts it; no evaluator path reads it"
        )
    )]
    edge_in: ChildEdge,
}

/// One attached expression of the shape: the node it is attached to and its compiled
/// program.
struct ExprSlot {
    /// The node the expression is attached to.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "the pre-order test asserts it; no evaluator path reads it"
        )
    )]
    owner: NodeId,
    /// The expression's compiled program, compiled the first time the expression is
    /// evaluated.
    program: OnceLock<Arc<ExprProgram>>,
}

/// Node address to id, for code that still reaches a node by reference: a view of a
/// shape's rows, whose [`NodeRow::by_address`] column lists every node in ascending
/// address order, so a lookup is a binary search.
#[derive(Clone, Copy)]
pub(crate) struct AddressIndex<'s>(&'s [NodeRow]);

impl AddressIndex<'_> {
    /// The id of the node at `address`, when one of the indexed nodes lives there.
    pub(crate) fn get(self, address: usize) -> Option<NodeId> {
        let rows = self.0;
        rows.binary_search_by_key(&address, |row| rows[row.by_address.index()].address)
            .ok()
            .map(|rank| rows[rank].by_address)
    }
}

/// The dense per-node tables of one tree. Holds no reference into the tree it was built
/// from, so it is shared by [`Arc`] with forked workers and with the charge ledger.
pub(crate) struct PlanShape {
    /// Each node's row, indexed by [`NodeId`].
    rows: Vec<NodeRow>,
    /// Each attached expression's owner and compiled program, indexed by [`ExprId`].
    exprs: Vec<ExprSlot>,
    /// Each property-path node, indexed by [`PathId`].
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "the pre-order test asserts it; no evaluator path reads it"
        )
    )]
    paths: Vec<NodeId>,
    /// Every `EXISTS` body, indexed by [`SiteId`]; ascending by body id.
    sites: Vec<ExistsSite>,
    /// The tree's variable-endpoint `SERVICE` analysis, indexed by node.
    endpoints: EndpointScan,
    /// The address the root lives at for the evaluation running over this shape. The
    /// root's row keeps the address it had when the shape was built; this is where it is
    /// now, and is the only way the root is resolved. Set by
    /// [`Tree::build`] and, for a kept shape, by [`PlanCache::handle`] before each
    /// evaluation; every evaluation sharing one kept shape runs over the one tree its
    /// owner holds, so every one of them sets the same address.
    root: AtomicUsize,
    /// Per [`ExprId`], whether the expression's value positions are rewritten between
    /// evaluations of a kept tree, so its program is compiled per evaluation and never
    /// shared between evaluations. Empty when no expression is.
    volatile_exprs: Vec<bool>,
    /// Per [`ExprId`], the program last compiled for a volatile expression. The next
    /// evaluation compiles into it in place when no evaluation still holds it, so a
    /// volatile expression reuses its program's tables rather than allocating new ones.
    /// Empty when no expression is volatile.
    recycled: Vec<Mutex<Option<Arc<ExprProgram>>>>,
    /// Per [`SiteId`], whether the body's value positions are rewritten between
    /// evaluations of a kept tree, so its preparation is built per evaluation and never
    /// kept. Empty when no body is.
    volatile_sites: Vec<bool>,
}

impl PlanShape {
    /// The number of nodes.
    pub(crate) fn len(&self) -> usize {
        self.rows.len()
    }

    /// The id of the node at `pattern`'s address, when `pattern` is a node of this tree.
    pub(crate) fn node_of(&self, pattern: &GraphPattern) -> Option<NodeId> {
        self.node_at(std::ptr::from_ref(pattern) as usize)
    }

    /// The id of the node at `address`, when a node of this tree lives there.
    pub(crate) fn node_at(&self, address: usize) -> Option<NodeId> {
        if address == self.root.load(Ordering::Relaxed) {
            return Some(NodeId::ROOT);
        }
        AddressIndex(&self.rows)
            .get(address)
            .filter(|id| *id != NodeId::ROOT)
    }

    /// `node`'s algebra variant.
    pub(crate) fn kind(&self, node: NodeId) -> NodeKind {
        self.rows[node.index()].kind
    }

    /// `node`'s depth below the root.
    pub(crate) fn depth(&self, node: NodeId) -> u32 {
        self.rows[node.index()].depth
    }

    /// The first id past `node`'s subtree.
    pub(crate) fn subtree_end(&self, node: NodeId) -> NodeId {
        self.rows[node.index()].subtree_end
    }

    /// The site whose body is `body`, when `body` is an `EXISTS` body of this tree.
    pub(crate) fn site_of(&self, body: NodeId) -> Option<SiteId> {
        self.sites
            .binary_search_by_key(&body, |site| site.body)
            .ok()
            .map(|index| SiteId(index as u32))
    }

    /// The preparation of `site`, built by `build` the first time it is asked for.
    pub(crate) fn prepared_exists(
        &self,
        site: SiteId,
        build: impl FnOnce() -> PreparedExists,
    ) -> Arc<PreparedExists> {
        if self.volatile_sites.get(site.0 as usize).copied() == Some(true) {
            return Arc::new(build());
        }
        Arc::clone(
            self.sites[site.0 as usize]
                .prepared
                .get_or_init(|| Arc::new(build())),
        )
    }

    /// The program of `expr`, `node`'s attached expression at `position` (in the order
    /// the shape numbers them), compiled the first time it is asked for. A position past
    /// the node's attached expressions has no slot, and its program is compiled unkept.
    /// A volatile expression's program is compiled on every ask, into the tables of the
    /// one it last compiled when nothing holds that one any more.
    pub(crate) fn program(
        &self,
        node: NodeId,
        position: usize,
        expr: &Expression,
    ) -> Arc<ExprProgram> {
        let range = &self.rows[node.index()].exprs;
        if position >= range.len() {
            return Arc::new(ExprProgram::compile(expr));
        }
        let id = range.start as usize + position;
        if self.volatile_exprs.get(id).copied() == Some(true) {
            let mut kept = self.recycled[id]
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            if let Some(program) = kept.as_mut()
                && let Some(tables) = Arc::get_mut(program)
            {
                tables.recompile(expr);
                return Arc::clone(program);
            }
            let program = Arc::new(ExprProgram::compile(expr));
            *kept = Some(Arc::clone(&program));
            drop(kept);
            return program;
        }
        Arc::clone(
            self.exprs[id]
                .program
                .get_or_init(|| Arc::new(ExprProgram::compile(expr))),
        )
    }

    /// The tree's variable-endpoint `SERVICE` analysis.
    pub(crate) const fn endpoints(&self) -> &EndpointScan {
        &self.endpoints
    }

    /// The address map the tree was numbered with.
    #[cfg(test)]
    pub(crate) fn addresses(&self) -> AddressIndex<'_> {
        AddressIndex(&self.rows)
    }
}

/// A numbered algebra tree: a fresh [`TreeKey`] and the shape, borrowing the pattern it
/// numbers so the nodes stay alive and unmoved at the addresses the shape recorded.
pub(crate) struct Tree<'q> {
    key: TreeKey,
    shape: Arc<PlanShape>,
    numbered: std::marker::PhantomData<&'q GraphPattern>,
}

/// What an evaluation context carries of its tree: the key and the shape, without the
/// borrow of the source.
#[derive(Clone)]
pub(crate) struct PlanHandle {
    key: TreeKey,
    shape: Arc<PlanShape>,
}

impl PlanHandle {
    /// The tree's key.
    pub(crate) const fn key(&self) -> TreeKey {
        self.key
    }

    /// The tree's shape.
    pub(crate) const fn shape(&self) -> &Arc<PlanShape> {
        &self.shape
    }

    /// The id of `pattern` in this tree, when it is one of its nodes.
    pub(crate) fn node_of(&self, pattern: &GraphPattern) -> Option<NodeId> {
        self.shape.node_of(pattern)
    }
}

/// A node of [`Tree::build`]'s walk whose subtree is still being visited: its id, its
/// depth, and the edge (and, for an `EXISTS` body, the expression) of each of its
/// children in classified order, with how many of them have been visited.
struct Open {
    id: NodeId,
    depth: usize,
    visited: usize,
    children: purrdf_core::SmallVec<[(ChildEdge, Option<ExprId>); 4]>,
}

impl<'q> Tree<'q> {
    /// Number `root` and every node below it, `EXISTS` bodies included, in the order
    /// [`walk_spine`] visits them, and analyse its variable-endpoint `SERVICE` clauses
    /// against that numbering.
    ///
    /// The ids are `walk_spine`'s visit order by construction: this is that walk. A node's
    /// parent is the nearest open node one level up, and the node is that parent's next
    /// unvisited child, so its edge and its expression are read off the parent's own
    /// decomposition. The walk runs over a work list, so a tree of any height costs heap
    /// and never stack.
    pub(crate) fn build(root: &'q GraphPattern) -> Self {
        // A first walk counts the nodes and their attached expressions, so each table is
        // allocated once at its final size.
        let (mut node_count, mut expr_count) = (0_usize, 0_usize);
        walk_spine(root, &mut |node, _context, _level| {
            node_count += 1;
            visit_pattern_parts(node, &mut |part| {
                expr_count += usize::from(matches!(part, PatternPart::Expression(_)));
                false
            });
        });
        let mut rows: Vec<NodeRow> = Vec::with_capacity(node_count);
        let mut exprs: Vec<ExprSlot> = Vec::with_capacity(expr_count);
        let mut paths = Vec::new();
        let mut sites = Vec::new();
        // The walk's open ancestors are inline up to a shallow tree's depth.
        let mut open: purrdf_core::SmallVec<[Open; 8]> = purrdf_core::SmallVec::new();
        walk_spine(root, &mut |node, _context, level| {
            while open.last().is_some_and(|ancestor| ancestor.depth >= level) {
                open.pop();
            }
            let id = NodeId::from_index(rows.len());
            let (up, edge, via) = match open.last_mut() {
                Some(ancestor) => {
                    let (edge, via) = ancestor.children[ancestor.visited];
                    ancestor.visited += 1;
                    (Some(ancestor.id), edge, via)
                }
                None => (None, ChildEdge::MONOTONE, None),
            };
            if matches!(node, GraphPattern::Path { .. }) {
                paths.push(id);
            }
            if let Some(expr) = via {
                sites.push(ExistsSite {
                    body: id,
                    expr,
                    prepared: OnceLock::new(),
                });
            }
            let first_expr = exprs.len() as u32;
            let mut children = purrdf_core::SmallVec::new();
            visit_pattern_parts(node, &mut |part| {
                match part {
                    PatternPart::Child(_, edge) => children.push((edge, None)),
                    PatternPart::Expression(expr) => {
                        let expr_id = ExprId(exprs.len() as u32);
                        exprs.push(ExprSlot {
                            owner: id,
                            program: OnceLock::new(),
                        });
                        visit_exists_patterns(expr, &mut |_| {
                            children.push((ChildEdge::OPAQUE, Some(expr_id)));
                            false
                        });
                    }
                }
                false
            });
            rows.push(NodeRow {
                kind: NodeKind::of(node),
                depth: level as u32,
                // Every subtree is sized by the reverse sweep below; a leaf keeps this.
                subtree_end: NodeId::from_index(id.index() + 1),
                exprs: first_expr..exprs.len() as u32,
                parent: up,
                address: std::ptr::from_ref(node) as usize,
                by_address: id,
                edge_in: edge,
            });
            open.push(Open {
                id,
                depth: level,
                visited: 0,
                children,
            });
        });

        // Pre-order puts every node after its parent and its subtree contiguously after
        // it, so one reverse sweep extends each parent's subtree end over its children's.
        for index in (1..rows.len()).rev() {
            if let Some(p) = rows[index].parent {
                let end = rows[index].subtree_end;
                let parent = &mut rows[p.index()].subtree_end;
                *parent = (*parent).max(end);
            }
        }
        // The address order, ranked in a scratch list inline up to a small tree's size and
        // written into the rows' `by_address` column.
        let mut ranked: purrdf_core::SmallVec<[(usize, NodeId); 16]> = rows
            .iter()
            .map(|row| (row.address, row.by_address))
            .collect();
        ranked.sort_unstable_by_key(|(address, _)| *address);
        for (row, (_, id)) in rows.iter_mut().zip(ranked) {
            row.by_address = id;
        }

        let endpoints = crate::service_endpoints::scan(root, &AddressIndex(&rows), rows.len());
        Self {
            key: TreeKey::fresh(),
            shape: Arc::new(PlanShape {
                rows,
                exprs,
                paths,
                sites,
                endpoints,
                root: AtomicUsize::new(std::ptr::from_ref(root) as usize),
                volatile_exprs: Vec::new(),
                recycled: Vec::new(),
                volatile_sites: Vec::new(),
            }),
            numbered: std::marker::PhantomData,
        }
    }

    /// The tree's shape.
    pub(crate) const fn shape(&self) -> &Arc<PlanShape> {
        &self.shape
    }

    /// What an evaluation context keeps of this tree.
    pub(crate) fn handle(&self) -> PlanHandle {
        PlanHandle {
            key: self.key,
            shape: Arc::clone(&self.shape),
        }
    }
}

/// The expressions attached directly to `node`, in the order the shape numbers them.
fn attached(node: &GraphPattern) -> purrdf_core::SmallVec<[&Expression; 4]> {
    let mut exprs = purrdf_core::SmallVec::new();
    visit_pattern_parts(node, &mut |part| {
        if let PatternPart::Expression(expr) = part {
            exprs.push(expr);
        }
        false
    });
    exprs
}

/// One tree's key and shape, kept by the owner of an algebra tree it holds unchanged
/// and shared by every evaluation of that tree. See the module docs.
///
/// A clone starts empty: the clone's nodes live at other addresses.
#[derive(Default)]
pub(crate) struct PlanCache {
    kept: OnceLock<PlanHandle>,
}

impl Clone for PlanCache {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl std::fmt::Debug for PlanCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlanCache")
            .field("nodes", &self.kept.get().map(|kept| kept.shape.len()))
            .finish()
    }
}

impl PlanCache {
    /// A cache over `root`, a tree whose value positions are rewritten between
    /// evaluations, `moved` being the same tree with every one of those positions
    /// holding a different value. An attached expression or an `EXISTS` body that
    /// differs between the two holds such a position, and its program or preparation
    /// is built per evaluation rather than kept.
    ///
    /// # Panics
    ///
    /// When `moved` is not numbered into as many nodes, expressions and sites as `root`
    /// — two trees that differ in more than their values.
    pub(crate) fn varying(root: &GraphPattern, moved: &GraphPattern) -> Self {
        let tree = Tree::build(root);
        // The tree's nodes, id for id: its numbering is this walk's pre-order.
        let mut nodes: Vec<&GraphPattern> = Vec::with_capacity(tree.shape.len());
        walk_spine(root, &mut |node, _context, _level| nodes.push(node));
        // `moved` has `root`'s structure, so its nodes in the same pre-order are the
        // tree's nodes, id for id.
        let mut others: Vec<&GraphPattern> = Vec::with_capacity(tree.shape.len());
        walk_spine(moved, &mut |node, _context, _level| others.push(node));
        assert!(
            others.len() == tree.shape.len(),
            "a retained tree and its value-moved copy number the same nodes"
        );
        let expr_count = tree.shape.exprs.len();
        let mut exprs: Vec<bool> = Vec::new();
        for (index, other) in others.iter().enumerate() {
            let here = attached(nodes[index]);
            let there = attached(other);
            assert!(
                here.len() == there.len(),
                "a retained tree and its value-moved copy attach the same expressions"
            );
            let first = tree.shape.rows[index].exprs.start as usize;
            for (offset, (a, b)) in here.iter().zip(&there).enumerate() {
                if a != b {
                    if exprs.is_empty() {
                        exprs = vec![false; expr_count];
                    }
                    exprs[first + offset] = true;
                }
            }
        }
        let mut sites: Vec<bool> = Vec::new();
        for (index, site) in tree.shape.sites.iter().enumerate() {
            if nodes[site.body.index()] != others[site.body.index()] {
                if sites.is_empty() {
                    sites = vec![false; tree.shape.sites.len()];
                }
                sites[index] = true;
            }
        }
        drop((nodes, others));
        let Tree { key, mut shape, .. } = tree;
        let table = Arc::get_mut(&mut shape).expect("a tree just built shares its shape with none");
        if !exprs.is_empty() {
            table.recycled = exprs.iter().map(|_| Mutex::new(None)).collect();
            table.volatile_exprs = exprs;
        }
        table.volatile_sites = sites;
        let kept = OnceLock::new();
        let _ = kept.set(PlanHandle { key, shape });
        Self { kept }
    }

    /// The kept tree's handle for an evaluation over `root`, the tree this cache's owner
    /// holds — numbered on the first call, and pointed at where `root` lives now.
    pub(crate) fn handle(&self, root: &GraphPattern) -> PlanHandle {
        let kept = self.kept.get_or_init(|| Tree::build(root).handle());
        debug_assert!(
            kept.shape.kind(NodeId::ROOT) == NodeKind::of(root),
            "a plan cache is only ever asked about the tree its owner holds"
        );
        kept.shape
            .root
            .store(std::ptr::from_ref(root) as usize, Ordering::Relaxed);
        kept.clone()
    }
}

#[cfg(test)]
mod tests {
    //! The shape's numbering is `walk_spine`'s pre-order, over generated algebra and over
    //! every query the repository's SPARQL suites parse.

    use purrdf_sparql_algebra::GraphPattern;

    use super::{ExprId, NodeId, PathId, PlanShape, SiteId, Tree};
    use crate::governor::soundness::{
        PatternPart, pattern_label, visit_classified_children, visit_pattern_parts, walk_spine,
    };

    impl PlanShape {
        /// `node`'s children with the edge reaching each, in id order: every node whose
        /// parent row names `node`.
        fn children_of(&self, node: NodeId) -> Vec<(NodeId, super::ChildEdge)> {
            self.rows
                .iter()
                .enumerate()
                .filter(|(_, row)| row.parent == Some(node))
                .map(|(index, row)| (NodeId::from_index(index), row.edge_in))
                .collect()
        }
    }

    /// Assert every table of `root`'s shape against `walk_spine` and the two visitors.
    fn assert_shape_is_walk_spine(root: &GraphPattern, context: &str) {
        let tree = Tree::build(root);
        let shape = tree.shape();
        let mut spine: Vec<(&GraphPattern, usize)> = Vec::new();
        walk_spine(root, &mut |node, _context, depth| spine.push((node, depth)));
        assert_eq!(shape.len(), spine.len(), "{context}: node count");
        let mut expected_exprs = 0_u32;
        let mut expected_paths = 0_u32;
        // Every `EXISTS` body with the node whose expression reaches it, gathered over
        // the walk and numbered once it is done.
        let mut bodies: Vec<(NodeId, NodeId)> = Vec::new();
        for (index, (node, depth)) in spine.iter().enumerate() {
            let id = NodeId::from_index(index);
            assert_eq!(
                shape.rows[index].address,
                std::ptr::from_ref(*node) as usize,
                "{context}: node {index} is walk_spine's node {index}"
            );
            assert_eq!(shape.node_of(node), Some(id), "{context}: bridge {index}");
            assert_eq!(shape.depth(id) as usize, *depth, "{context}: depth {index}");
            assert_eq!(shape.kind(id).label(), pattern_label(node), "{context}");

            // Children: the classified children, in order, each one's parent this node,
            // and the subtree the contiguous range its pre-order says it is.
            let mut expected: Vec<(&GraphPattern, super::ChildEdge)> = Vec::new();
            visit_classified_children(node, &mut |child, edge| {
                expected.push((child, edge));
                false
            });
            let children = shape.children_of(id);
            assert_eq!(
                children.len(),
                expected.len(),
                "{context}: children {index}"
            );
            let mut next = index + 1;
            for ((child, edge), (expected_child, expected_edge)) in children.iter().zip(&expected) {
                assert!(
                    std::ptr::eq(spine[child.index()].0, *expected_child),
                    "{context}"
                );
                assert_eq!(edge, expected_edge, "{context}: edge under {index}");
                assert_eq!(shape.rows[child.index()].parent, Some(id), "{context}");
                assert_eq!(child.index(), next, "{context}: children are contiguous");
                next = shape.subtree_end(*child).index();
            }
            assert_eq!(
                shape.subtree_end(id).index(),
                next,
                "{context}: subtree end"
            );

            // Attached expressions and the EXISTS sites reached through them.
            let mut exprs = 0_u32;
            visit_pattern_parts(node, &mut |part| {
                if matches!(part, PatternPart::Expression(_)) {
                    exprs += 1;
                }
                false
            });
            let range = shape.rows[index].exprs.clone();
            assert_eq!(range, expected_exprs..expected_exprs + exprs, "{context}");
            for expr in range {
                assert_eq!(shape.exprs[expr as usize].owner, id, "{context}");
            }
            expected_exprs += exprs;
            let direct = {
                let mut direct = 0_usize;
                visit_pattern_parts(node, &mut |part| {
                    if matches!(part, PatternPart::Child(..)) {
                        direct += 1;
                    }
                    false
                });
                direct
            };
            for (child, _) in &children[direct..] {
                bodies.push((*child, id));
            }
            for (child, _) in &children[..direct] {
                assert_eq!(shape.site_of(*child), None, "{context}: a direct child");
            }
            if matches!(node, GraphPattern::Path { .. }) {
                assert_eq!(shape.paths[expected_paths as usize], id, "{context}");
                assert_eq!(PathId(expected_paths).0, expected_paths);
                expected_paths += 1;
            }
        }
        // Sites are numbered in ascending body id — the pre-order of the bodies
        // themselves, which `site_of`'s binary search relies on. A body nested inside an
        // earlier sibling body therefore takes a lower site id than that sibling's later
        // siblings, so the numbering is not the order of the owning nodes.
        bodies.sort_by_key(|(body, _)| body.index());
        for (rank, (body, owner)) in bodies.iter().enumerate() {
            let site = shape.site_of(*body).expect("an EXISTS body is a site");
            assert_eq!(
                site,
                SiteId(u32::try_from(rank).expect("the rank fits")),
                "{context}: site order"
            );
            assert_eq!(shape.sites[site.0 as usize].body, *body, "{context}");
            let expr_owner = shape.exprs[shape.sites[site.0 as usize].expr.0 as usize].owner;
            assert_eq!(
                expr_owner, *owner,
                "{context}: a site's expression is its parent's"
            );
        }
        let expected_sites = bodies.len();
        assert_eq!(shape.exprs.len(), expected_exprs as usize, "{context}");
        assert_eq!(ExprId(expected_exprs).0 as usize, shape.exprs.len());
        assert_eq!(shape.paths.len(), expected_paths as usize, "{context}");
        assert_eq!(shape.sites.len(), expected_sites, "{context}");
        assert_eq!(
            shape.rows[0].parent, None,
            "{context}: the root has no parent"
        );
    }

    #[test]
    fn the_shape_numbers_generated_algebra_in_walk_spine_order() {
        let mut choices = crate::service_endpoints::walk_tests::Choices { state: 0x5EED_0B1D };
        for shape in 0..300 {
            let mut budget = 32;
            let root = crate::service_endpoints::walk_tests::pattern(&mut choices, &mut budget);
            assert_shape_is_walk_spine(&root, &format!("generated shape {shape}"));
        }
    }

    #[test]
    fn the_shape_numbers_every_suite_query_in_walk_spine_order() {
        let suites = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../sparql-conformance");
        let mut files = Vec::new();
        let mut dirs = vec![suites.join("suite"), suites.join("corpus")];
        while let Some(dir) = dirs.pop() {
            for entry in std::fs::read_dir(&dir).expect("the conformance suites are present") {
                let path = entry.expect("a directory entry").path();
                if path.is_dir() {
                    dirs.push(path);
                } else if path.extension().is_some_and(|ext| ext == "rq") {
                    files.push(path);
                }
            }
        }
        files.sort();
        let mut parsed = 0_usize;
        for file in &files {
            let text = std::fs::read_to_string(file).expect("a query file is UTF-8");
            // Negative-syntax cases and queries outside the parser's language are not
            // algebra; the rest are.
            let Ok(query) = purrdf_sparql_algebra::SparqlParser::new().parse_query(&text) else {
                continue;
            };
            parsed += 1;
            assert_shape_is_walk_spine(
                crate::eval::query_pattern(&query),
                &file.display().to_string(),
            );
        }
        assert!(parsed > 500, "the suites parse into a corpus: {parsed}");
    }

    #[test]
    fn a_tree_key_is_never_reused() {
        let root = GraphPattern::Bgp {
            patterns: Vec::new(),
        };
        let first = Tree::build(&root);
        let second = Tree::build(&root);
        assert_ne!(first.handle().key(), second.handle().key());
        assert_eq!(first.handle().key(), first.key);
        assert!(std::sync::Arc::ptr_eq(
            first.handle().shape(),
            first.shape()
        ));
    }
}
