// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! A hash-consed arena of compound terms: function-symbol applications and simple
//! binders over locally-nameless (de Bruijn) bound variables, plus first-class
//! unification metavariables.
//!
//! # Why this exists beside the flat quad IR
//!
//! [`crate::clause`]'s `triple(s, p, o, g)` is exactly wide enough for the
//! semi-naive fixpoint and the restricted chase, and no wider: every atom is the
//! same arity-4 shape, so nothing in that IR needs a node that stands for a
//! function symbol applied to an argument LIST, or for a variable bound by a
//! quantifier rather than named by the planner. [`resolve_fol`](crate::resolve_fol)'s
//! SLG-tabled backward resolver needs exactly that richer structure — an atom is a
//! compound term there, not a fixed 4-tuple — and a future description-logic layer
//! built on top of this receiving surface will need genuinely compound concept
//! terms (`∃r.(C ⊓ D)` is not expressible as a quad at any arity). So this module is
//! a second, independent term representation, not a replacement for the first: the
//! quad IR stays the shape the evaluator and the chase reason over, and this arena
//! is the shape the goal-directed resolver reasons over.
//!
//! # Hash-consing
//!
//! [`TermDag`] follows exactly the pattern [`crate::proof::ProofArena`] establishes:
//! every node is interned by content hash into a [`hashbrown::HashTable`], so two
//! structurally identical terms — including two ALPHA-EQUIVALENT binders, since the
//! locally-nameless encoding makes alpha-equivalent terms literally identical
//! structure — collapse to the same [`NodeId`]. That collapse is what makes
//! unification's fast path (`a == b` after resolving through the substitution) a
//! single id comparison rather than a structural walk, and what keeps
//! [`unify::apply`](crate::unify::apply) linear in the DAG rather than exponential
//! in a term's sharing.
//!
//! # Locally-nameless binding
//!
//! A bound variable is [`NodeData::Bound { debruijn, slot }`]: `debruijn` counts
//! enclosing [`NodeData::Binder`]s outward from the occurrence (0 is the innermost),
//! and `slot` selects among that binder's simultaneously-bound positions. There is
//! no separately-named bound variable to rename, which is exactly what makes two
//! differently-authored but alpha-equivalent terms hash-cons to one node. A
//! [`NodeData::Free`] variable is the opposite kind of thing — a named,
//! non-unifiable placeholder, useful as a de Bruijn root or a rigid constant-like
//! variable — and a [`NodeData::Meta`] is the third, unifiable kind:
//! [`crate::unify`] binds metavariables and never touches `Free` or `Bound` nodes.
//!
//! # Free-metavariable caching
//!
//! Every node caches the sorted, deduplicated set of [`MetaId`]s it mentions,
//! computed bottom-up at intern time from its children's already-cached sets. This
//! makes [`TermDag::free_meta`] O(1) and gives the occurs-check
//! ([`crate::unify`]'s `occurs_through`) a binary-searchable fast path instead of a
//! structural walk on every call.

use hashbrown::HashTable;

use crate::id::{MetaId, NodeId, SymId};

/// One node's shape.
///
/// A `Leaf`/`Free`/`Meta`/`Bound` node is a LEAF of the term structure (it has no
/// children in this enum's own sense, though a `Meta` may later be bound to
/// arbitrary structure by [`crate::unify`]); `App` and `Binder` are the two ways
/// this arena builds compound structure.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NodeData {
    /// An interned atomic constant (an IRI or a rendered literal surface) — not
    /// unifiable, but comparable and shareable.
    Leaf(SymId),
    /// A named free (non-unifiable) variable, e.g. a de Bruijn placeholder root.
    Free(SymId),
    /// A unification metavariable.
    Meta(MetaId),
    /// A locally-bound occurrence: `debruijn` counts enclosing binders outward from
    /// this occurrence (0 = the innermost), `slot` selects among that binder's
    /// simultaneously-bound positions.
    Bound {
        /// The number of enclosing binders crossed to reach the one this
        /// occurrence refers to, counting outward from 0 at the innermost.
        debruijn: u32,
        /// Which of that binder's simultaneously-bound positions this is.
        slot: u16,
    },
    /// A function/predicate application `op(args...)`.
    App {
        /// The applied operator (typically a `Leaf` naming a function symbol).
        op: NodeId,
        /// The argument nodes, in authored order.
        args: Vec<NodeId>,
    },
    /// A binder introducing `sorts.len()` simultaneously-bound positions over `body`.
    Binder {
        /// The binder's own operator (e.g. a quantifier symbol).
        op: NodeId,
        /// One sort node per simultaneously-bound position, in slot order.
        sorts: Vec<NodeId>,
        /// The bound body, one de Bruijn level deeper than the binder itself.
        body: NodeId,
    },
}

/// The interning hash of one node's shape.
///
/// Fixed-key `ahash`, exactly as [`crate::proof::ProofArena`]'s `term_hash` uses:
/// seeded from constants rather than ambient entropy, which does not exist on
/// `wasm32-unknown-unknown`. The table this feeds is never iterated.
fn node_hash(data: &NodeData) -> u64 {
    use core::hash::{Hash, Hasher};
    let mut hasher = ahash::AHasher::default();
    data.hash(&mut hasher);
    hasher.finish()
}

/// The interning hash of one symbol string.
fn symbol_hash(symbol: &str) -> u64 {
    use core::hash::{Hash, Hasher};
    let mut hasher = ahash::AHasher::default();
    symbol.hash(&mut hasher);
    hasher.finish()
}

/// Merge two already-sorted, already-deduplicated `MetaId` slices into one sorted,
/// deduplicated `Vec`.
fn merge_free_meta(left: &[MetaId], right: &[MetaId]) -> Vec<MetaId> {
    let mut out = Vec::with_capacity(left.len() + right.len());
    out.extend_from_slice(left);
    out.extend_from_slice(right);
    out.sort_unstable();
    out.dedup();
    out
}

/// A hash-consed arena of compound terms.
///
/// See the [module docs](self) for the shape this holds and why it exists beside
/// [`crate::clause`]'s flat quad IR. Owned, never global: two callers building
/// unrelated terms use two arenas, exactly as two [`crate::proof::ProofArena`]s
/// never share ids.
#[derive(Debug, Clone, Default)]
pub struct TermDag {
    /// The interned nodes, indexed by [`NodeId`] slot.
    nodes: Vec<NodeData>,
    /// Content hash → id, for O(1) node interning. Probed, never iterated.
    by_content: HashTable<NodeId>,
    /// Each node's sorted, deduplicated free-metavariable set, indexed in lockstep
    /// with `nodes`.
    free_meta: Vec<Vec<MetaId>>,
    /// The interned symbol strings, indexed by [`SymId`] slot.
    symbols: Vec<String>,
    /// Content hash → id, for O(1) symbol interning. Probed, never iterated.
    symbols_by_content: HashTable<SymId>,
    /// The number of metavariables minted so far — [`MetaId`]s are dense and
    /// assigned in mint order, independent of node interning order (a `Meta` node
    /// is interned once per DISTINCT metavariable, but a metavariable itself is
    /// minted exactly once, by [`Self::fresh_meta`]).
    next_meta: u32,
}

impl TermDag {
    /// A fresh, empty arena.
    pub fn new() -> Self {
        Self::default()
    }

    /// The number of interned nodes.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Whether the arena holds no nodes.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// The node at `node`.
    ///
    /// # Panics
    ///
    /// Panics if `node` was not minted by this arena. Ids are per-arena handles,
    /// exactly as `crate::proof::ProofArena::term`'s are, so a foreign id is a
    /// programming error rather than a data state.
    pub fn data(&self, node: NodeId) -> &NodeData {
        self.nodes.get(node.index()).unwrap_or_else(|| {
            panic!(
                "NodeId {node:?} was not minted by this arena (len {}): term ids are \
                 per-arena handles and must never cross arena boundaries",
                self.nodes.len()
            )
        })
    }

    /// `node`'s sorted, deduplicated set of free metavariables — O(1), since it is
    /// cached bottom-up at intern time.
    ///
    /// # Panics
    ///
    /// Panics if `node` was not minted by this arena.
    pub fn free_meta(&self, node: NodeId) -> &[MetaId] {
        self.free_meta.get(node.index()).unwrap_or_else(|| {
            panic!(
                "NodeId {node:?} was not minted by this arena (len {}): term ids are \
                 per-arena handles and must never cross arena boundaries",
                self.nodes.len()
            )
        })
    }

    /// Mint a fresh metavariable, and intern the [`NodeData::Meta`] node that
    /// denotes it.
    ///
    /// A fresh metavariable is, by construction, distinct from every metavariable
    /// minted before it, so its node's free-meta set is exactly `{itself}` and can
    /// never coincide with an earlier node's cache through hash-consing.
    pub fn fresh_meta(&mut self) -> (MetaId, NodeId) {
        let meta = MetaId::from_index(self.next_meta as usize);
        self.next_meta += 1;
        let node = self.intern(NodeData::Meta(meta));
        (meta, node)
    }

    /// Intern a symbol string, returning its dense [`SymId`].
    pub fn intern_symbol(&mut self, symbol: &str) -> SymId {
        let hash = symbol_hash(symbol);
        let symbols = &self.symbols;
        if let Some(&id) = self
            .symbols_by_content
            .find(hash, |&id| symbols[id.index()] == symbol)
        {
            return id;
        }
        let id = SymId::from_index(self.symbols.len());
        self.symbols.push(symbol.to_owned());
        let symbols = &self.symbols;
        self.symbols_by_content
            .insert_unique(hash, id, |&id| symbol_hash(&symbols[id.index()]));
        id
    }

    /// The symbol string a [`SymId`] addresses.
    ///
    /// # Panics
    ///
    /// Panics if `id` was not minted by this arena's symbol interner.
    pub fn symbol(&self, id: SymId) -> &str {
        self.symbols.get(id.index()).unwrap_or_else(|| {
            panic!(
                "SymId {id:?} was not minted by this arena (len {}): symbol ids are \
                 per-arena handles and must never cross arena boundaries",
                self.symbols.len()
            )
        })
    }

    /// Intern a [`NodeData::Leaf`] naming `symbol`.
    pub fn intern_leaf(&mut self, symbol: &str) -> NodeId {
        let sym = self.intern_symbol(symbol);
        self.intern(NodeData::Leaf(sym))
    }

    /// Intern a [`NodeData::Free`] naming `symbol`.
    pub fn intern_free(&mut self, symbol: &str) -> NodeId {
        let sym = self.intern_symbol(symbol);
        self.intern(NodeData::Free(sym))
    }

    /// Intern a [`NodeData::Bound`] occurrence.
    pub fn intern_bound(&mut self, debruijn: u32, slot: u16) -> NodeId {
        self.intern(NodeData::Bound { debruijn, slot })
    }

    /// Intern an [`NodeData::App`] node.
    pub fn intern_app(&mut self, op: NodeId, args: Vec<NodeId>) -> NodeId {
        self.intern(NodeData::App { op, args })
    }

    /// Intern a [`NodeData::Binder`] node.
    pub fn intern_binder(&mut self, op: NodeId, sorts: Vec<NodeId>, body: NodeId) -> NodeId {
        self.intern(NodeData::Binder { op, sorts, body })
    }

    /// This node's free-metavariable set, computed from already-interned children.
    ///
    /// Called only from [`Self::intern`], after any child node the shape mentions
    /// is already resident (a precondition every public `intern_*` constructor
    /// upholds, since a caller can only ever hold a `NodeId` this arena already
    /// minted).
    fn compute_free_meta(&self, data: &NodeData) -> Vec<MetaId> {
        match data {
            NodeData::Leaf(_) | NodeData::Free(_) | NodeData::Bound { .. } => Vec::new(),
            NodeData::Meta(meta) => vec![*meta],
            NodeData::App { op, args } => {
                let mut acc = self.free_meta(*op).to_vec();
                for &arg in args {
                    acc = merge_free_meta(&acc, self.free_meta(arg));
                }
                acc
            }
            // Binders don't bind metavariables, only de Bruijn positions, so a
            // binder's free-meta set is exactly its children's union too.
            NodeData::Binder { op, sorts, body } => {
                let mut acc = self.free_meta(*op).to_vec();
                for &sort in sorts {
                    acc = merge_free_meta(&acc, self.free_meta(sort));
                }
                acc = merge_free_meta(&acc, self.free_meta(*body));
                acc
            }
        }
    }

    /// Intern `data`, returning the existing id if an identical node is already
    /// held, and caching its free-metavariable set on first insertion.
    fn intern(&mut self, data: NodeData) -> NodeId {
        let hash = node_hash(&data);
        let nodes = &self.nodes;
        if let Some(&id) = self.by_content.find(hash, |&id| nodes[id.index()] == data) {
            return id;
        }
        let cache = self.compute_free_meta(&data);
        let id = NodeId::from_index(self.nodes.len());
        self.nodes.push(data);
        self.free_meta.push(cache);
        let nodes = &self.nodes;
        self.by_content
            .insert_unique(hash, id, |&id| node_hash(&nodes[id.index()]));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Interning the same leaf symbol twice yields the same `NodeId`.
    #[test]
    fn hash_consing_collapses_identical_leaves() {
        let mut dag = TermDag::new();
        let a = dag.intern_leaf("https://example.org/a");
        let b = dag.intern_leaf("https://example.org/a");
        assert_eq!(a, b);
        assert_eq!(dag.len(), 1, "one leaf, one node");
    }

    /// Interning the same application twice yields the same `NodeId`, and a
    /// differently-shaped application is a distinct node.
    #[test]
    fn hash_consing_collapses_identical_applications() {
        let mut dag = TermDag::new();
        let f = dag.intern_leaf("f");
        let a = dag.intern_leaf("a");
        let b = dag.intern_leaf("b");
        let first = dag.intern_app(f, vec![a, b]);
        let second = dag.intern_app(f, vec![a, b]);
        assert_eq!(first, second);
        let different = dag.intern_app(f, vec![b, a]);
        assert_ne!(first, different, "argument order is part of the shape");
    }

    /// Interning the same binder twice yields the same `NodeId`.
    #[test]
    fn hash_consing_collapses_identical_binders() {
        let mut dag = TermDag::new();
        let forall = dag.intern_leaf("forall");
        let sort = dag.intern_leaf("thing");
        let body = dag.intern_bound(0, 0);
        let first = dag.intern_binder(forall, vec![sort], body);
        let second = dag.intern_binder(forall, vec![sort], body);
        assert_eq!(first, second);
    }

    /// A fresh metavariable's own node has free-meta set `{itself}`, and no two
    /// calls to `fresh_meta` ever mint the same `MetaId`.
    #[test]
    fn fresh_meta_is_unique_and_self_free() {
        let mut dag = TermDag::new();
        let (m1, n1) = dag.fresh_meta();
        let (m2, n2) = dag.fresh_meta();
        assert_ne!(m1, m2);
        assert_ne!(n1, n2);
        assert_eq!(dag.free_meta(n1), &[m1]);
        assert_eq!(dag.free_meta(n2), &[m2]);
    }

    /// An `App` node's free-meta set is the sorted, deduplicated union of its
    /// children's — including the operator position.
    #[test]
    fn app_free_meta_is_the_union_of_children() {
        let mut dag = TermDag::new();
        let f = dag.intern_leaf("f");
        let (m1, meta1) = dag.fresh_meta();
        let (m2, meta2) = dag.fresh_meta();
        let app = dag.intern_app(f, vec![meta1, meta2, meta1]);
        let mut expected = [m1, m2];
        expected.sort_unstable();
        assert_eq!(dag.free_meta(app), &expected);
    }

    /// A `Binder`'s free-meta set excludes nothing: binders scope de Bruijn
    /// positions, not metavariables, so its cache is exactly its children's union,
    /// the same rule an `App` follows.
    #[test]
    fn binder_free_meta_excludes_nothing() {
        let mut dag = TermDag::new();
        let op = dag.intern_leaf("forall");
        let sort = dag.intern_leaf("thing");
        let (m, meta) = dag.fresh_meta();
        let binder = dag.intern_binder(op, vec![sort], meta);
        assert_eq!(dag.free_meta(binder), &[m]);
    }

    /// A leaf, a free variable and a bound occurrence all have empty free-meta
    /// sets — none of them mentions a metavariable.
    #[test]
    fn leaves_free_variables_and_bound_occurrences_have_no_free_meta() {
        let mut dag = TermDag::new();
        let leaf = dag.intern_leaf("a");
        let free = dag.intern_free("x");
        let bound = dag.intern_bound(0, 0);
        assert_eq!(dag.free_meta(leaf), []);
        assert_eq!(dag.free_meta(free), []);
        assert_eq!(dag.free_meta(bound), []);
    }

    /// `symbol`/`intern_symbol` round-trip, and interning the same string twice
    /// yields the same `SymId`.
    #[test]
    fn symbol_interning_round_trips() {
        let mut dag = TermDag::new();
        let id = dag.intern_symbol("https://example.org/p");
        let again = dag.intern_symbol("https://example.org/p");
        assert_eq!(id, again);
        assert_eq!(dag.symbol(id), "https://example.org/p");
    }

    /// A `Free` variable and a `Leaf` over the same symbol text are distinct
    /// nodes: they occupy the same symbol interner but different `NodeData` shapes.
    #[test]
    fn free_and_leaf_over_the_same_symbol_are_distinct_nodes() {
        let mut dag = TermDag::new();
        let leaf = dag.intern_leaf("x");
        let free = dag.intern_free("x");
        assert_ne!(leaf, free);
    }

    /// `len`/`is_empty` track the number of DISTINCT interned nodes.
    #[test]
    fn len_and_is_empty_track_distinct_nodes() {
        let mut dag = TermDag::new();
        assert!(dag.is_empty());
        let a = dag.intern_leaf("a");
        assert_eq!(dag.len(), 1);
        let again = dag.intern_leaf("a");
        assert_eq!(a, again, "re-interning the same leaf returns the same id");
        assert_eq!(dag.len(), 1, "re-interning must not grow the arena");
        let _ = dag.intern_leaf("b");
        assert_eq!(dag.len(), 2);
    }

    /// A `Meta` node minted via `fresh_meta` is itself hash-consed like any other
    /// node: re-interning the SAME `NodeData::Meta(m)` (which only `fresh_meta`
    /// ever constructs, so this is exercised through the id crate's own contract)
    /// never happens by construction, but the node/id pairing returned is
    /// internally consistent.
    #[test]
    fn fresh_meta_node_resolves_back_to_a_meta_shape() {
        let mut dag = TermDag::new();
        let (meta, node) = dag.fresh_meta();
        assert_eq!(dag.data(node), &NodeData::Meta(meta));
    }
}
