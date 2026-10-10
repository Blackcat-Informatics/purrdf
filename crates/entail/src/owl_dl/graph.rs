// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The completion graph, and the two-domain semantics every rule over it obeys.
//!
//! This is the state a `SHOIQ(D)` decision procedure searches over, factored out of the
//! procedures themselves because there are two of them: the clause-driven
//! [`hyper`](crate::owl_dl::hyper) hypertableau that decides every question this crate
//! asks, and the concept-tree [`tableau`](crate::owl_dl::tableau) kept as its differential
//! reference. Both build THE SAME graph — same node identity, same merges, same
//! distinctness, same concrete domain — so a verdict difference between them is a
//! difference of CALCULUS and never of bookkeeping. That is what makes the differential
//! test evidence rather than a comparison of two spellings.
//!
//! ## Two domains, not one
//!
//! OWL 2 interprets an ontology over an object domain `Δ_I` — what `owl:Thing` denotes — and
//! a disjoint data domain `Δ_D` of literal values. A node inhabits one or the other
//! ([`Node::concrete`]), and the difference is load-bearing in two places: a concrete node is
//! NOT seeded with the internalized TBox, because a general concept inclusion quantifies over
//! `Δ_I` alone; and a concrete node's constraints are decided by [`crate::owl_dl::data`]
//! against the XSD value spaces rather than by the abstract rules. Two literals are one
//! element of `Δ_D` exactly when they denote one VALUE — the data domain has no unique-name
//! freedom to spend — which is what lets a functional data property clash on
//! `"1"^^xsd:integer` and `"2"^^xsd:integer` while accepting `"1"^^xsd:integer` and
//! `"01"^^xsd:integer`.
//!
//! ## No unique name assumption
//!
//! OWL 2 does not assume distinct names denote distinct elements. Nominals are therefore
//! handled by *identification*, never by name comparison: `{a} ∈ L(x)` merges `x` with `a`'s
//! root whatever `x` is already called. Two named individuals become distinct only when
//! something forces it — an explicit `≠` recorded by the `≥`-rule or by
//! `owl:differentFrom`, or a `¬{a}` in a label — and only then can a nominal constraint
//! clash. [`Graph::merge_nodes`] is the one place identification happens, so neither
//! calculus can grow a second, name-comparing answer to the same question.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

use crate::owl_dl::Kb;
use crate::owl_dl::clause::{BodyAtom, TransitivePatterns};
use crate::owl_dl::concept::{Decomp, Role};

/// A single completion-graph node.
#[derive(Clone)]
pub(crate) struct Node {
    /// The concept-id label set (ordered; drives no result via hash iteration).
    pub(crate) label: BTreeSet<u32>,
    /// The generating predecessor (tree parent); `None` for root/nominal nodes.
    pub(crate) parent: Option<usize>,
    /// The role `(property, inverted)` on the edge from `parent` to this node.
    pub(crate) incoming: Option<(u32, bool)>,
    /// Whether this is a root node — never blocked. NOTE: a root is not necessarily a NOMINAL
    /// (an anonymous `fresh_types` witness is an unblocked root with no nominal identity); the
    /// nominal-introduction rules gate on [`Node::nominal_id`], not on this flag.
    pub(crate) root: bool,
    /// The stable nominal identity this node denotes, when it is a NOMINAL root (named or
    /// generated). `None` for anonymous roots and blockable tree nodes. Drives the
    /// nominal-introduction (`NN`/`NI`) trigger and supplies the `u` of a reserved root.
    pub(crate) nominal_id: Option<NominalId>,
    /// The individual term ids this node denotes.
    ///
    /// A root starts out denoting exactly the one individual it was created for, but
    /// OWL 2 makes **no unique name assumption**: two names may denote the same
    /// element, and identification merges the two nodes. A merge therefore *unions* the
    /// two sets, so a node can end up denoting several names. Empty for anonymous tree
    /// nodes.
    pub(crate) nominals: BTreeSet<u32>,
    /// Nodes this node is forced to be distinct from (`≠`), by node index.
    pub(crate) neq: BTreeSet<usize>,
    /// Union-find forward pointer once merged away (`None` while a representative).
    pub(crate) merged: Option<usize>,
    /// Whether this node inhabits the DATA domain (a literal value) rather than the object
    /// domain.
    ///
    /// OWL 2 interprets an ontology over two domains, and `owl:Thing` denotes only the object
    /// one. A concrete node is therefore NOT seeded with the internalized TBox: every general
    /// concept inclusion is a statement about `Δ_I`, and placing `nnf(¬C ⊔ D)` on a literal's
    /// node would let a TBox axiom close a branch over an element the axiom does not
    /// quantify over — an inconsistency the ontology does not state.
    pub(crate) concrete: bool,
    /// The VALUE class this node denotes, when it denotes a literal whose value is known.
    ///
    /// The data domain admits no unique-name freedom: two literals denote one element exactly
    /// when they denote one value. Two nodes carrying different classes are therefore
    /// DISTINCT with nothing having said so, which is what lets a functional data property
    /// clash on two disagreeing values; and two nodes carrying the same class can never be
    /// counted as two, which is what stops `"1"^^xsd:integer` and `"01"^^xsd:integer` from
    /// satisfying a `≥2` restriction between them.
    pub(crate) value_class: Option<u32>,
}

/// The STABLE identity of a nominal — either a named individual or a generated reserved one.
///
/// Not every structurally-unblocked (`root`) node is a nominal: an anonymous `fresh_types`
/// witness is a root with no nominal identity, and a blockable node merged into a root does not
/// become one. A `NominalId` is carried only by nodes that genuinely DENOTE a nominal, and it is
/// stable across merges and state clones — it is never a node index, which a merge would
/// invalidate. It doubles as the `u` of Motik–Shearer–Horrocks' reserved root `u.⟨R,B,i⟩`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum NominalId {
    /// A named individual, by its term id.
    Named(u32),
    /// A generated reserved nominal, by its own reserved tag (MSH's `u.⟨R,B,i⟩`, nested).
    Generated(Box<GeneratedRoot>),
}

/// A TYPED, collision-free identity for a generated (nominal-introduction) reserved root.
///
/// It is the Motik–Shearer–Horrocks reserved root `u.⟨R,B,i⟩`: the AT-MOST ROOT `u` this
/// reserved set belongs to (a stable [`NominalId`], NOT a mutable node index), the counted role
/// `R`, the at-most filler concept id `B`, and the index `i` within the bound. Because the key
/// carries `u`, two independent at-most roots with the same `⟨R,B,i⟩` get DISTINCT reserved
/// roots and can never alias; and because `u` is stable, a merge or clone never invalidates it.
/// The concept-tree `NN`-rule and the hypertableau `NI`-rule share this identity so the two
/// cores mint the same nominal for the same trigger.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct GeneratedRoot {
    /// The at-most root `u` whose reserved set this belongs to.
    pub(crate) origin: NominalId,
    /// The counted role `R`.
    pub(crate) role: Role,
    /// The at-most filler concept id `B`.
    pub(crate) filler: u32,
    /// The index `i` within the bound `1..=n`.
    pub(crate) index: u32,
}

/// Elements per [`PVec`] leaf: a write copies at most one leaf of them.
const LEAF_BITS: u32 = 8;
/// See [`LEAF_BITS`].
const LEAF: usize = 1 << LEAF_BITS;
/// Children per [`PVec`] branch: a write copies at most one branch of them per level.
const BRANCH_BITS: u32 = 6;
/// See [`BRANCH_BITS`].
const BRANCH: usize = 1 << BRANCH_BITS;

/// One subtree of a [`PVec`]: a leaf of elements, or a branch of subtrees, each behind an
/// [`Rc`](std::rc::Rc) so that a clone shares it.
enum Tree<T> {
    /// Up to [`LEAF`] elements.
    Leaf(std::rc::Rc<Vec<T>>),
    /// Up to [`BRANCH`] subtrees one level lower.
    Branch(std::rc::Rc<Vec<Self>>),
}

impl<T> Clone for Tree<T> {
    fn clone(&self) -> Self {
        match self {
            Self::Leaf(leaf) => Self::Leaf(std::rc::Rc::clone(leaf)),
            Self::Branch(branch) => Self::Branch(std::rc::Rc::clone(branch)),
        }
    }
}

/// A persistent vector: a radix tree of [`Rc`](std::rc::Rc)-shared leaves and branches,
/// copy-on-write.
///
/// A search keeps the state of every open level, and a chain of choices that never backtracks
/// opens one level per choice — as many levels as the knowledge base has choices to make. A
/// level that deep-cloned the graph's vectors cost the whole graph, so memory grew with depth
/// times graph. Here a clone copies ONE pointer whatever the length, and a write copies the
/// path to its element — a leaf and one branch per level, three levels deep at a million
/// elements — so a level costs what it changed, and a choice's clone is flat in the size of
/// the graph. Values and their order are those of a plain vector.
pub(crate) struct PVec<T> {
    /// The tree, absent while empty.
    root: Option<Tree<T>>,
    /// How many branch levels sit above the leaves.
    height: u32,
    /// The element count.
    len: usize,
}

impl<T> Clone for PVec<T> {
    fn clone(&self) -> Self {
        Self {
            root: self.root.clone(),
            height: self.height,
            len: self.len,
        }
    }
}

impl<T> Default for PVec<T> {
    fn default() -> Self {
        Self {
            root: None,
            height: 0,
            len: 0,
        }
    }
}

/// The bit a branch at `height` (one or more) reads its child index from.
const fn shift(height: u32) -> u32 {
    LEAF_BITS + BRANCH_BITS * (height - 1)
}

impl<T: Clone> PVec<T> {
    pub(crate) const fn len(&self) -> usize {
        self.len
    }

    /// How many elements a tree of `height` branch levels holds.
    const fn capacity(height: u32) -> usize {
        LEAF << (BRANCH_BITS * height)
    }

    pub(crate) fn get(&self, index: usize) -> Option<&T> {
        if index >= self.len {
            return None;
        }
        let mut node = self.root.as_ref()?;
        let mut height = self.height;
        loop {
            match node {
                Tree::Leaf(leaf) => return leaf.get(index & (LEAF - 1)),
                Tree::Branch(branch) => {
                    node = &branch[(index >> shift(height)) & (BRANCH - 1)];
                    height -= 1;
                }
            }
        }
    }

    /// The element at `index`, the path to it copied wherever another vector shares it.
    fn get_mut(node: &mut Tree<T>, height: u32, index: usize) -> &mut T {
        match node {
            Tree::Leaf(leaf) => &mut std::rc::Rc::make_mut(leaf)[index & (LEAF - 1)],
            Tree::Branch(branch) => {
                let child =
                    &mut std::rc::Rc::make_mut(branch)[(index >> shift(height)) & (BRANCH - 1)];
                Self::get_mut(child, height - 1, index)
            }
        }
    }

    /// An empty subtree of `height` branch levels.
    fn empty(height: u32) -> Tree<T> {
        if height == 0 {
            Tree::Leaf(std::rc::Rc::new(Vec::with_capacity(LEAF)))
        } else {
            Tree::Branch(std::rc::Rc::new(Vec::with_capacity(BRANCH)))
        }
    }

    /// Append `value` as element `index` under `node`, a subtree of `height` levels.
    fn append(node: &mut Tree<T>, height: u32, index: usize, value: T) {
        match node {
            Tree::Leaf(leaf) => std::rc::Rc::make_mut(leaf).push(value),
            Tree::Branch(branch) => {
                let branch = std::rc::Rc::make_mut(branch);
                let slot = (index >> shift(height)) & (BRANCH - 1);
                if slot == branch.len() {
                    branch.push(Self::empty(height - 1));
                }
                Self::append(&mut branch[slot], height - 1, index, value);
            }
        }
    }

    pub(crate) fn push(&mut self, value: T) {
        if self.root.is_none() {
            self.root = Some(Self::empty(0));
        }
        if self.len == Self::capacity(self.height) {
            let grown = self.root.take().expect("a nonempty tree has a root");
            self.root = Some(Tree::Branch(std::rc::Rc::new(vec![grown])));
            self.height += 1;
        }
        let root = self.root.as_mut().expect("the tree has a root");
        Self::append(root, self.height, self.len, value);
        self.len += 1;
    }

    /// How many pointers a clone copies: one, whatever the length.
    pub(crate) const fn chunks(&self) -> usize {
        1
    }

    /// Every element, in index order.
    pub(crate) fn iter(&self) -> impl Iterator<Item = &T> {
        (0..self.len).map(move |index| &self[index])
    }

    pub(crate) fn resize_with(&mut self, len: usize, mut fill: impl FnMut() -> T) {
        while self.len < len {
            self.push(fill());
        }
    }
}

impl<T: Clone> std::ops::Index<usize> for PVec<T> {
    type Output = T;

    fn index(&self, index: usize) -> &T {
        self.get(index)
            .unwrap_or_else(|| panic!("PVec index {index} out of bounds of {}", self.len))
    }
}

impl<T: Clone> std::ops::IndexMut<usize> for PVec<T> {
    fn index_mut(&mut self, index: usize) -> &mut T {
        assert!(
            index < self.len,
            "PVec index {index} out of bounds of {}",
            self.len
        );
        let height = self.height;
        let root = self.root.as_mut().expect("a nonempty vector has a root");
        Self::get_mut(root, height, index)
    }
}

/// One slot's chain of fingerprints, each with its ascending node list.
type Chain = std::rc::Rc<Vec<(u64, Vec<usize>)>>;

/// A persistent multimap from blocking-signature fingerprints to the nodes holding them, each
/// list ascending: hashed into chained slots that live in a [`PVec`], so a clone copies one
/// pointer per chunk of slots and a write copies one slot's chain.
#[derive(Clone)]
pub(crate) struct Buckets {
    /// The chains, a power of two of them.
    slots: PVec<Chain>,
    /// How many fingerprints are held.
    len: usize,
}

impl Default for Buckets {
    fn default() -> Self {
        let mut slots = PVec::default();
        slots.resize_with(64, std::rc::Rc::default);
        Self { slots, len: 0 }
    }
}

impl Buckets {
    /// The slot fingerprint `key` hashes to.
    fn slot(&self, key: u64) -> usize {
        // A fingerprint's low bits are FNV's and mix well; folding the high half in keeps a
        // fingerprint that differs only above the mask from sharing a chain.
        let mixed = key ^ (key >> 32);
        usize::try_from(mixed & (self.slots.len() as u64 - 1)).expect("below the slot count")
    }

    /// The nodes holding `key`, ascending.
    pub(crate) fn members(&self, key: u64) -> &[usize] {
        self.slots[self.slot(key)]
            .iter()
            .find(|(held, _)| *held == key)
            .map_or(&[], |(_, members)| members.as_slice())
    }

    /// Add `node` under `key`.
    pub(crate) fn insert(&mut self, key: u64, node: usize) {
        if self.len >= 2 * self.slots.len() {
            self.grow();
        }
        let slot = self.slot(key);
        let chain = std::rc::Rc::make_mut(&mut self.slots[slot]);
        match chain.iter_mut().find(|(held, _)| *held == key) {
            Some((_, members)) => {
                if let Err(at) = members.binary_search(&node) {
                    members.insert(at, node);
                }
            }
            None => {
                chain.push((key, vec![node]));
                self.len += 1;
            }
        }
    }

    /// Remove `node` from under `key`.
    pub(crate) fn remove(&mut self, key: u64, node: usize) {
        let slot = self.slot(key);
        if !self.slots[slot].iter().any(|(held, _)| *held == key) {
            return;
        }
        let chain = std::rc::Rc::make_mut(&mut self.slots[slot]);
        if let Some(at) = chain.iter().position(|(held, _)| *held == key) {
            let members = &mut chain[at].1;
            if let Ok(index) = members.binary_search(&node) {
                members.remove(index);
            }
            if members.is_empty() {
                chain.swap_remove(at);
                self.len -= 1;
            }
        }
    }

    /// Double the slots and rehash: amortized against the insertions that filled them.
    fn grow(&mut self) {
        let mut entries: Vec<(u64, Vec<usize>)> = Vec::with_capacity(self.len);
        for chain in self.slots.iter() {
            entries.extend(chain.iter().cloned());
        }
        let mut slots = PVec::default();
        slots.resize_with(self.slots.len() * 2, std::rc::Rc::default);
        self.slots = slots;
        for (key, members) in entries {
            let slot = self.slot(key);
            std::rc::Rc::make_mut(&mut self.slots[slot]).push((key, members));
        }
    }
}

/// A persistent set of node indices: a bit per node in a [`PVec`] of words, with a summary bit
/// per word, so finding the next member past a node skips sixty-four empty words at a time and
/// a clone copies one pointer per chunk.
#[derive(Clone, Default)]
pub(crate) struct NodeSet {
    /// Bit `x % 64` of word `x / 64` is node `x`.
    words: PVec<u64>,
    /// Bit `w % 64` of summary word `w / 64` says word `w` is nonzero.
    summary: PVec<u64>,
}

impl NodeSet {
    /// Add `x`.
    pub(crate) fn insert(&mut self, x: usize) {
        let word = x / 64;
        if self.words.len() <= word {
            self.words.resize_with(word + 1, || 0);
            self.summary.resize_with(word / 64 + 1, || 0);
        }
        if self.words[word] & (1 << (x % 64)) == 0 {
            self.words[word] |= 1 << (x % 64);
            self.summary[word / 64] |= 1 << (word % 64);
        }
    }

    /// Remove `x`.
    pub(crate) fn remove(&mut self, x: usize) {
        let word = x / 64;
        if self
            .words
            .get(word)
            .is_none_or(|&bits| bits & (1 << (x % 64)) == 0)
        {
            return;
        }
        self.words[word] &= !(1 << (x % 64));
        if self.words[word] == 0 {
            self.summary[word / 64] &= !(1 << (word % 64));
        }
    }

    /// The smallest member at or past `x`.
    pub(crate) fn first_from(&self, x: usize) -> Option<usize> {
        let mut word = x / 64;
        if let Some(&bits) = self.words.get(word) {
            let here = bits & (u64::MAX << (x % 64));
            if here != 0 {
                return Some(word * 64 + here.trailing_zeros() as usize);
            }
        }
        word += 1;
        let mut group = word / 64;
        let mut mask = if word.is_multiple_of(64) {
            u64::MAX
        } else {
            u64::MAX << (word % 64)
        };
        while let Some(&summary) = self.summary.get(group) {
            let nonzero = summary & mask;
            if nonzero != 0 {
                let word = group * 64 + nonzero.trailing_zeros() as usize;
                return Some(word * 64 + self.words[word].trailing_zeros() as usize);
            }
            group += 1;
            mask = u64::MAX;
        }
        None
    }
}

/// Which nodes are blocked, kept current from round to round instead of recomputed.
///
/// Blocking is a pure function of the graph: one pass in ascending index order, a node
/// directly blocked by the first earlier unblocked candidate with its signature, indirectly by
/// a blocked predecessor (see the hypertableau's module docs). Every input of a node's status
/// is its own signature, its predecessor's status and the earlier candidates sharing its
/// signature, so a round recomputes only the nodes whose signature was written, their
/// children, and — as statuses flip — the later nodes those flips can reach. Persistent, so a
/// branch copies only what it changes.
#[derive(Clone, Default)]
pub(crate) struct Blocking {
    /// Per node: its signature's fingerprint while it is a candidate — an unmerged, non-root
    /// tree node with a predecessor.
    pub(crate) key: PVec<Option<u64>>,
    /// Per node: whether it is blocked.
    pub(crate) blocked: PVec<bool>,
    /// Per node: the candidate directly blocking it, if one does.
    pub(crate) blocker: PVec<Option<usize>>,
    /// Candidates by fingerprint.
    pub(crate) buckets: Buckets,
}

impl Blocking {
    /// Whether node `x` is blocked; a node the index has not reached yet is not.
    pub(crate) fn is_blocked(&self, x: usize) -> bool {
        self.blocked.get(x).copied().unwrap_or(false)
    }

    /// Grow the per-node vectors to `nodes`.
    pub(crate) fn reserve(&mut self, nodes: usize) {
        self.key.resize_with(nodes, || None);
        self.blocked.resize_with(nodes, || false);
        self.blocker.resize_with(nodes, || None);
    }
}

/// A completion graph's nodes, SHARED between the states a search clones, with the log of
/// which ones were written since the log was last taken.
///
/// Every alternative of a case split starts from a clone of the state it splits, and the
/// search keeps the state of every open level. A deep clone of a large ABox's nodes per level
/// exhausted memory within seconds; here a clone copies one pointer per chunk of nodes, and a
/// node is copied only when a branch writes to it ([`Rc::make_mut`] through [`IndexMut`]).
///
/// The same write path is what delta saturation reads. Every mutable access to a node — a
/// concept entering its label, an inequality, a merge folding into it or forwarding it, a
/// value class — goes through [`IndexMut`], and a new node through [`NodeVec::push`], so the
/// log [`NodeVec::take_touched`] hands a round is a superset of the nodes whose own reading
/// changed: nothing is compared against a snapshot, and a round costs what changed rather
/// than a pass over the graph to find out. A write that changes nothing (a concept already
/// present) is logged too, which can only widen what the next round re-matches.
///
/// [`Rc::make_mut`]: std::rc::Rc::make_mut
/// [`IndexMut`]: std::ops::IndexMut
#[derive(Clone, Default)]
pub(crate) struct NodeVec {
    /// The nodes.
    nodes: PVec<std::rc::Rc<Node>>,
    /// Node indices written since [`Self::take_touched`] last ran, in write order.
    touched: Vec<usize>,
}

impl NodeVec {
    pub(crate) fn len(&self) -> usize {
        self.nodes.len()
    }

    pub(crate) fn push(&mut self, node: Node) {
        self.touched.push(self.nodes.len());
        self.nodes.push(std::rc::Rc::new(node));
    }

    /// The nodes written since the last call, in write order and possibly repeated, leaving
    /// the log empty.
    pub(crate) fn take_touched(&mut self) -> Vec<usize> {
        std::mem::take(&mut self.touched)
    }

    #[cfg(test)]
    pub(crate) fn iter(&self) -> impl Iterator<Item = &Node> {
        (0..self.len()).map(|index| &self[index])
    }
}

impl std::ops::Index<usize> for NodeVec {
    type Output = Node;

    fn index(&self, index: usize) -> &Node {
        &self.nodes[index]
    }
}

impl std::ops::IndexMut<usize> for NodeVec {
    fn index_mut(&mut self, index: usize) -> &mut Node {
        self.touched.push(index);
        std::rc::Rc::make_mut(&mut self.nodes[index])
    }
}

impl State {
    /// What cloning this state copies: one pointer per persistent vector it holds, plus the
    /// write log — which a saturated level has emptied. Every structure here is persistent, so
    /// this is what a branch's alternative pays to start from its level, and it does not grow
    /// with the graph.
    pub(crate) fn clone_cost(&self) -> u64 {
        let closures = self.closures.borrow();
        let cached: usize = closures.reach.iter().map(PVec::chunks).sum::<usize>()
            + closures.readers.iter().map(PVec::chunks).sum::<usize>();
        let blocking = &self.blocking;
        (self.nodes.nodes.chunks()
            + self.nodes.touched.len()
            + self.deferred.len()
            + self.edges.chunks()
            + self.adjacency.chunks()
            + cached
            + blocking.key.chunks()
            + blocking.blocked.chunks()
            + blocking.blocker.chunks()
            + blocking.buckets.slots.chunks()
            + self.children.chunks()
            + self.merged_in.chunks()
            + self.open.words.chunks()
            + self.open.summary.chunks()
            + 1) as u64
    }

    /// Append the edge `from → to` over `property`, indexing it under both endpoints' roots.
    pub(crate) fn push_edge(&mut self, from: usize, to: usize, property: u32) {
        let edge = self.edges.len();
        self.edges.push((from, to, property));
        let from = find(self, from);
        let to = find(self, to);
        for node in [from, to] {
            if self.adjacency.len() <= node {
                self.adjacency.resize_with(node + 1, std::rc::Rc::default);
            }
        }
        std::rc::Rc::make_mut(&mut self.adjacency[from]).push(edge);
        if to != from {
            std::rc::Rc::make_mut(&mut self.adjacency[to]).push(edge);
        }
    }

    /// Forward the root `discard` to the root `keep` — the one place a node stops being a
    /// root — and fold its indexed edges into the keeper's, so [`State::class_edges`] of the
    /// surviving root still lists exactly the edges the full scan would keep for it. Its
    /// children and its union-find class fold into the keeper's the same way, and the cached
    /// transitive closures are dropped, since identity moved under all of them.
    fn forward(&mut self, keep: usize, discard: usize) {
        self.nodes[discard].merged = Some(keep);
        self.merge_adjacency(keep, discard);
        self.merge_children(keep, discard);
        self.merge_class(keep, discard);
        // Identity moved under every cached closure at once.
        let edges = self.edges.len();
        self.closures.get_mut().clear(edges);
    }

    /// Fold `discard`'s indexed edges into `keep`'s, keeping the list ascending and unique.
    fn merge_adjacency(&mut self, keep: usize, discard: usize) {
        if discard >= self.adjacency.len() || self.adjacency[discard].is_empty() {
            return;
        }
        let folded = std::mem::take(&mut self.adjacency[discard]);
        if self.adjacency.len() <= keep {
            self.adjacency.resize_with(keep + 1, std::rc::Rc::default);
        }
        let kept = std::mem::take(&mut self.adjacency[keep]);
        let (kept, folded) = (
            std::rc::Rc::unwrap_or_clone(kept),
            std::rc::Rc::unwrap_or_clone(folded),
        );
        let mut merged = Vec::with_capacity(kept.len() + folded.len());
        let (mut left, mut right) = (kept.into_iter().peekable(), folded.into_iter().peekable());
        loop {
            let next = match (left.peek(), right.peek()) {
                (Some(&l), Some(&r)) if l < r => left.next(),
                (Some(&l), Some(&r)) if r < l => right.next(),
                (Some(_), Some(_)) => {
                    right.next();
                    left.next()
                }
                (Some(_), None) => left.next(),
                (None, Some(_)) => right.next(),
                (None, None) => break,
            };
            merged.extend(next);
        }
        self.adjacency[keep] = std::rc::Rc::new(merged);
    }

    /// Fold `discard`'s children into `keep`'s, keeping the list ascending.
    fn merge_children(&mut self, keep: usize, discard: usize) {
        if discard >= self.children.len() || self.children[discard].is_empty() {
            return;
        }
        let folded = std::mem::take(&mut self.children[discard]);
        if self.children.len() <= keep {
            self.children.resize_with(keep + 1, std::rc::Rc::default);
        }
        let kept = std::rc::Rc::make_mut(&mut self.children[keep]);
        kept.extend_from_slice(&folded);
        kept.sort_unstable();
        kept.dedup();
    }

    /// Fold `discard`'s class into `keep`'s: `discard` and everything merged into it.
    fn merge_class(&mut self, keep: usize, discard: usize) {
        let mut folded: Vec<usize> = vec![discard];
        if discard < self.merged_in.len() {
            folded.extend_from_slice(&std::mem::take(&mut self.merged_in[discard]));
        }
        if self.merged_in.len() <= keep {
            self.merged_in.resize_with(keep + 1, std::rc::Rc::default);
        }
        let kept = std::rc::Rc::make_mut(&mut self.merged_in[keep]);
        kept.extend_from_slice(&folded);
        kept.sort_unstable();
    }

    /// The nodes resolving to the root `x` — `x` and every node merged into it — ascending.
    pub(crate) fn class_of(&self, x: usize) -> impl Iterator<Item = usize> + '_ {
        let merged = self
            .merged_in
            .get(x)
            .map_or(&[][..], |merged| merged.as_slice());
        let at = merged.partition_point(|&n| n < x);
        merged[..at]
            .iter()
            .copied()
            .chain(std::iter::once(x))
            .chain(merged[at..].iter().copied())
    }

    /// The nodes whose predecessor resolves to the root `x`, ascending.
    pub(crate) fn children_of(&self, x: usize) -> &[usize] {
        self.children
            .get(x)
            .map_or(&[], |children| children.as_slice())
    }

    /// The indices of every edge with an endpoint resolving to the root `x`, ascending.
    pub(crate) fn class_edges(&self, x: usize) -> &[usize] {
        self.adjacency.get(x).map_or(&[], |edges| edges.as_slice())
    }
}

/// The `(property, forward?)` edge patterns that realize a role, sorted ascending. Shared
/// rather than cloned: a neighbourhood read is the most-called scan in either calculus, and a
/// cached closure is read far more often than it is built.
pub(crate) type Achievers = std::rc::Rc<[(u32, bool)]>;

/// Closed role closures, by role.
///
/// A neighbourhood read asks for its role's closure every time, so the lookup is on the
/// hottest path either calculus has: a hash probe with the workspace's fixed-key table hasher
/// is one where an ordered map paid a tree search of `Role` comparisons per read. Keyed by the
/// role itself rather than by a slot derived from its term id, so its size is the number of
/// roles read, not the largest property id the interner handed out — and no id is too large to
/// address on a 32-bit target.
#[derive(Default)]
pub(crate) struct AchieverCache {
    closures: hashbrown::HashMap<Role, Achievers, purrdf_core::FastHasher>,
}

impl AchieverCache {
    pub(crate) fn get(&self, role: Role) -> Option<&Achievers> {
        self.closures.get(&role)
    }

    pub(crate) fn insert(&mut self, role: Role, closure: Achievers) {
        self.closures.insert(role, closure);
    }

    #[cfg(test)]
    pub(crate) fn contains_key(&self, role: Role) -> bool {
        self.closures.contains_key(&role)
    }

    #[cfg(test)]
    pub(crate) fn is_empty(&self) -> bool {
        self.closures.is_empty()
    }
}

/// The most closure members a state caches, summed over every root and pattern.
///
/// A cached closure is quadratic in the worst case — every node of a long transitive chain
/// reads every node after it — so past this ceiling a read walks its closure as it always
/// did instead of caching another one. A resource bound, not a semantic one: a read answers
/// the same either way.
const MAX_CACHED_MEMBERS: usize = 1 << 22;

/// One root's closure over one transitive pattern, cached: every node the pattern's edges
/// reach from it, in the order a walk reached them and then in the order edges extended it.
#[derive(Clone, Default)]
pub(crate) struct Reach {
    /// The members, in reach order.
    order: Vec<usize>,
    /// The same members, ascending, for membership.
    sorted: Vec<usize>,
    /// `order[fresh..]` is what the round numbered [`Self::round`] sees for the first time.
    fresh: usize,
    /// The round the members from [`Self::fresh`] on are new to.
    round: u64,
}

impl Reach {
    /// The members, in reach order.
    pub(crate) fn order(&self) -> &[usize] {
        &self.order
    }

    /// The members round `round` is the first to see: what was appended for it, or nothing.
    pub(crate) fn fresh_in(&self, round: u64) -> &[usize] {
        if self.round == round {
            &self.order[self.fresh..]
        } else {
            &[]
        }
    }

    /// Whether `y` is a member.
    pub(crate) fn contains(&self, y: usize) -> bool {
        self.sorted.binary_search(&y).is_ok()
    }
}

/// The transitive closures a state has read, cached per root and pattern, and kept current as
/// edges are appended.
///
/// A transitive read walks its closure one edge step at a time, every step billed the edges it
/// reads ([`Graph::charge_step`]), so a long chain read from each of its nodes, round after
/// round, was the dearest work the calculus did. A cached closure is read for the
/// length of its member list instead, and it is MAINTAINED rather than recomputed: an edge
/// `src → dst` realizing a pattern extends exactly the closures that contain `src` (found
/// through [`Self::readers`]) and `src`'s own, by `dst` and `dst`'s closure. A merge changes
/// node identity under every closure at once, so it clears the cache instead.
///
/// Persistent like the rest of the state: a clone shares every entry, and a branch copies an
/// entry only when it extends it.
#[derive(Clone, Default)]
pub(crate) struct Closures {
    /// Per pattern index: root → its cached closure.
    reach: Vec<PVec<Option<std::rc::Rc<Reach>>>>,
    /// Per pattern index: node → the roots whose cached closure has it as a member.
    readers: Vec<PVec<std::rc::Rc<Vec<usize>>>>,
    /// How many of the state's edges the cache is current with.
    integrated: usize,
    /// Members cached, summed — held to [`MAX_CACHED_MEMBERS`].
    cached: usize,
}

impl Closures {
    /// The cached closure of `x` over pattern `index`, if there is one.
    fn get(&self, index: usize, x: usize) -> Option<&std::rc::Rc<Reach>> {
        self.reach.get(index)?.get(x)?.as_ref()
    }

    /// The roots whose cached closure over pattern `index` has `y` as a member.
    fn readers_of(&self, index: usize, y: usize) -> &[usize] {
        self.readers
            .get(index)
            .and_then(|readers| readers.get(y))
            .map_or(&[], |readers| readers.as_slice())
    }

    /// Note that `reader`'s closure over pattern `index` now has `y` as a member.
    fn add_reader(&mut self, index: usize, y: usize, reader: usize) {
        let readers = &mut self.readers[index];
        if readers.len() <= y {
            readers.resize_with(y + 1, std::rc::Rc::default);
        }
        std::rc::Rc::make_mut(&mut readers[y]).push(reader);
    }

    /// Cache `members` as the closure of `x` over pattern `index`, new to round `round`.
    fn store(&mut self, patterns: usize, index: usize, x: usize, members: &[usize], round: u64) {
        if self.reach.len() < patterns {
            self.reach.resize_with(patterns, PVec::default);
            self.readers.resize_with(patterns, PVec::default);
        }
        let mut sorted = members.to_vec();
        sorted.sort_unstable();
        for &y in members {
            self.add_reader(index, y, x);
        }
        let reach = &mut self.reach[index];
        if reach.len() <= x {
            reach.resize_with(x + 1, || None);
        }
        self.cached += members.len();
        reach[x] = Some(std::rc::Rc::new(Reach {
            order: members.to_vec(),
            sorted,
            fresh: 0,
            round,
        }));
    }

    /// Extend the cached closure of `u` over pattern `index` by whichever of `gain` it lacks,
    /// as members new to round `round`. Returns how many were added.
    fn extend(&mut self, index: usize, u: usize, gain: &[usize], round: u64) -> usize {
        let Some(Some(present)) = self.reach.get(index).and_then(|reach| reach.get(u)) else {
            return 0;
        };
        let mut seen = BTreeSet::new();
        let fresh: Vec<usize> = gain
            .iter()
            .copied()
            .filter(|&y| !present.contains(y) && seen.insert(y))
            .collect();
        if fresh.is_empty() {
            return 0;
        }
        let slot = self.reach[index][u]
            .as_mut()
            .expect("the entry was read above");
        let entry = std::rc::Rc::make_mut(slot);
        if entry.round != round {
            entry.round = round;
            entry.fresh = entry.order.len();
        }
        entry.order.extend_from_slice(&fresh);
        entry.sorted.extend_from_slice(&fresh);
        entry.sorted.sort_unstable();
        for &y in &fresh {
            self.add_reader(index, y, u);
        }
        self.cached += fresh.len();
        fresh.len()
    }

    /// Forget every cached closure: node identity changed under all of them.
    fn clear(&mut self, edges: usize) {
        self.reach.clear();
        self.readers.clear();
        self.cached = 0;
        self.integrated = edges;
    }
}

/// The membership scratch every neighbourhood read reuses: two generation-stamped vectors
/// indexed by node, and the closure walk's frontier.
///
/// A stamp vector answers "already emitted?" and "already reached?" in one index, and it is
/// reset by bumping a counter rather than by clearing it, so the per-read sets and vectors a
/// neighbourhood read used to build are gone: once the stamps have grown to the graph, a read
/// allocates nothing.
#[derive(Default)]
pub(crate) struct ReadScratch {
    /// `seen[y] == epoch` exactly when `y` was emitted by the current read.
    seen: Vec<u32>,
    /// `visited[y] == walk` exactly when `y` was reached by the current closure walk.
    visited: Vec<u32>,
    /// The current read's stamp.
    epoch: u32,
    /// The current closure walk's stamp.
    walk: u32,
    /// The closure walk's depth-first frontier.
    frontier: Vec<usize>,
    /// The closure walk's members, in the order it reached them, for the cache.
    members: Vec<usize>,
}

impl ReadScratch {
    /// Start a read over a graph of `nodes` nodes.
    fn begin(&mut self, nodes: usize) {
        if self.seen.len() < nodes {
            self.seen.resize(nodes, 0);
            self.visited.resize(nodes, 0);
        }
        self.epoch = self.epoch.wrapping_add(1);
        if self.epoch == 0 {
            // Four billion reads in: every stale stamp could now collide, so clear once.
            self.seen.fill(0);
            self.epoch = 1;
        }
        self.frontier.clear();
    }

    /// Start one transitive closure walk inside the current read.
    fn begin_walk(&mut self) {
        self.walk = self.walk.wrapping_add(1);
        if self.walk == 0 {
            self.visited.fill(0);
            self.walk = 1;
        }
        self.frontier.clear();
    }

    /// Mark `y` emitted; whether it was not already.
    fn emit(&mut self, y: usize) -> bool {
        let fresh = self.seen[y] != self.epoch;
        self.seen[y] = self.epoch;
        fresh
    }

    /// Mark `y` reached by the current walk; whether it was not already.
    fn reach(&mut self, y: usize) -> bool {
        let fresh = self.visited[y] != self.walk;
        self.visited[y] = self.walk;
        fresh
    }
}

/// A node buffer borrowed from a [`Graph`]'s read pool, cleared and returned when dropped.
pub(crate) struct Buffer<'g> {
    /// The nodes.
    items: Vec<usize>,
    /// The pool it goes back to.
    pool: &'g RefCell<Vec<Vec<usize>>>,
}

impl std::ops::Deref for Buffer<'_> {
    type Target = Vec<usize>;

    fn deref(&self) -> &Vec<usize> {
        &self.items
    }
}

impl std::ops::DerefMut for Buffer<'_> {
    fn deref_mut(&mut self) -> &mut Vec<usize> {
        &mut self.items
    }
}

impl Drop for Buffer<'_> {
    fn drop(&mut self) {
        let mut items = std::mem::take(&mut self.items);
        items.clear();
        self.pool.borrow_mut().push(items);
    }
}

/// The role an achiever pattern `(property, forward?)` names: `property` itself read forward,
/// its inverse read backward.
pub(crate) const fn pattern_role(property: u32, forward: bool) -> Role {
    if forward {
        Role::Named(property)
    } else {
        Role::Inv(property)
    }
}

/// Every endpoint one step from `y` over the edge patterns `ach` reaches, in edge order and
/// possibly repeated: the forward endpoint of an edge leaving `y`'s class over a forward
/// pattern, the backward endpoint of one entering it over a backward pattern. Charges nothing;
/// the callers charge for the step.
pub(crate) fn for_each_step(
    st: &State,
    y: usize,
    ach: &[(u32, bool)],
    mut visit: impl FnMut(usize),
) {
    let y = find(st, y);
    for &edge in st.class_edges(y) {
        let (from, to, prop) = st.edges[edge];
        let f = find(st, from);
        let t = find(st, to);
        if realizes(ach, (prop, true)) && f == y {
            visit(t);
        }
        if realizes(ach, (prop, false)) && t == y {
            visit(f);
        }
    }
}

/// Whether `pattern` realizes the role `achievers` was closed for.
fn realizes(achievers: &[(u32, bool)], pattern: (u32, bool)) -> bool {
    achievers.binary_search(&pattern).is_ok()
}

/// A completion graph under construction.
#[derive(Clone)]
pub(crate) struct State {
    /// Observational transcript only for prepared-clash proof production.
    pub(crate) support: super::support::Cursor,
    /// All nodes ever created (merged-away ones remain, forwarded via `merged`).
    pub(crate) nodes: NodeVec,
    /// Directed role edges `(from, to, property)`; endpoints resolved via [`find`]. Only
    /// [`State::push_edge`] appends here, so [`State::adjacency`] indexes every edge.
    pub(crate) edges: PVec<(usize, usize, u32)>,
    /// Union-find root → the indices into [`State::edges`] of every edge with an endpoint
    /// resolving to it, in ascending order.
    ///
    /// A neighbourhood read ([`Graph::neighbors`]) needs the edges touching ONE node's class,
    /// and reading them off the whole edge vector made every round cost the node count times
    /// the edge count. An edge is indexed under its endpoints' roots when it is pushed, and a
    /// merge folds the discarded root's list into the keeper's, so a root's list holds exactly
    /// the edges the full scan would have kept for it. Walking it in ascending order visits
    /// them in the order the scan did, which keeps every neighbourhood — and so every search,
    /// verdict and proof — identical.
    pub(crate) adjacency: PVec<std::rc::Rc<Vec<usize>>>,
    /// How many of [`State::edges`] the last saturation round had already seen when it began.
    ///
    /// Edges are only ever appended within a branch, so the ones a round has not matched
    /// against yet are exactly the tail past this mark — a change log that costs nothing to
    /// keep and is inherited by every clone.
    pub(crate) edges_seen: usize,
    /// Tree nodes still to be identified with a nominal whose at-least head — for a witness that
    /// would itself await an identification — was matched but held back from minting until
    /// hyperresolution reached a fixpoint, and, while the identification is a choice, until
    /// that choice is made. Empty in every completion.
    pub(crate) deferred: Vec<usize>,
    /// The transitive closures this state's reads have cached — see [`Closures`].
    pub(crate) closures: RefCell<Closures>,
    /// Which nodes are blocked, as the last saturation round left it — see [`Blocking`]. A
    /// node whose status flips is a change the next round re-matches around, exactly as a
    /// write to it is.
    pub(crate) blocking: Blocking,
    /// Root → the nodes whose predecessor resolves to it, ascending: whose blocking signature
    /// reads that root's label. A merge folds the discarded root's list into the keeper's.
    pub(crate) children: PVec<std::rc::Rc<Vec<usize>>>,
    /// Every root that may hold an open disjunction — a superset of the ones that do. A round
    /// adds every root it re-matches, since only a new body match opens one; the `⊔`-rule's
    /// scan removes each root it finds holding none. So the scan reads what changed since it
    /// last looked, not the graph.
    pub(crate) open: NodeSet,
    /// Root → the nodes merged into it, ascending — with the root itself, its union-find
    /// class. A merge folds the discarded root's class into the keeper's.
    pub(crate) merged_in: PVec<std::rc::Rc<Vec<usize>>>,
    /// Named individual term id → the node minted for it, resolved through [`find`] to the
    /// root that denotes it now.
    pub(crate) root_of: std::rc::Rc<BTreeMap<u32, usize>>,
    /// Generated (nominal-introduction) root identity → its root node index. Kept separate
    /// from [`State::root_of`] because the two identity spaces are disjoint by type — see
    /// [`GeneratedRoot`]. A merged-away entry is forwarded through [`find`] on lookup, exactly
    /// as [`Graph::root`] forwards [`State::root_of`].
    pub(crate) generated_root_of: std::rc::Rc<BTreeMap<GeneratedRoot, usize>>,
    /// A clash has been detected (e.g. a forced `≠` merge).
    pub(crate) clash: bool,
    /// A clique-work budget ran out mid-rule ([`max_clique`] returned `None`), so this
    /// state's counting answers are incomplete: the driver must surface the decision as
    /// EXHAUSTED, never as a verdict. A `Cell` because the read-only satisfaction check
    /// (`has_at_least`) can hit the budget through `&State`.
    pub(crate) clique_exhausted: std::cell::Cell<bool>,
}

/// What a decision is made *on top of* the knowledge base.
///
/// A refutation adds premises — the negated conclusion, and for a role axiom a pair of
/// fresh individuals joined by the antecedent role — so every entry here is an assumption
/// the caller injected, never something the ontology said. Gathering them into one struct
/// rather than passing four positional slices is what keeps a fifth kind of assumption
/// from being appended to a signature nobody can read.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Assumptions<'a> {
    /// Whether to pull in the ABox (individual roots, role edges, `owl:sameAs` merges).
    /// A pure subsumption check passes `false` and reasons over the TBox alone.
    pub(crate) include_abox: bool,
    /// Extra concept assertions `a : C`, as `(individual term id, concept id)`.
    pub(crate) types: &'a [(u32, u32)],
    /// Extra role assertions `a r b`, as `(subject, property, object)` term ids. The
    /// endpoints need not be knowledge-base individuals: a fresh one gets a root node of
    /// its own, which is exactly what a role-inclusion refutation needs.
    pub(crate) roles: &'a [(u32, u32, u32)],
    /// Concept ids placed on ONE fresh, anonymous, unnamed root — the witness a
    /// satisfiability or subsumption question asks about.
    pub(crate) fresh_types: &'a [u32],
}

impl Assumptions<'_> {
    /// The bare "is this knowledge base consistent?" question: the whole ABox, nothing
    /// added.
    pub(crate) const fn of_kb() -> Self {
        Self {
            include_abox: true,
            types: &[],
            roles: &[],
            fresh_types: &[],
        }
    }
}

/// The two caps one decision procedure run spends against.
///
/// One struct rather than two positional `u64`s for the reason [`Assumptions`] is one struct:
/// two same-typed budgets in a signature are two arguments a caller can silently swap, and
/// swapping them here would run a rounds-denominated search under a work cap thousands of
/// times its size and call the result decided.
///
/// The two bound DIFFERENT quantities and neither implies the other — see [`work_cap`] for
/// why a rounds cap cannot see inside a round, which is the whole reason there are two.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Budget {
    /// Derivation ROUNDS the search may consume, summed over every branch.
    pub(crate) steps: u64,
    /// WORK UNITS the search may consume — the counted matcher, scan and clone work that
    /// happens INSIDE those rounds.
    pub(crate) work: u64,
}

impl Budget {
    /// The knowledge base's own two caps, both pure functions of its size.
    pub(crate) fn for_kb(kb: &Kb) -> Self {
        Self {
            steps: step_cap(kb),
            work: work_cap(kb),
        }
    }
}

/// The deterministic WORK meter one decision procedure run charges against.
///
/// # Why a second budget exists at all
///
/// [`step_cap`] is denominated in derivation ROUNDS, and a round is not a unit of work: it
/// is a pass whose cost is the graph it runs over times the clause set it matches. One
/// individual co-typed with several equivalence-defined classes makes every round enumerate
/// clause matches, successor subsets, achiever closures and branch-state clones whose count
/// grows with the co-typing, so the search can spend hours a few percent into a rounds
/// budget — reporting a cap it is nowhere near while it grinds. That is not a cap being
/// generous; it is a cap that cannot see the quantity that grew. This meter is charged at
/// the sites where that work actually happens, so the class of ontology above stops at a
/// declared ceiling and reports `budget-exhausted` instead of running unbounded.
///
/// # Every charge is a pure function of the search
///
/// The units are integers counted off the state — edges scanned, body atoms joined, subsets
/// enumerated, nodes cloned — never a clock reading, never a float, never a hash iteration
/// order. So a run's work figure is byte-identical run to run and on `wasm32`, exactly as
/// its round count is, and the [`Decision`] carrying both stays comparable as one struct.
///
/// # Why it saturates at the cap
///
/// A charge that would cross the cap is clamped to it. Two reasons: the search stops there
/// anyway, so the excess measures nothing; and a reader of an exhausted certificate can then
/// see `work` and `work-budget` agree exactly, which is what says WHICH of the two budgets
/// ended the run.
///
/// A [`std::cell::Cell`] because the sites that do the work — a neighbour scan, a
/// satisfaction test — hold the graph through `&self` and the state through `&State`, and
/// threading `&mut` through them would turn a measurement into a refactor of every rule.
pub(crate) struct Work {
    storage_refusal: std::cell::Cell<Option<purrdf_lex::allocation::StorageError>>,
    /// Units charged so far, clamped at [`Self::cap`].
    spent: std::cell::Cell<u64>,
    /// The ceiling this meter stops at.
    cap: u64,
}

impl Work {
    /// A meter that stops at `cap`.
    const fn new(cap: u64) -> Self {
        Self {
            storage_refusal: std::cell::Cell::new(None),
            spent: std::cell::Cell::new(0),
            cap,
        }
    }

    /// Charge `units` of work, clamping at the cap.
    pub(crate) fn charge(&self, units: u64) {
        self.spent
            .set(self.spent.get().saturating_add(units).min(self.cap));
    }

    /// Work charged so far.
    pub(crate) fn spent(&self) -> u64 {
        self.spent.get()
    }

    /// Set the meter back to `spent` — for a test-only check that must charge nothing.
    #[cfg(test)]
    pub(crate) fn restore(&self, spent: u64) {
        self.spent.set(spent);
    }

    /// Whether the budget is gone.
    ///
    /// Every enumerator that could run long consults this and stops, so the latency between
    /// the cap being reached and the search reporting it is bounded by one charge rather than
    /// by whatever the enumeration would have cost.
    pub(crate) fn exhausted(&self) -> bool {
        self.spent.get() >= self.cap || self.storage_refusal.get().is_some()
    }
}

/// What one decision procedure run decided, and what it consumed deciding it.
///
/// `consistent` is meaningful only when NEITHER `exhausted` NOR `stopped` is true: a run
/// that stopped at its cap, or that stopped because the caller asked it to, has closed
/// some branches and not others, and reporting the "no branch succeeded *yet*" state as
/// `false` would turn a resource limit — or a cancellation — into an entailment. Every
/// consumer in this crate reads BOTH flags before `consistent`.
///
/// Equality is over the WHOLE struct, and it is there so determinism can be asserted as one
/// comparison rather than as a list of field comparisons that a fourth field would silently
/// escape: two runs over one knowledge base must produce the same verdict, the same round
/// count, the same work figure, the same two stop flags and the same three shape counters.
///
/// # The three shape counters
///
/// `steps` and `work` are the two numbers the two caps are denominated in, and together they
/// say how much a search cost. Neither says WHY. Three searches costing a thousand rounds
/// each — one that built a thousand-node graph without branching once, one that branched a
/// thousand times over a two-node graph, and one that went a thousand levels deep down a
/// single spine — are three different situations with three different fixes, and a total
/// cannot tell them apart. [`Self::peak_nodes`], [`Self::disjunctions`] and [`Self::peak_depth`] are
/// the three quantities that do, and they are measured rather than derived: each is
/// observed at the one point in the search where the thing it names changes.
///
/// All three are counts over a deterministic search, so they are byte-identical run to run
/// and on `wasm32` for exactly the reason `steps` is — nothing here reads a clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Decision {
    /// Original physical refusal of a role-language read. Distinct from both
    /// caller cancellation and a semantic work limit.
    pub(crate) storage_refusal: Option<purrdf_lex::allocation::StorageError>,
    /// Whether a clash-free completion was found. Only meaningful when `!exhausted`.
    pub(crate) consistent: bool,
    /// Derivation rounds consumed, summed over every branch the search explored.
    pub(crate) steps: u64,
    /// WORK units consumed — the counted cost INSIDE those rounds.
    ///
    /// The quantity `steps` cannot see: matcher join steps, successor-subset enumeration,
    /// achiever closures, neighbour scans and branch-state clones. See [`Work`] for why the
    /// two are separate budgets, and [`work_cap`] for what bounds this one.
    pub(crate) work: u64,
    /// Whether the search stopped because it reached one of its two caps.
    ///
    /// One flag for both, deliberately: a caller's question is whether the answer is a
    /// verdict or a resource limit, and that is one fact. WHICH cap ended the run is read
    /// off the figures — an exhausted run has `steps == step cap` or `work == work cap`,
    /// and both are reported.
    pub(crate) exhausted: bool,
    /// Whether the search stopped because the caller's stop signal fired.
    ///
    /// Distinct from [`Self::exhausted`] because the two are different facts about a run and
    /// only one of them is about this library: a cap reached is a termination-bug backstop
    /// tripping, while this is the host having asked for the run to end. `consistent` is
    /// meaningless under either, and every consumer in this crate reads both before it.
    pub(crate) stopped: bool,
    /// The largest node vector any completion graph the search built ever held.
    ///
    /// A MAXIMUM over branches, not a sum: each branch is a graph of its own, and what a
    /// reader wants to know is how big one got. Merged-away nodes are counted, because a
    /// merge forwards a node through [`find`] rather than freeing it — the vector is what
    /// the search allocated, and it is the quantity blocking is supposed to bound.
    pub(crate) peak_nodes: u64,
    /// How many times the `⊔`-rule branched.
    ///
    /// A SUM over the whole search, one per branch point opened — which is the number of
    /// interior nodes of the search tree, and so the quantity the clausification and the
    /// authored disjunct order exist to hold down.
    ///
    /// In [`crate::owl_dl::hyper`] this is literally the `⊔`-rule and nothing else: that
    /// calculus has exactly one non-deterministic rule, because a `≤n` violation, the
    /// `o`-rule's identification and a disjunctive head are all one clause form. The
    /// `cfg(test)` reference tableau counts the same thing under its own four
    /// non-deterministic rules, which are what the hypertableau compiles INTO the `⊔`-rule,
    /// so the two numbers measure one quantity and are comparable as case splits.
    pub(crate) disjunctions: u64,
    /// The deepest the `⊔`-rule's branch stack ever got, in levels.
    ///
    /// A MAXIMUM, and the one counter that is about the SHAPE of the search tree rather
    /// than its size: a search that is wide and shallow and one that is narrow and deep
    /// spend their rounds very differently, and only this number separates them.
    pub(crate) peak_depth: u64,
}

impl Decision {
    pub(crate) fn undecided(&self) -> bool {
        self.exhausted || self.stopped || self.storage_refusal.is_some()
    }
}

/// The search reached one of its two caps. A private marker rather than an
/// [`EntailError`](crate::EntailError): it is not a failure at this layer, it is one of the
/// three things a decision reports.
#[derive(Debug)]
pub(crate) struct Exhausted;

/// The step cap for a knowledge base: generous and size-proportional.
///
/// # What it bounds, and what it does not
///
/// It is a GLOBAL search budget, summed over every branch the search explores — [`Decision`]'s
/// `steps` says so — and not a bound on one completion graph. Blocking bounds a different
/// quantity: how many NODES one branch may expand, by making the unblocked nodes finitely many
/// (see the signature argument in [`crate::owl_dl::hyper`]). The two are independent, and
/// conflating them is a mistake this comment used to make: it claimed the cap could only be
/// reached by a termination bug or an adversarial instance, and an ordinary satisfiable
/// 17-triple ontology — one `owl:equivalentClass` over an untyped restriction, one
/// `rdfs:range`, one assertion — reached it, because a terminology internalized into every
/// node's label branches once per node per axiom and the SUM over those branches is what this
/// number bounds.
///
/// So what keeps an ordinary ontology far below the cap is not blocking but the encoding it
/// reaches the search under: the canonical normal form
/// ([`Concept::or`](crate::owl_dl::concept::Concept)) deletes the branch points a `⊤`-subsumption
/// used to seed, absorption ([`crate::owl_dl::absorb`]) turns the axioms whose antecedent is
/// faithful into guarded clauses that branch not at all, and what disjunctions survive are
/// tried with their cheap alternatives first
/// ([`Kb::order_disjuncts`](crate::owl_dl::Kb::order_disjuncts)). Reaching the cap means one of
/// those did not bite — which is a fact about the terminology, reportable as
/// `budget-exhausted`, and not a claim that the ontology was adversarial.
///
/// It is a pure function of the knowledge base — same input, same cap — so a [`Decision`] is
/// reproducible run to run, and it is a STEP count rather than a clock reading, which is what
/// keeps it reproducible on wasm32 (where there is no clock to read).
pub(crate) fn step_cap(kb: &Kb) -> u64 {
    100_000 + cap_base(kb).saturating_mul(cap_base(kb)).saturating_mul(64)
}

/// The size both caps are derived from: the axioms, assertions and individuals the search
/// runs over, plus a floor so a tiny knowledge base still gets a usable budget.
fn cap_base(kb: &Kb) -> u64 {
    (kb.abox_types.len() + kb.abox_roles.len() + kb.tbox.len() + kb.individuals.len() + 16) as u64
}

/// The WORK cap for a knowledge base: generous and size-proportional, in the units
/// [`Work`] counts.
///
/// # What it bounds that [`step_cap`] cannot
///
/// A round is not a unit of work. Its cost is the completion graph it runs over times the
/// clauses it matches against it, so per-round work grows with the ontology while the round
/// count need not — and a search can therefore spend unbounded time a few percent into its
/// rounds budget. The shape that demonstrates it is one individual co-typed with `n`
/// equivalence-defined classes: the converse direction of each equivalence reaches the search
/// as a disjunction, the `n` of them interleave on ONE node, and what grows is the matching,
/// the successor-subset enumeration, the achiever closures and the branch-state clones inside
/// each round rather than the number of rounds. This cap is what bounds that class, and the
/// two caps together are what make an unanswerable ontology answer `budget-exhausted`
/// promptly instead of grinding.
///
/// # Where the formula comes from
///
/// MEASURED, over three populations, against the criterion "an ontology this reasoner is
/// expected to decide keeps at least ten times the work it actually spends in hand":
///
/// * **the ledgered fixtures** (`crates/validate/tests/dl_step_ledger.rs`, pinned by
///   `every_ledgered_search_costs_exactly_what_it_is_pinned_to`). The equivalence-over-
///   untyped-restrictions ontology's 17-triple `owl:equivalentClass` shape spends 1,568
///   units; its `rdfs:subClassOf` control — the same seventeen triples with BOTH
///   restrictions moved off the equivalence — 180.
/// * **the differential corpora** of [`crate::owl_dl::oracle`] — 10,400 generated,
///   deliberately adversarial knowledge bases (pinned by
///   `the_enumerated_search_spaces_are_pinned`). Their most expensive DECIDING case spends
///   121,160 units, over a knowledge base whose completion graph reaches 87 nodes, and that
///   margin is ASSERTED, per corpus, by the oracle's `run_property`: a decided case may spend
///   at most a tenth of [`WORK_FLOOR`]. That case is what fixes the constant term: work is a
///   function of the SEARCH rather than of the input's size, so a size-derived cap has to
///   carry a floor generous enough for a small ontology whose search is not. 64 million
///   keeps over 500 times it in hand. The case is a transitive role every element needs a
///   predecessor over, so its completion graph is one long transitive chain that grows by a
///   node a round, every node reading every other in a single neighbourhood read; it spent
///   39,380,845 units when every change re-matched the whole chain and re-walked each
///   closure edge by edge, and what brought it down is delta saturation reaching a node
///   through a closure only for the clauses that read that closure, matching those against
///   what the closure gained, reading cached closures instead of walking them, and billing
///   each step the edges it reads rather than the whole graph's.
/// * **the two block families** of this crate's consistency bench (`benches/consistency.rs`),
///   at 1/2/4/8/16 blocks. The INDEPENDENT family (one individual per block) spends 1,568 /
///   5,692 / 25,427 / 142,033 / 925,037 units and decides at every size. The STACKED family —
///   the same blocks co-typed on ONE individual, which is the shape this cap exists for —
///   spends 1,568 / 21,664 / 123,288 / 453,526 / 1,286,448 / 3,070,426 / 6,479,894 /
///   12,471,548 / 22,344,986 / 37,807,788 at one to ten blocks (the two-block knowledge base
///   is the one the step ledger pins as `co-typed-equivalence-blocks`, at the same 21,664),
///   and 94,793,274 at twelve, and decides every one of them inside its own cap — about one
///   and a half times the work per added block, against a cubic budget that grows by a fifth.
///   The cap still bounds the class: a caller who narrows it gets `unknown` under
///   `completeness budget-exhausted`, with `work` equal to `work-budget` in the certificate,
///   which is the signature `crates/validate/tests/dl_work_budget.rs` pins for ten co-typed
///   copies under a ten-million-unit cap.
///
/// The base is [`cap_base`] — the same size the round cap is derived from — and the formula is
/// `64,000,000 + base³ × 256`. CUBIC rather than the round cap's quadratic, because the two
/// bound quantities one degree apart: a round's own cost is about quadratic in the size (nodes
/// times clauses) and the number of rounds is about linear in it, so a work bound that grew
/// only as fast as the round cap would tighten as ontologies grow.
///
/// What the formula does NOT promise is that every such family decides. The stacked family's
/// work grows geometrically per added block against a cubic budget, so every cap has an `n` it
/// stops at; the honest curve is stated above, and what a cap buys is that the ontology past
/// that `n` ANSWERS — `unknown`, with `completeness budget-exhausted` and `work` equal to
/// `work-budget` — after a bounded, counted search instead of grinding.
///
/// It is a pure function of the knowledge base — same input, same cap — and it is a COUNT
/// rather than a clock reading, which is what keeps a [`Decision`] reproducible run to run
/// and on `wasm32`.
pub(crate) fn work_cap(kb: &Kb) -> u64 {
    let base = cap_base(kb);
    WORK_FLOOR
        + base
            .saturating_mul(base)
            .saturating_mul(base)
            .saturating_mul(256)
}

/// The constant term of [`work_cap`]: the budget a small knowledge base whose SEARCH is large
/// gets whatever its size. See [`work_cap`] for the measurement it rests on.
pub(crate) const WORK_FLOOR: u64 = 64_000_000;

/// Resolve a node index to its union-find representative.
pub(crate) fn find(st: &State, mut x: usize) -> usize {
    while let Some(n) = st.nodes[x].merged {
        x = n;
    }
    x
}

/// Whether `a` and `b` are forced distinct (`a ≠ b`), resolving representatives.
///
/// Two kinds of force, and the second is not a recorded `≠`: an explicit inequality the
/// `≥`-rule or an `owl:differentFrom` put on the graph, and a disagreement of VALUE CLASS.
/// The data domain interprets a literal as its value, so two nodes denoting different values
/// are different elements whether or not anything said so — that is not a unique-name
/// assumption, it is the datatype map.
pub(crate) fn are_distinct(st: &State, a: usize, b: usize) -> bool {
    let a = find(st, a);
    let b = find(st, b);
    if a == b {
        return false;
    }
    if let (Some(left), Some(right)) = (st.nodes[a].value_class, st.nodes[b].value_class)
        && left != right
    {
        return true;
    }
    st.nodes[a].neq.iter().any(|&w| find(st, w) == b)
        || st.nodes[b].neq.iter().any(|&w| find(st, w) == a)
}

/// Record `a ≠ b`.
///
/// Two nodes denoting ONE value cannot be distinct, so forcing an inequality between them is a
/// clash for the same reason forcing one between a node and itself is.
pub(crate) fn set_distinct(st: &mut State, a: usize, b: usize) {
    let a = find(st, a);
    let b = find(st, b);
    if a == b {
        st.clash = true;
        return;
    }
    if let (Some(left), Some(right)) = (st.nodes[a].value_class, st.nodes[b].value_class)
        && left == right
    {
        st.clash = true;
        return;
    }
    st.nodes[a].neq.insert(b);
    st.nodes[b].neq.insert(a);
}

/// A maximum pairwise-compatible subset of `items` (a max clique under `compat`), or
/// `None` when the search exceeded its work budget.
///
/// `compat(a, b)` is `true` when `a` and `b` may coexist (here: are forced `≠`).
/// Deterministic: prefers lower-indexed members.
///
/// Two disciplines keep this from being the exponential cliff a naive backtracking
/// search is. A BRANCH-AND-BOUND prune abandons any partial clique that cannot beat the
/// best found even if every remaining item joins it — on the `≥`-rule's own witness
/// sets, which are pairwise-`≠` BY CONSTRUCTION, the first depth-first descent collects
/// the whole set and the bound then prunes every backtrack, so the common case is
/// quadratic in `n` rather than `2^n`. And a WORK BUDGET bounds the adversarial case —
/// a mixed `≠`-graph the prune does not tame — so exhaustion surfaces as `None`, which
/// callers report as a budget-exhausted decision (`Verdict::Unknown` upstream), never
/// as a hang and never as a guessed verdict. A search cap that counts rounds cannot see
/// work INSIDE a round; this one counts the work itself.
///
/// Its expansions are also charged to the SEARCH's own [`Work`] meter, so a clique search
/// that stays inside its private ceiling still shows up in what the whole decision spent.
/// The two budgets answer different questions — this one bounds ONE clique search, that one
/// bounds the run — and a rule that calls this a thousand times is only visible in the
/// second.
#[cfg(test)]
pub(crate) fn max_clique(
    items: &[usize],
    compat: &dyn Fn(usize, usize) -> bool,
    meter: &Work,
) -> Option<Vec<usize>> {
    clique_of(items, compat, meter, usize::MAX)
}

/// [`max_clique`], stopped as soon as a clique of `enough` members is found.
///
/// A counting rule asks whether `n` pairwise-distinct witnesses exist, not how many do, so
/// the first clique of `n` answers it exactly; a search that never reaches `n` runs to the
/// end and returns the maximum, which is what a rule that has to mint the shortfall needs.
/// Same order, same charges up to the stop, same `None` on exhaustion.
pub(crate) fn clique_of(
    items: &[usize],
    compat: &dyn Fn(usize, usize) -> bool,
    meter: &Work,
    enough: usize,
) -> Option<Vec<usize>> {
    let mut search = Clique {
        items,
        compat,
        meter,
        enough,
        best: Vec::new(),
        current: Vec::new(),
        work: 0,
    };
    search.extend(0).then_some(search.best)
}

/// Expansion-count ceiling for one [`max_clique`] search.
///
/// A pure work bound, not a semantic knob: it changes WHETHER a search finishes inside
/// the budget, never what a finished search answers. Sized far above anything the
/// pruned common case reaches (quadratic in the successor count) while bounding the
/// adversarial mixed-graph case to well under a second.
const MAX_CLIQUE_WORK: u64 = 1 << 20;

/// The most successors one `≥`-rule application will materialize.
///
/// Pairwise distinctness records `n·(n-1)/2` pairs, so the cost of minting is quadratic
/// in the bound whatever the search does; 4,096 witnesses is ~8.4 million pairs, decided
/// exactly and quickly, while a bound beyond it exhausts the decision honestly instead
/// of hanging on gigabytes of bookkeeping. A resource ceiling, not a semantic knob: it
/// never changes a verdict, only whether one is reached inside the budget.
pub(crate) const MAX_COUNTING_WITNESSES: usize = 4096;

/// The backtracking search behind [`clique_of`]; [`Clique::extend`] answering `false` means a
/// work budget ran out — either this search's own local ceiling, or the run's shared [`Work`]
/// meter.
///
/// # The meter is polled DURING the recursion, not after it
///
/// The local `work` counter used to be the only thing charged to `meter`, and only once, after
/// the whole search returned — so a run whose shared meter had almost nothing left still paid
/// for this search's full local ceiling (up to [`MAX_CLIQUE_WORK`]) before anyone found out. A
/// NARROW work cap has to see the exhaustion while the search is still inside it, so every unit
/// is now charged to `meter` at the moment it is spent, and `meter.exhausted()` is checked
/// beside the local ceiling everywhere `work` is — at each call, so a chain of compatible
/// candidates that recurses deeply stops promptly, and inside the candidate loop, so a level
/// whose candidates mostly fail `compat` and never recurse — the scan `compat` alone can make
/// arbitrarily long — stops promptly too. Nothing here reads a clock or a hash iteration order,
/// so which candidate the search is on when it stops is a pure function of `items` and `compat`,
/// and two runs over the same inputs under the same cap stop at the same candidate.
///
/// A caller reads `false` as "budget exhausted" whichever budget it was — [`max_clique`]'s
/// `None` does not distinguish them — because both mean the same thing to every caller: the
/// clique found so far cannot be trusted as maximum, so the decision this feeds must report
/// itself EXHAUSTED rather than a wrong answer built on a truncated search.
struct Clique<'a> {
    /// The candidates, in the order they are tried.
    items: &'a [usize],
    /// Whether two candidates may stand together.
    compat: &'a dyn Fn(usize, usize) -> bool,
    /// The run's shared meter.
    meter: &'a Work,
    /// The clique size that ends the search early.
    enough: usize,
    /// The largest clique found so far.
    best: Vec<usize>,
    /// The clique being extended.
    current: Vec<usize>,
    /// This search's own expansions, against [`MAX_CLIQUE_WORK`].
    work: u64,
}

impl Clique<'_> {
    /// Charge one unit to both budgets; whether neither ran out.
    fn charge(&mut self) -> bool {
        self.work += 1;
        self.meter.charge(1);
        self.work <= MAX_CLIQUE_WORK && !self.meter.exhausted()
    }

    /// Whether the search may stop with what it holds.
    fn done(&self) -> bool {
        self.best.len() >= self.enough
    }

    /// Extend [`Self::current`] with candidates from `start` on; `false` when a budget ran out.
    fn extend(&mut self, start: usize) -> bool {
        if !self.charge() {
            return false;
        }
        if self.current.len() > self.best.len() {
            self.best.clone_from(&self.current);
        }
        if self.done() {
            return true;
        }
        // Bound: even taking every remaining item, this recursive case cannot beat `best`.
        if self.current.len() + (self.items.len() - start) <= self.best.len() {
            return true;
        }
        for i in start..self.items.len() {
            // One unit per CANDIDATE CONSIDERED, not only per recursive call: a level whose
            // candidates mostly fail `compat` never recurses, so without this charge the loop
            // below could walk the whole remaining slice — items.len() - start candidates, which
            // a mixed `≠`-graph can make large — with neither budget seeing it.
            if !self.charge() {
                return false;
            }
            let cand = self.items[i];
            if self.current.iter().all(|&m| (self.compat)(m, cand)) {
                self.current.push(cand);
                if !self.extend(i + 1) {
                    return false;
                }
                self.current.pop();
                if self.done() {
                    return true;
                }
            }
            // Re-check the bound as the window shrinks.
            if self.current.len() + (self.items.len() - i - 1) <= self.best.len() {
                return true;
            }
        }
        true
    }
}

/// The knowledge base plus the internalized TBox, and every operation on a completion graph
/// over them.
///
/// Read-only in the knowledge base and carrying exactly one piece of state of its own — the
/// run's [`Work`] meter. A decision procedure owns its round budget and its rule set, and
/// borrows this for the graph operations both procedures must perform identically; the meter
/// lives here because the work worth counting is done here, and because a counter reachable
/// through `&self` is what lets a scan charge itself without every rule in both calculi
/// growing a `&mut`.
pub(crate) struct Graph<'a> {
    top_role: Option<u32>,
    /// The knowledge base (concept table, role hierarchy, inverses).
    kb: &'a Kb,
    /// The run's work meter — charged by every scan and enumeration below, and by the two
    /// drivers for the work they do outside these methods.
    work: Work,
    /// The internalized TBox: meta-concept ids placed in every abstract node's label.
    meta: BTreeSet<u32>,
    /// Every concept the TBox asserts UNCONDITIONALLY of an element of the object domain: the
    /// internalized meta-concepts, plus the heads of the absorbed clauses with an empty guard
    /// (the `⊤ ⊑ D` inclusions).
    ///
    /// This is what an identification with a literal WITHDRAWS — see [`Graph::merge_nodes`].
    /// The two spellings have to be withdrawn together because they are the same axiom: `⊤ ⊑ D`
    /// reaches an abstract node's label as a seeded meta-concept under one encoding and as a
    /// derived clause head under the other, and a node that turns out to inhabit `Δ_D` was
    /// never constrained by it either way.
    ///
    /// A GUARDED head is deliberately not here. `A ⊑ D` fired because the node carried `A`,
    /// which is a derivation the search made rather than a blanket assertion, and withdrawing
    /// it would need a provenance a label set does not carry.
    unconditional: BTreeSet<u32>,
    /// Memoized [`Self::achievers`] closures, by role.
    ///
    /// The closure is a pure function of the KB's role hierarchy and inverse declarations —
    /// neither changes once a `Graph` is built — so it is computed at most once per role for
    /// the WHOLE search this `Graph` runs, however many nodes and rounds call
    /// [`Self::neighbors`] on it. That matters most for a role that only ever labels an
    /// UNTRIGGERED clause (an `rdfs:domain`/`rdfs:range` axiom with no class in its guard,
    /// see [`crate::owl_dl::clause::ClauseSet::untriggered`]): those clauses are retried at
    /// every node of every round, and before this cache each retry rebuilt the same closure
    /// from scratch.
    achiever_cache: RefCell<AchieverCache>,
    /// The membership scratch every neighbourhood read reuses — see [`ReadScratch`].
    scratch: RefCell<ReadScratch>,
    /// The transitive roles' edge patterns, numbered, which [`Closures`] caches by.
    patterns: TransitivePatterns,
    /// The derivation round a closure member appended now is new to — see [`Reach::fresh_in`].
    /// The search that rounds belong to sets it ([`Self::begin_round`]).
    epoch: std::cell::Cell<u64>,
    /// Node buffers returned by finished reads, handed out again by [`Self::buffer`].
    buffers: RefCell<Vec<Vec<usize>>>,
    /// Absorbed range clauses (`⊤ ⊑ ∀r.DR`, from `rdfs:range` over a data property),
    /// pre-indexed by the edge role — the narrowed data-range ids a `≥n r.DR` counting
    /// question at [`Self::data_clashes`] must fold in.
    ///
    /// Built once here rather than walked per call: [`Self::data_clashes`] used to scan the
    /// WHOLE absorbed table per `Min` concept in a node's label, looking for the one shape
    /// that matches, and an ontology with many domain/range axioms pays for every one of them
    /// at every such node. The lookup is by the role the clause's body atom names VERBATIM —
    /// see the soundness note at [`Self::data_clashes`].
    range_by_role: BTreeMap<Role, Vec<u32>>,
}

impl<'a> Graph<'a> {
    /// Build the graph operations over `kb`, snapshotting the internalized TBox, with a work
    /// meter stopping at `work_cap`.
    pub(crate) fn new(kb: &'a Kb, work_cap: u64) -> Self {
        let meta: BTreeSet<u32> = kb.meta.iter().copied().collect();
        let mut unconditional = meta.clone();
        unconditional.extend(
            kb.absorbed
                .iter()
                .filter(|clause| clause.body.is_empty())
                .map(|clause| clause.head),
        );
        let mut range_by_role: BTreeMap<Role, Vec<u32>> = BTreeMap::new();
        for clause in &kb.absorbed {
            if let [
                BodyAtom::Role {
                    from: 0,
                    to: 1,
                    role,
                },
            ] = clause.body.as_slice()
                && clause.head_var == 1
                && let Decomp::Data(narrowed) = *kb.table.decomp(clause.head)
            {
                range_by_role.entry(*role).or_default().push(narrowed);
            }
        }
        Self {
            top_role: kb
                .interner
                .id_of_iri(purrdf_iri::vocab::owl::TOP_OBJECT_PROPERTY),
            kb,
            work: Work::new(work_cap),
            meta,
            unconditional,
            achiever_cache: RefCell::new(AchieverCache::default()),
            scratch: RefCell::new(ReadScratch::default()),
            patterns: TransitivePatterns::of(kb),
            epoch: std::cell::Cell::new(1),
            buffers: RefCell::new(Vec::new()),
            range_by_role,
        }
    }

    /// The knowledge base every rule reads.
    pub(crate) const fn kb(&self) -> &'a Kb {
        self.kb
    }

    /// The fixed universal role is present even without an authored incident edge.
    pub(crate) const fn has_universal_role(&self) -> bool {
        self.top_role.is_some()
    }

    /// The run's work meter — what both drivers charge their own scans and clones to, and
    /// read the run's work figure off.
    pub(crate) const fn work(&self) -> &Work {
        &self.work
    }

    pub(crate) fn storage_refusal(&self) -> Option<purrdf_lex::allocation::StorageError> {
        self.work.storage_refusal.get()
    }

    /// Latch the original caller stop and inspect the shared read-work refusal.
    /// Both decision drivers use this after scans and before publishing a verdict.
    pub(crate) fn refused(&self, stopped: &mut bool) -> bool {
        *stopped |= self.kb.stopped();
        *stopped || self.work.exhausted()
    }

    /// A fresh label seeded with the internalized TBox.
    pub(crate) fn seed_label(&self) -> BTreeSet<u32> {
        self.meta.clone()
    }

    /// Build the initial completion graph.
    pub(crate) fn init_state(&self, assumptions: &Assumptions<'_>) -> State {
        let Assumptions {
            include_abox,
            types: extra,
            roles: extra_roles,
            fresh_types,
        } = *assumptions;
        let mut st = State {
            support: super::support::Cursor::Disabled,
            nodes: NodeVec::default(),
            edges: PVec::default(),
            adjacency: PVec::default(),
            edges_seen: 0,
            deferred: Vec::new(),
            closures: RefCell::default(),
            blocking: Blocking::default(),
            children: PVec::default(),
            open: NodeSet::default(),
            merged_in: PVec::default(),
            root_of: std::rc::Rc::default(),
            generated_root_of: std::rc::Rc::default(),
            clash: false,
            clique_exhausted: std::cell::Cell::new(false),
        };
        if include_abox {
            for &ind in &self.kb.individuals {
                self.root(&mut st, ind);
            }
            for &(a, c) in &self.kb.abox_types {
                let ra = self.root(&mut st, a);
                st.nodes[ra].label.insert(c);
            }
            for &(a, p, b) in &self.kb.abox_roles {
                let ra = self.root(&mut st, a);
                let rb = self.root(&mut st, b);
                st.push_edge(ra, rb, p);
            }
            for &(a, b) in &self.kb.same_as {
                let ra = self.root(&mut st, a);
                let rb = self.root(&mut st, b);
                self.merge_nodes(&mut st, ra, rb);
            }
            // `owl:differentFrom` / `owl:AllDifferent`, as recorded `≠` pairs. Without
            // them no `≤n r.C` restriction can be violated, because a violation counts
            // PAIRWISE-DISTINCT neighbours and OWL 2 makes no unique name assumption.
            for &(a, b) in &self.kb.different_from {
                let ra = self.root(&mut st, a);
                let rb = self.root(&mut st, b);
                set_distinct(&mut st, ra, rb);
            }
        }
        if self.top_role.is_some() {
            // Every named nominal denotes an element, including a query's
            // previously unmentioned object. The universal role reaches it
            // even when no asserted edge or positive nominal needs that root.
            for concept in 0..self.kb.table.len() {
                if let Decomp::Nominal(members) | Decomp::NegNominal(members) = self
                    .kb
                    .table
                    .decomp(u32::try_from(concept).expect("dense concept id"))
                {
                    for &individual in members {
                        self.root(&mut st, individual);
                    }
                }
            }
        }
        for &(a, c) in extra {
            let ra = self.root(&mut st, a);
            st.nodes[ra].label.insert(c);
        }
        // An assumed role edge, whose endpoints may be individuals the ontology never
        // mentions: `root` mints a node for one on demand, which is what lets a role-axiom
        // refutation run over a pair of fresh symbols.
        for &(a, p, b) in extra_roles {
            let ra = self.root(&mut st, a);
            let rb = self.root(&mut st, b);
            st.push_edge(ra, rb, p);
        }
        // OWL interpretations have a non-empty object domain even with no ABox.
        // A data-only ABox does not supply an object-domain witness either.
        if !fresh_types.is_empty()
            || !(0..st.nodes.len())
                .any(|index| st.nodes[index].merged.is_none() && !st.nodes[index].concrete)
        {
            self.work.charge(self.meta.len() as u64 + 1);
            let mut label = self.seed_label();
            label.extend(fresh_types.iter().copied());
            st.nodes.push(Node {
                label,
                parent: None,
                incoming: None,
                root: true,
                // An anonymous satisfiability/subsumption witness is an unblocked root but NOT a
                // nominal — it denotes no named element — so it must not trigger `NN`/`NI`.
                nominal_id: None,
                nominals: BTreeSet::new(),
                neq: BTreeSet::new(),
                merged: None,
                concrete: false,
                value_class: None,
            });
        }
        st
    }

    /// Get or create the root node for individual term id `a`.
    ///
    /// A LITERAL gets a root here exactly as a named individual does — it is the object of a
    /// data-property assertion and every rule that reads a neighbourhood must see it — but it
    /// is a node of the DATA domain: it carries the literal's value class and it is not seeded
    /// with the internalized TBox, because a general concept inclusion quantifies over
    /// `owl:Thing` and a literal value is not in it.
    pub(crate) fn root(&self, st: &mut State, a: u32) -> usize {
        if let Some(&n) = st.root_of.get(&a) {
            return find(st, n);
        }
        let idx = st.nodes.len();
        let concrete = self.kb.interner.is_literal(a);
        // Minting a node copies the internalized TBox into its label; the meta set's size is
        // therefore what a node costs.
        self.work.charge(self.meta.len() as u64 + 1);
        st.nodes.push(Node {
            label: if concrete {
                BTreeSet::new()
            } else {
                self.seed_label()
            },
            parent: None,
            incoming: None,
            root: true,
            // A named individual is a nominal; a literal's root is not (it denotes a value, not a
            // nominal element), so the nominal-introduction rules never fire on it.
            nominal_id: if concrete {
                None
            } else {
                Some(NominalId::Named(a))
            },
            nominals: std::iter::once(a).collect(),
            neq: BTreeSet::new(),
            merged: None,
            concrete,
            value_class: self.kb.literal_class.get(&a).copied(),
        });
        std::rc::Rc::make_mut(&mut st.root_of).insert(a, idx);
        idx
    }

    /// Get or create the **generated root** node for the typed identity `key`.
    ///
    /// This is the nominal-introduction primitive both cores share: Horrocks & Sattler's
    /// `NN`-rule (concept-tree) and Motik–Shearer–Horrocks' Table 5 `NI`-rule (hypertableau)
    /// both need to mint new NOMINAL ROOT nodes for the nominal/inverse/counting corner, and
    /// they must mint the SAME ones a second firing would, or the graph never converges. It
    /// mirrors [`Graph::root`] exactly — a `root = true` node (so it is never blocked), seeded
    /// with the internalized TBox because it inhabits the OBJECT domain — but it is keyed by a
    /// [`GeneratedRoot`] rather than a term id, so its identity can never alias a named
    /// individual. A repeated call with the same `key` returns the existing node (forwarded
    /// through [`find`] if it has since been merged away), so the reserved pool stays bounded.
    /// It carries no term-space name in [`Node::nominals`]: it denotes a *fresh* element, and a
    /// later identification with a named individual unions that name in through
    /// [`Graph::merge_nodes`] like any other merge.
    pub(crate) fn generated_root(&self, st: &mut State, key: GeneratedRoot) -> usize {
        if let Some(&n) = st.generated_root_of.get(&key) {
            return find(st, n);
        }
        let idx = st.nodes.len();
        // Same charge a named root costs: minting copies the internalized TBox into the label.
        self.work.charge(self.meta.len() as u64 + 1);
        st.nodes.push(Node {
            label: self.seed_label(),
            parent: None,
            incoming: None,
            root: true,
            nominal_id: Some(NominalId::Generated(Box::new(key.clone()))),
            nominals: BTreeSet::new(),
            neq: BTreeSet::new(),
            merged: None,
            concrete: false,
            value_class: None,
        });
        std::rc::Rc::make_mut(&mut st.generated_root_of).insert(key, idx);
        idx
    }

    /// The stable nominal identity of node `x` if it is a NOMINAL root (named or generated),
    /// else `None`. The nominal-introduction rules gate on this rather than on the bare
    /// [`Node::root`] flag, because an anonymous `fresh_types` witness is an unblocked root that
    /// is not a nominal.
    pub(crate) fn nominal_id(&self, st: &State, x: usize) -> Option<NominalId> {
        st.nodes[find(st, x)].nominal_id.clone()
    }

    /// Whether `filler` is a nominal `{o₁,…}` at least one of whose roots counts over an INVERSE
    /// role (`≤n S.C` with `S` an inverse role, or a named role some `owl:inverseOf` makes one).
    ///
    /// This is the ONLY case in which a blocked node's `∃R.{o}` / `≥n R.{o}` obligation must fire:
    /// the nominal's inverse-role count has to see the blocked predecessor, or the bound is
    /// silently under-counted (the incompleteness the nominal-introduction rule repairs). A
    /// nominal that counts nothing over an inverse has no such bound, so blocking may withhold
    /// the obligation as usual — which is what keeps the exemption from firing on every blocked
    /// node and blowing up the search.
    pub(crate) fn nominal_counts_over_inverse(&self, st: &State, filler: u32) -> bool {
        let Decomp::Nominal(members) = self.kb.table.decomp(filler) else {
            return false;
        };
        for &o in members {
            let Some(&n) = st.root_of.get(&o) else {
                continue;
            };
            let node = find(st, n);
            for &cid in &st.nodes[node].label {
                if let Decomp::Max(_, role, _) = *self.kb.table.decomp(cid) {
                    let over_inverse = matches!(role, Role::Inv(_))
                        || matches!(role, Role::Named(p) if self.kb.inverses.contains_key(&p));
                    if over_inverse {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Whether the root `x` names an individual and carries an at-most bound over an inverse
    /// role — what makes [`Self::nominal_counts_over_inverse`] true of a nominal filler naming
    /// it, read from wherever the filler is.
    pub(crate) fn bounds_an_inverse_count(&self, st: &State, x: usize) -> bool {
        let node = &st.nodes[x];
        !node.nominals.is_empty()
            && node.label.iter().any(|&cid| {
                matches!(*self.kb.table.decomp(cid), Decomp::Max(_, role, _)
                    if matches!(role, Role::Inv(_))
                        || matches!(role, Role::Named(p) if self.kb.inverses.contains_key(&p)))
            })
    }

    /// The BLOCKABLE `role`-neighbours `y` of `x` for which `x` is a completion-graph SUCCESSOR of
    /// `y` — `y` GENERATED (the representative of) `x`, and the generating edge, read through the
    /// role hierarchy and inverse closure, makes `y` a `role`-neighbour of `x`.
    ///
    /// This is the trigger shape of the nominal-introduction (`NN`/`NI`) rule. Predecessorhood is
    /// the Horrocks & Sattler completion notion — defined by which node GENERATED which
    /// ([`Node::parent`]/[`Node::incoming`]), NOT by raw RDF edge direction, which cannot tell an
    /// ordinary `role`-successor of `x` from a true predecessor of `x` after a merge, and which
    /// sees only one of the two label spellings (`S = Inv(p)` vs `S = Named(p)`). A blockable tree
    /// node `n` that has since been identified with the nominal `x` (`find(n) == x`) contributes
    /// its own generating predecessor `parent(n)`: if that resolves to a blockable `y` and the
    /// generating role `R` on `n.incoming` makes `y` a `role`-neighbour of `x`, then `x` is a
    /// successor of the blockable `y`. Both `R = Named(p)` and `R = Inv(p)` are handled: the
    /// generating edge is stored `parent → n` for a named `R` (so `x` is its target, matched by
    /// the inverse achiever `(p, false)`) and `n → parent` for an inverse `R` (so `x` is its
    /// source, matched by the forward achiever `(p, true)`). Number-restricted roles are simple,
    /// so no transitive closure is walked.
    pub(crate) fn blockable_predecessor_neighbours(
        &self,
        st: &State,
        x: usize,
        role: Role,
    ) -> Vec<usize> {
        if self.work.exhausted() {
            return Vec::new();
        }
        let ach = self.achievers(role);
        let x = find(st, x);
        self.work.charge(1);
        let mut out: Vec<usize> = Vec::new();
        let mut seen: BTreeSet<usize> = BTreeSet::new();
        // The nodes that resolve to `x` are its union-find class, kept per root, so this reads
        // the class rather than scanning the graph for it: one unit per member, in the
        // ascending order the scan met them.
        for n in st.class_of(x) {
            self.work.charge(1);
            let (Some(parent), Some((prop, inverted))) = (st.nodes[n].parent, st.nodes[n].incoming)
            else {
                continue;
            };
            let y = find(st, parent);
            // A blockable predecessor is a tree node (`!root`); a root parent is not the corner.
            if y == x || st.nodes[y].root {
                continue;
            }
            // `parent` generated `n` under `R` (`incoming`): the edge is `n → parent` for an
            // inverse `R` (x the source, forward achiever) and `parent → n` for a named `R`
            // (x the target, inverse achiever).
            let realized = if inverted {
                realizes(&ach, (prop, true))
            } else {
                realizes(&ach, (prop, false))
            };
            if realized && seen.insert(y) {
                out.push(y);
            }
        }
        out
    }

    /// Merge `discard` into `keep`, identifying the two nodes.
    ///
    /// Orientation keeps a root over a tree node, else the lower index. A forced merge of
    /// a `≠` pair sets [`State::clash`].
    ///
    /// # Why this needs the TBox
    ///
    /// Identifying an abstract node with a literal's node says the abstract node WAS that
    /// literal value all along, and every concept the TBox asserted of it unconditionally
    /// therefore never applied: a general concept inclusion quantifies over `owl:Thing`, and no
    /// literal value is in it. Dropping them needs [`Graph::unconditional`] in scope, which is
    /// what makes this a method rather than a free function over the state.
    pub(crate) fn merge_nodes(&self, st: &mut State, keep: usize, discard: usize) {
        let mut keep = find(st, keep);
        let mut discard = find(st, discard);
        if keep == discard {
            return;
        }
        let kr = st.nodes[keep].root;
        let dr = st.nodes[discard].root;
        let swap = if kr != dr { dr } else { discard < keep };
        if swap {
            std::mem::swap(&mut keep, &mut discard);
        }
        if are_distinct(st, keep, discard) {
            st.clash = true;
            return;
        }
        // Folding one node into another copies its label, its inequalities and its names, so
        // the cost is the size of what is folded. Charged here because identification is how
        // a nominal-heavy ontology spends a round without adding a node to count.
        self.work.charge(
            (st.nodes[discard].label.len()
                + st.nodes[discard].neq.len()
                + st.nodes[discard].nominals.len()) as u64
                + 1,
        );
        // Fold the discarded node's label and distinctness into the keeper.
        let disc_label = st.nodes[discard].label.clone();
        st.nodes[keep].label.extend(disc_label);
        let disc_neq: Vec<usize> = st.nodes[discard].neq.iter().copied().collect();
        for w in disc_neq {
            let w = find(st, w);
            if w == keep {
                st.clash = true;
            }
            st.nodes[keep].neq.insert(w);
            st.nodes[w].neq.insert(keep);
        }
        // Carry every nominal identity onto the keeper. The keeper now denotes *both* names,
        // which is exactly what the absence of a unique name assumption permits. The root map
        // is NOT repointed: every lookup resolves its entry through [`find`], which already
        // lands on the keeper, and repointing would copy the whole map on the first merge of
        // every branch that shares it.
        let disc_nominals = st.nodes[discard].nominals.clone();
        st.nodes[keep].nominals.extend(disc_nominals);
        if st.nodes[discard].root {
            st.nodes[keep].root = true;
        }
        // Carry the nominal identity onto the keeper, preferring a NAMED identity — the
        // Motik–Shearer–Horrocks named-root merge direction: a generated reserved root
        // identified with a named individual becomes that individual, never the other way, which
        // is what keeps the reserved pool from renaming a named element and prevents the
        // caterpillar of reserved-of-reserved roots from growing without bound.
        st.nodes[keep].nominal_id = match (
            st.nodes[keep].nominal_id.take(),
            st.nodes[discard].nominal_id.clone(),
        ) {
            (Some(NominalId::Named(a)), _) | (_, Some(NominalId::Named(a))) => {
                Some(NominalId::Named(a))
            }
            (Some(keep_id), _) => Some(keep_id),
            (None, disc_id) => disc_id,
        };
        // A node identified with a literal's node denotes that literal's value, and inherits
        // both the domain it lives in and the value class that decides its identity.
        // `are_distinct` above already refused the merge when the two classes disagree, so
        // this cannot silently overwrite one value with another.
        if st.nodes[discard].concrete {
            st.nodes[keep].concrete = true;
        }
        if st.nodes[keep].value_class.is_none() {
            st.nodes[keep].value_class = st.nodes[discard].value_class;
        }
        // The keeper is now known to inhabit the DATA domain, so the TBox's unconditional
        // assertions never constrained it — whichever encoding they arrived in. Withdrawing
        // them can only remove a clash, never add one, which is the direction an
        // identification is allowed to move the answer in.
        if st.nodes[keep].concrete {
            for concept in &self.unconditional {
                st.nodes[keep].label.remove(concept);
            }
        }
        st.forward(keep, discard);
    }

    /// Whether a filler concept can only be satisfied by an element of the DATA domain.
    ///
    /// Two shapes say so: a data range, and a nominal naming a literal (which is how
    /// `owl:hasValue` over a data property reads). Both are POSITIVE forms — `¬Data(r)` and
    /// `¬{"cat"}` hold of every abstract element too, so neither says anything about which
    /// domain a node inhabits.
    fn is_concrete_filler(&self, c: u32) -> bool {
        match self.kb.table.decomp(c) {
            Decomp::Data(_) => true,
            Decomp::Nominal(members) => members
                .iter()
                .any(|&member| self.kb.interner.is_literal(member)),
            _ => false,
        }
    }

    /// Add concept `c` to node `y`'s label; `⊤` is trivially present. Returns whether
    /// the label grew.
    pub(crate) fn add_concept(&self, st: &mut State, y: usize, c: u32) -> bool {
        if matches!(self.kb.table.decomp(c), Decomp::Top) {
            return false;
        }
        let y = find(st, y);
        st.nodes[y].label.insert(c)
    }

    /// Whether node `y` satisfies concept `c` (with `⊤` always satisfied).
    pub(crate) fn has_concept(&self, st: &State, y: usize, c: u32) -> bool {
        matches!(self.kb.table.decomp(c), Decomp::Top) || st.nodes[find(st, y)].label.contains(&c)
    }

    /// Create a fresh tree successor of `x` under `role`, labelled with `fillers`.
    ///
    /// A successor whose filler is a DATA RANGE is a node of the data domain, and is therefore
    /// created without the internalized TBox in its label — see [`Node::concrete`].
    pub(crate) fn new_successor(
        &self,
        st: &mut State,
        x: usize,
        role: Role,
        fillers: &[u32],
    ) -> usize {
        let concrete = fillers.iter().any(|&c| self.is_concrete_filler(c));
        let mut label = if concrete {
            BTreeSet::new()
        } else {
            self.seed_label()
        };
        for &c in fillers {
            if !matches!(self.kb.table.decomp(c), Decomp::Top) {
                label.insert(c);
            }
        }
        // The same node cost as [`Graph::root`], plus the fillers placed on it.
        self.work
            .charge((self.meta.len() + fillers.len()) as u64 + 1);
        let idx = st.nodes.len();
        let (prop, inverted) = match role {
            Role::Named(p) => (p, false),
            Role::Inv(p) => (p, true),
        };
        st.nodes.push(Node {
            label,
            parent: Some(x),
            incoming: Some((prop, inverted)),
            root: false,
            // A blockable tree successor is not a nominal.
            nominal_id: None,
            nominals: BTreeSet::new(),
            neq: BTreeSet::new(),
            merged: None,
            concrete,
            value_class: None,
        });
        let parent = find(st, x);
        if st.children.len() <= parent {
            st.children.resize_with(parent + 1, std::rc::Rc::default);
        }
        std::rc::Rc::make_mut(&mut st.children[parent]).push(idx);
        // A forward role stores `x → y`; an inverse role stores `y → x`.
        if inverted {
            st.push_edge(idx, x, prop);
        } else {
            st.push_edge(x, idx, prop);
        }
        idx
    }

    /// The `role`-neighbours of `x` (deterministic, first-seen edge order).
    ///
    /// # Transitivity is in the NEIGHBOURHOOD, not in a second rule
    ///
    /// A role declared `owl:TransitiveProperty` contributes its TRANSITIVE CLOSURE here, so
    /// every rule that reads a neighbourhood — every clause body atom over a role, every
    /// counting rule, the two role axioms — sees the semantics of the transitive role without
    /// any of them being taught about transitivity. `∀r.C` therefore propagates `C` along a
    /// whole `r`-path, which is exactly what transitivity entails, and it does so without the
    /// `∀+` rule's habit of interning a fresh `∀s.C` concept mid-search (the concept table is
    /// finalized before any decision starts, so there is no fresh concept to intern).
    ///
    /// The closure is taken per transitive achiever, never over the union: `q ⊑ r` with `q`
    /// transitive and `r` not gives `r` every `q⁺`-pair, but two DIFFERENT sub-roles of `r`
    /// do not compose into one — `r` itself is not transitive, and composing them would
    /// invent pairs the ontology does not entail.
    ///
    /// A step of a transitive achiever `t` is, in turn, an edge realizing `t` — through `t`'s
    /// OWN closure under sub-roles and inverses, not `t`'s name alone. `s ⊑ t` with `t`
    /// transitive makes every `s`-edge a `t`-edge, so `x s y, y t z` is a `t`-path and `t⁺`
    /// relates `x` to `z`; likewise an `owl:inverseOf` partner of `t` stored the other way
    /// round. Walking `t`-labelled edges alone would miss both pairs and answer `consistent`
    /// for a knowledge base whose only model needs them.
    ///
    /// Counting a transitive role's neighbours in a `≤n` restriction is only meaningful
    /// because OWL 2 DL forbids exactly that combination; an ontology that states it is not
    /// OWL 2 DL and the reverse mapping raises
    /// [`Construct::NonSimpleRole`](crate::Construct::NonSimpleRole) for it.
    pub(crate) fn neighbors(&self, st: &State, x: usize, role: Role) -> Buffer<'_> {
        let mut out = self.buffer();
        self.neighbors_into(st, x, role, &mut out);
        out
    }

    /// [`Self::neighbors`], written into `out` (cleared first) rather than into a buffer of
    /// its own.
    pub(crate) fn neighbors_into(&self, st: &State, x: usize, role: Role, out: &mut Vec<usize>) {
        out.clear();
        self.read_neighbours(st, x, role, &mut |y| {
            out.push(y);
            false
        });
    }

    /// Whether `target` is a `role`-neighbour of `x`.
    ///
    /// The read [`Self::neighbors`] makes, stopped at `target`: what was visited before it is
    /// visited in the same order and charged the same, and nothing after it is read at all.
    pub(crate) fn is_neighbour(&self, st: &State, x: usize, role: Role, target: usize) -> bool {
        let target = find(st, target);
        self.read_neighbours(st, x, role, &mut |y| y == target)
    }

    /// Whether some `role`-neighbour of `x` satisfies `test`, read in [`Self::neighbors`]'s
    /// order and stopped at the first that does.
    ///
    /// `test` runs while the read's scratch is held, so it must not read a neighbourhood
    /// itself; a label or identity test is what it is for.
    pub(crate) fn any_neighbour(
        &self,
        st: &State,
        x: usize,
        role: Role,
        test: &mut dyn FnMut(usize) -> bool,
    ) -> bool {
        self.read_neighbours(st, x, role, test)
    }

    /// A cleared buffer from the read pool, returned to it when dropped.
    ///
    /// The pool is how a neighbourhood read allocates nothing in steady state: a clause match
    /// holds one buffer per role atom it is iterating, so the pool grows to the deepest body
    /// a round matches and is reused from then on.
    pub(crate) fn buffer(&self) -> Buffer<'_> {
        let items = self.buffers.borrow_mut().pop().unwrap_or_default();
        Buffer {
            items,
            pool: &self.buffers,
        }
    }

    /// Evaluate the compiled language over the original completion edges and
    /// equality representatives. Product identities are discovered before they
    /// are queued; epsilon cycles and graph cycles therefore terminate without
    /// imposing a path-length bound. All scratch dies before its original grant.
    fn read_role_language(
        &self,
        st: &State,
        x: usize,
        machine: &super::roles::Automaton,
        visit: &mut dyn FnMut(usize) -> bool,
    ) -> Result<bool, purrdf_lex::allocation::StorageError> {
        use purrdf_lex::allocation::{Memory, Resident};
        let mut resident = Resident;
        let mut memory = Memory::new(&mut resident);
        memory.scope(|memory| {
            let mut pending = Vec::new();
            let mut reached = Vec::new();
            let mut emitted = Vec::new();
            let initial = (find(st, x), machine.initial);
            memory.push(&mut reached, initial)?;
            memory.push(&mut pending, initial)?;
            let mut answer = false;
            while let Some((node, state)) = pending.pop() {
                if self.work.exhausted() || self.kb.stopped() {
                    break;
                }
                self.work.charge(1);
                if state == machine.accepting
                    && let Err(position) = emitted.binary_search(&node)
                {
                    memory.reserve_for_push(&mut emitted)?;
                    emitted.insert(position, node);
                    if visit(node) {
                        answer = true;
                        break;
                    }
                }
                for transition in &machine.transitions {
                    self.work.charge(1);
                    if self.work.exhausted() || self.kb.stopped() {
                        break;
                    }
                    if transition.from != state {
                        continue;
                    }
                    let mut enqueue =
                        |target: usize| -> Result<(), purrdf_lex::allocation::StorageError> {
                            let next = (target, transition.to);
                            if let Err(position) = reached.binary_search(&next) {
                                memory.reserve_for_push(&mut reached)?;
                                reached.insert(position, next);
                                memory.push(&mut pending, next)?;
                            }
                            Ok(())
                        };
                    if let Some(letter) = transition.letter {
                        let (Role::Named(property) | Role::Inv(property)) = letter;
                        if self.top_role == Some(property) {
                            if st.nodes[node].concrete {
                                continue;
                            }
                            for target in 0..st.nodes.len() {
                                if self.work.exhausted() || self.kb.stopped() {
                                    break;
                                }
                                self.work.charge(1);
                                if find(st, target) == target && !st.nodes[target].concrete {
                                    enqueue(target)?;
                                }
                            }
                            continue;
                        }
                        if !self.charge_step(st, node) {
                            break;
                        }
                        for &edge in st.class_edges(node) {
                            let (from, to, property) = st.edges[edge];
                            let from = find(st, from);
                            let to = find(st, to);
                            match letter {
                                Role::Named(wanted) if wanted == property && from == node => {
                                    enqueue(to)?;
                                }
                                Role::Inv(wanted) if wanted == property && to == node => {
                                    enqueue(from)?;
                                }
                                _ => {}
                            }
                        }
                    } else {
                        enqueue(node)?;
                    }
                }
            }
            memory.release_vec(emitted)?;
            memory.release_vec(reached)?;
            memory.release_vec(pending)?;
            Ok(answer)
        })
    }

    /// The one neighbourhood read: call `visit` on every `role`-neighbour of `x` in
    /// first-seen order, each once, and stop the moment it answers `true`. Returns whether it
    /// did.
    ///
    /// The order is the direct step's first — every edge of `x`'s class in ascending edge
    /// order, realized forward then backward — and then each transitive achiever's closure,
    /// walked depth-first from `x` over that achiever's own edges. Membership is kept in two
    /// GENERATION-STAMPED vectors indexed by node rather than in sets built per read: a
    /// neighbour has been emitted exactly when its `seen` stamp is this read's, and reached by
    /// the current closure walk exactly when its `visited` stamp is that walk's. A new read or
    /// walk bumps a counter instead of clearing anything, so a read allocates nothing once the
    /// stamps have grown to the graph.
    ///
    /// Charged for what it reads: the achiever closure, then each step's edges
    /// ([`Self::charge_step`]). A read stopped early charges only the
    /// steps it took, and a transitive closure this state has cached ([`Closures`]) is read
    /// for its member count instead of walked — a walk that will be cached runs to its end
    /// even once `visit` has its answer, so the next read is a cache read.
    fn read_neighbours(
        &self,
        st: &State,
        x: usize,
        role: Role,
        visit: &mut dyn FnMut(usize) -> bool,
    ) -> bool {
        // A neighbourhood read is the single most-called scan in either calculus — every
        // clause body atom over a role, every counting rule and every satisfaction test goes
        // through it — so it is where an unbounded search spends most of what a round cap
        // cannot see.
        if self.work.exhausted() {
            return false;
        }
        // The fixed universal role ranges over every current object-domain
        // representative, including isolated roots and newly minted witnesses.
        // It is not approximated by paths through authored top-role assertions.
        let (Role::Named(property) | Role::Inv(property)) = role;
        if self.top_role == Some(property)
            || self.top_role.is_some_and(|top| {
                let achievers = self.achievers(role);
                realizes(&achievers, (top, true)) || realizes(&achievers, (top, false))
            })
            || self.kb.role_program.as_ref().is_some_and(|program| {
                program.top.is_some_and(|top| {
                    match (
                        program.roles.binary_search(&top),
                        program.roles.binary_search(&role),
                    ) {
                        (Ok(top), Ok(role)) => {
                            program.machine_for[top] == program.machine_for[role]
                        }
                        _ => false,
                    }
                })
            })
        {
            if st.nodes[find(st, x)].concrete {
                return false;
            }
            for node in 0..st.nodes.len() {
                if self.work.exhausted() || self.kb.stopped() {
                    return false;
                }
                self.work.charge(1);
                if find(st, node) == node && !st.nodes[node].concrete && visit(node) {
                    return true;
                }
            }
            return false;
        }
        if let Some(program) = &self.kb.role_program
            && let Some(machine) = program.machine(role)
        {
            return match self.read_role_language(st, x, machine, visit) {
                Ok(found) => found,
                Err(original) => {
                    if self.work.storage_refusal.get().is_none() {
                        self.work.storage_refusal.set(Some(original));
                    }
                    // Stop at the original work poll without inventing work to
                    // reach its cap. Decision retains the physical cause and
                    // the exact work consumed before that failure.
                    false
                }
            };
        }
        let ach = self.achievers(role);
        let x = find(st, x);
        self.integrate(st);
        let mut guard = self.scratch.borrow_mut();
        let scratch = &mut *guard;
        scratch.begin(st.nodes.len());
        if !self.charge_step(st, x) {
            return false;
        }
        for &edge in st.class_edges(x) {
            let (from, to, prop) = st.edges[edge];
            let f = find(st, from);
            let t = find(st, to);
            if realizes(&ach, (prop, true)) && f == x && scratch.emit(t) && visit(t) {
                return true;
            }
            if realizes(&ach, (prop, false)) && t == x && scratch.emit(f) && visit(f) {
                return true;
            }
        }
        let mut stopped = false;
        for &(prop, dir) in ach.iter() {
            let Some(index) = self.patterns.index((prop, dir)) else {
                continue;
            };
            if self.work.exhausted() {
                return false;
            }
            // A closure this state has read before is read off its cache, for the length of
            // its member list; walking it again would charge an edge scan per member.
            let cached = st.closures.borrow().get(index, x).cloned();
            if let Some(reach) = cached {
                self.work.charge(reach.order.len() as u64 + 1);
                for &y in &reach.order {
                    if scratch.emit(y) && visit(y) {
                        return true;
                    }
                }
                continue;
            }
            // A step of the transitive role `T` this pattern names is an edge realizing `T`
            // ITSELF — any of `T`'s own achievers, its sub-roles and inverse partners — and not
            // only an edge labelled with `T`'s name: `s ⊑ t` with `t` transitive makes
            // `x s y, y t z` a `t`-path, so `t⁺` relates `x` to `z`.
            let single = self.achievers(pattern_role(prop, dir));
            scratch.begin_walk();
            // A walk that will be cached runs to the end even once `visit` has its answer, so
            // the next read of this closure is a cache read; one past the cache's ceiling
            // stops where `visit` does, as every read used to.
            let record = st.closures.borrow().cached < MAX_CACHED_MEMBERS;
            scratch.members.clear();
            // Depth-first over this one transitive role, seeded from `x`'s own step.
            if !self.charge_step(st, x) {
                return false;
            }
            Self::reach(st, x, &single, scratch);
            while let Some(y) = scratch.frontier.pop() {
                // The transitive closure is the one loop here whose length is a function of
                // the graph rather than of the role hierarchy, so it is polled as well as
                // charged: a run whose budget went while this was running stops here.
                if self.work.exhausted() {
                    return false;
                }
                if record {
                    scratch.members.push(y);
                }
                if !stopped && scratch.emit(y) && visit(y) {
                    if !record {
                        return true;
                    }
                    stopped = true;
                }
                if !self.charge_step(st, y) {
                    return false;
                }
                Self::reach(st, y, &single, scratch);
            }
            if record {
                st.closures.borrow_mut().store(
                    self.patterns.len(),
                    index,
                    x,
                    &scratch.members,
                    self.epoch.get(),
                );
            }
            if stopped {
                return true;
            }
        }
        false
    }

    /// Bring the state's cached closures up to date with the edges appended since they were
    /// last brought up to date — see [`Closures`].
    ///
    /// Edge by edge, in append order: a pattern the edge realizes as a step `src → dst` gains
    /// `dst` and `dst`'s closure in every cached closure with `src` as a member, and in `src`'s
    /// own. Taken in order, that is exactly the closure of the graph with every edge in it — a
    /// member a later edge adds reaches the closures that gained it through the earlier ones,
    /// because those now list it among their members.
    fn integrate(&self, st: &State) {
        let pending = {
            let closures = st.closures.borrow();
            if closures.integrated >= st.edges.len() {
                return;
            }
            closures.integrated..st.edges.len()
        };
        st.closures.borrow_mut().integrated = st.edges.len();
        if self.patterns.len() == 0 {
            return;
        }
        let round = self.epoch.get();
        for edge in pending {
            let (from, to, property) = st.edges[edge];
            let (from, to) = (find(st, from), find(st, to));
            for index in 0..self.patterns.len() {
                let forward = self.patterns.forward(index);
                for (src, dst, realized) in [
                    (from, to, realizes(forward, (property, true))),
                    (to, from, realizes(forward, (property, false))),
                ] {
                    if !realized {
                        continue;
                    }
                    let mut owners: Vec<usize> =
                        st.closures.borrow().readers_of(index, src).to_vec();
                    if st.closures.borrow().get(index, src).is_some() {
                        owners.push(src);
                    }
                    if owners.is_empty() {
                        continue;
                    }
                    let mut gain = vec![dst];
                    gain.extend(self.closure_members(st, dst, index));
                    self.work.charge((owners.len() + gain.len()) as u64);
                    let mut closures = st.closures.borrow_mut();
                    for owner in owners {
                        closures.extend(index, owner, &gain, round);
                    }
                }
            }
        }
    }

    /// The members of `x`'s closure over pattern `index`: the cached ones, or a walk's.
    fn closure_members(&self, st: &State, x: usize, index: usize) -> Vec<usize> {
        if let Some(reach) = st.closures.borrow().get(index, x) {
            return reach.order.clone();
        }
        let single = self.patterns.forward(index);
        let mut out: Vec<usize> = Vec::new();
        let mut visited: BTreeSet<usize> = BTreeSet::new();
        let mut frontier: Vec<usize> = Vec::new();
        self.charge_step(st, x);
        for_each_step(st, x, single, |z| {
            if visited.insert(z) {
                frontier.push(z);
            }
        });
        while let Some(y) = frontier.pop() {
            if self.work.exhausted() {
                break;
            }
            out.push(y);
            self.charge_step(st, y);
            for_each_step(st, y, single, |z| {
                if visited.insert(z) {
                    frontier.push(z);
                }
            });
        }
        out
    }

    /// Start derivation round `round`: bring the cached closures up to date, so that what
    /// the edges appended since the last round added to them is new to THIS round, and then
    /// mark what is appended from now on as new to the next one.
    pub(crate) fn begin_round(&self, st: &State, round: u64) {
        self.epoch.set(round);
        self.integrate(st);
        self.epoch.set(round + 1);
    }

    /// The cached closure of the root `x` over transitive pattern `index`, brought up to date,
    /// if a read has cached it.
    pub(crate) fn reach_of(
        &self,
        st: &State,
        x: usize,
        index: usize,
    ) -> Option<std::rc::Rc<Reach>> {
        self.integrate(st);
        st.closures.borrow().get(index, find(st, x)).cloned()
    }

    /// The transitive roles' numbered edge patterns.
    pub(crate) const fn patterns(&self) -> &TransitivePatterns {
        &self.patterns
    }

    /// Charge one edge step from `y`, and say whether the budget still allows it.
    ///
    /// One unit per edge the step examines — the edges indexed under `y`'s class
    /// ([`State::class_edges`]), which are all a step reads — plus one for the step itself, so
    /// a node with no edge still costs its lookup. The meter bills what a read actually does,
    /// so a choice beside a large saturated graph costs what it changes and not the graph's
    /// size. The check right after the charge is what a NARROW cap needs: a class whose edge
    /// count alone exhausts the meter must not still be walked before the read returns.
    fn charge_step(&self, st: &State, y: usize) -> bool {
        self.work
            .charge(st.class_edges(find(st, y)).len() as u64 + 1);
        !self.work.exhausted()
    }

    /// One closure step from `y` over the patterns `ach`: push every endpoint the current walk
    /// has not reached onto the frontier, in edge order.
    fn reach(st: &State, y: usize, ach: &[(u32, bool)], scratch: &mut ReadScratch) {
        for_each_step(st, y, ach, |z| {
            if scratch.reach(z) {
                scratch.frontier.push(z);
            }
        });
    }

    /// The `(property, forward?)` edge patterns that realize `role`, closed under the
    /// role hierarchy and inverse-role declarations.
    ///
    /// Memoized in [`Self::achiever_cache`]: the closure is a pure function of the KB's role
    /// axioms, which do not change while this `Graph` runs a search, so it is built at most
    /// once per role however many times [`Self::neighbors`] asks for it. A HIT is charged one
    /// unit — a lookup and a clone of a small set — and only a MISS pays for the stack walk
    /// below, so the meter still moves on every call but the search now pays the closure's
    /// real cost once rather than once per call.
    fn achievers(&self, role: Role) -> Achievers {
        if let Some(cached) = self.achiever_cache.borrow().get(role) {
            self.work.charge(1);
            return std::rc::Rc::clone(cached);
        }
        let start = match role {
            Role::Named(p) => (p, true),
            Role::Inv(p) => (p, false),
        };
        let mut set: BTreeSet<(u32, bool)> = BTreeSet::new();
        let mut stack = vec![start];
        // One unit for the call itself, charged up front, so a role with no sub-roles and no
        // inverse — which closes without a single stack pop — still moves the meter: a scan
        // that charged zero would let a rule call it forever without the meter ever seeing it.
        self.work.charge(1);
        // One unit per closure expansion, charged and POLLED as the stack is walked rather than
        // summed and charged once after — a role hierarchy deep or wide enough to make this
        // walk itself the expensive part must be cut off by a NARROW cap exactly as every other
        // bulk enumeration here is, rather than running the whole walk before the meter is
        // consulted.
        while let Some((q, dir)) = stack.pop() {
            if self.work.exhausted() {
                // Exhausted mid-walk: the closure this stack was building is incomplete, so it
                // must never be memoized — a cached partial closure would silently answer every
                // later call for this role, in this run or (were the cap raised) a later one,
                // with fewer achievers than the role hierarchy actually has.
                return set.into_iter().collect();
            }
            self.work.charge(1);
            if !set.insert((q, dir)) {
                continue;
            }
            if let Some(subs) = self.kb.role_sub.get(&q) {
                for &s in subs {
                    stack.push((s, dir));
                }
            }
            if let Some(invs) = self.kb.inverses.get(&q) {
                for &s in invs {
                    stack.push((s, !dir));
                }
            }
        }
        // Ascending, as `BTreeSet` iterates: [`realizes`] binary-searches it.
        let closed: Achievers = set.into_iter().collect();
        self.achiever_cache
            .borrow_mut()
            .insert(role, std::rc::Rc::clone(&closed));
        closed
    }

    /// Whether `x` has a `role`-edge to itself, read through the role hierarchy and the
    /// inverse-role closure.
    pub(crate) fn has_self_loop(&self, st: &State, x: usize, role: Role) -> bool {
        let x = find(st, x);
        self.is_neighbour(st, x, role, x)
    }

    /// Give `x` a `role`-edge to itself, if it has none. Returns whether an edge was added.
    ///
    /// The edge, not a fresh successor: `∃r.Self` says the node is its OWN `r`-successor,
    /// which is why it is an atomic leaf rather than a quantifier and why this is
    /// deterministic and terminating — a node has at most one loop per role.
    pub(crate) fn add_self_loop(&self, st: &mut State, x: usize, role: Role) -> bool {
        if self.has_self_loop(st, x, role) {
            return false;
        }
        let x = find(st, x);
        // A loop is its own inverse, so the direction the edge is stored in does not matter;
        // the named property is what the role hierarchy is closed over.
        let (Role::Named(property) | Role::Inv(property)) = role;
        st.push_edge(x, x, property);
        true
    }

    /// Whether the CONCRETE-domain constraints on `x` have no solution.
    ///
    /// A node labelled `Data(r₁) … Data(rₘ) ¬Data(s₁) … ¬Data(sₖ)` denotes a literal value in
    /// `r₁ ∩ … ∩ rₘ ∩ ¬s₁ ∩ … ∩ ¬sₖ`, and an EMPTY intersection has no such value. That is the
    /// whole of the concrete-domain decision procedure at this layer, and it is
    /// [`purrdf_xsd::range`]'s answer rather than a second datatype model written beside it.
    ///
    /// Only a PROVED emptiness closes the branch. A range the decision procedure cannot decide
    /// answers "not provably empty" and is reported as a boundary instead, because inventing an
    /// inconsistency is the one error a reasoner cannot recover from.
    ///
    /// The second half is the counting question a per-node emptiness check cannot see: `≥n r.DR`
    /// demands `n` PAIRWISE-DISTINCT values of `DR`, and the data domain has no unique-name
    /// freedom to supply them from, so a range holding fewer than `n` values refutes the
    /// restriction outright. Every `∀r.DR′` on the same node narrows the range those witnesses
    /// are drawn from, so the two are counted together.
    ///
    /// An ontology stating no data range and holding no literal skips all of it.
    pub(crate) fn data_clashes(&self, st: &State, x: usize) -> bool {
        if self.kb.data_ranges.is_empty() {
            return false;
        }
        // Two label scans and, per at-least-over-a-data-range concept, a third plus a lookup
        // in the pre-indexed range clauses ([`Self::range_by_role`]). An ontology that states
        // no data range paid nothing above; one that does pays for what it made this method
        // read.
        self.work.charge(st.nodes[x].label.len() as u64 + 1);
        let mut positive: Vec<u32> = Vec::new();
        let mut negative: Vec<u32> = Vec::new();
        for &cid in &st.nodes[x].label {
            match *self.kb.table.decomp(cid) {
                Decomp::Data(range) => positive.push(range),
                Decomp::NegData(range) => negative.push(range),
                _ => {}
            }
        }
        if (!positive.is_empty() || !negative.is_empty())
            && self
                .kb
                .data_ranges
                .conjunction_is_empty(&positive, &negative)
        {
            return true;
        }
        for &cid in &st.nodes[x].label {
            let Decomp::Min(n, role, filler) = *self.kb.table.decomp(cid) else {
                continue;
            };
            let Decomp::Data(range) = *self.kb.table.decomp(filler) else {
                continue;
            };
            let mut demanded = vec![range];
            // A `⊤ ⊑ ∀r.DR` axiom — every `rdfs:range` over a data property — is an EDGE
            // CLAUSE rather than a label ([`crate::owl_dl::absorb`]), so the narrowing it
            // contributes is read from [`Self::range_by_role`] rather than off this node's
            // own label. It narrows every node's `r`-successors unconditionally, which is
            // what an unguarded range clause says, and it used to be read off the label back
            // when a range axiom was internalized into it. Losing it here would silently
            // weaken the counting question below on exactly the ontologies that state a
            // range.
            //
            // The lookup key is the role a range clause's body atom names VERBATIM, at
            // absorption time — not the achiever closure [`Self::achievers`] resolves for a
            // `neighbors` scan. A range stated over `Named(p)` therefore narrows a `Min`
            // counted over `Named(p)` itself but is silently absent for one counted over
            // `Inv(p)` or over a super-role of `p`: the match is SYNTACTIC, so it can only
            // WITHHOLD a clash a sub-role or inverse relationship would in fact justify, never
            // manufacture one that is not there. That keeps this sound and incomplete rather
            // than unsound, and completeness for the syntactic cases this table cannot see is
            // recovered by the meta-encoding of the same `rdfs:range` axiom on every other
            // role spelling.
            //
            // AND IT NARROWS NOTHING AT A NODE OF `Δ_D`, for the reason a TBox clause is not
            // fired from one ([`crate::owl_dl::hyper`]'s round) and unconditional consequents
            // are withdrawn from one ([`Self::merge_nodes`]): `⊤ ⊑ ∀r.DR` quantifies over
            // `owl:Thing`, so it says nothing about the `r`-successors of an element that
            // turns out to be a literal VALUE. The gate has to be here as well as at the
            // firing site because this is a SECOND reader of the same clause — one that
            // consults the clause table directly rather than waiting for a head to be
            // derived — and without it the two TBox encodings answer differently: the
            // internalized `∀r.DR` is withdrawn from the merged node's label while the
            // absorbed clause was still folded in here. That is a knowledge base the absorbed
            // encoding refuted and the all-meta one satisfied, which is how the differential
            // corpus of [`crate::owl_dl::oracle`] found it.
            let role_ranges = if st.nodes[x].concrete {
                &[][..]
            } else {
                self.range_by_role.get(&role).map_or(&[][..], Vec::as_slice)
            };
            self.work
                .charge((st.nodes[x].label.len() + role_ranges.len()) as u64);
            for &other in &st.nodes[x].label {
                if let Decomp::All(universal_role, universal_filler) = *self.kb.table.decomp(other)
                    && universal_role == role
                    && let Decomp::Data(narrowed) = *self.kb.table.decomp(universal_filler)
                {
                    demanded.push(narrowed);
                }
            }
            demanded.extend_from_slice(role_ranges);
            if self.kb.data_ranges.provably_fewer_than(&demanded, n) {
                return true;
            }
        }
        false
    }

    /// Ensure `x` has `n` pairwise-`≠` `role`-neighbours satisfying `filler`, minting the
    /// missing ones. Returns whether the graph changed.
    ///
    /// The witness discipline both calculi share: existing neighbours are counted first (as a
    /// maximum `≠`-clique, because OWL 2 makes no unique name assumption and two neighbours
    /// nothing forced apart may be one element), fresh successors make up the shortfall, and
    /// the whole witness set is then forced pairwise distinct — which is what makes `≥n`
    /// demand `n` ELEMENTS rather than `n` edges. No IRI is minted: a witness is an anonymous
    /// tree node.
    pub(crate) fn ensure_at_least(
        &self,
        st: &mut State,
        x: usize,
        n: u32,
        role: Role,
        filler: u32,
    ) -> bool {
        let n = n as usize;
        if n == 0 {
            return false;
        }
        let mut with_filler = self.neighbors(st, x, role);
        with_filler.retain(|&y| self.has_concept(st, y, filler));
        // Only whether `n` are already there decides anything: a clique that reaches `n` is
        // as good as the maximum, and one that cannot is the maximum.
        let Some(mut clique) =
            clique_of(&with_filler, &|a, b| are_distinct(st, a, b), &self.work, n)
        else {
            // Clique-work exhaustion: surface as search exhaustion, never as a guess.
            st.clique_exhausted.set(true);
            return false;
        };
        if clique.len() >= n {
            return false;
        }
        // The witnesses to mint are pairwise-`≠`, which is quadratically many recorded
        // pairs: a bound that large is a resource statement, not a logical one, so past
        // this ceiling the decision degrades to EXHAUSTED (Unknown upstream) — the same
        // honest three-valued answer every other budget produces — rather than spending
        // gigabytes of `≠`-pairs or answering wrongly.
        if n > MAX_COUNTING_WITNESSES {
            st.clique_exhausted.set(true);
            return false;
        }
        while clique.len() < n {
            let y = self.new_successor(st, x, role, &[filler]);
            clique.push(y);
        }
        // Forcing the witness set pairwise distinct records `n·(n-1)/2` inequalities, which is
        // quadratic bookkeeping the `≥`-rule does in ONE round however large `n` is — the
        // clearest example of work a rounds cap cannot see.
        self.work
            .charge((clique.len() as u64).saturating_mul(clique.len() as u64));
        for a in 0..clique.len() {
            for b in (a + 1)..clique.len() {
                set_distinct(st, clique[a], clique[b]);
            }
        }
        true
    }

    /// Whether `x` already has `n` pairwise-`≠` `role`-neighbours satisfying `filler`.
    pub(crate) fn has_at_least(
        &self,
        st: &State,
        x: usize,
        n: u32,
        role: Role,
        filler: u32,
    ) -> bool {
        if n == 0 {
            return true;
        }
        // One witness is a clique of one: the first neighbour carrying the filler settles it,
        // and the read stops there.
        if n == 1 {
            return self.any_neighbour(st, x, role, &mut |y| self.has_concept(st, y, filler));
        }
        let mut with_filler = self.neighbors(st, x, role);
        with_filler.retain(|&y| self.has_concept(st, y, filler));
        match clique_of(
            &with_filler,
            &|a, b| are_distinct(st, a, b),
            &self.work,
            n as usize,
        ) {
            Some(clique) => clique.len() >= n as usize,
            None => {
                // Exhaustion is recorded on the state the caller already consults; a
                // `false` here only ever WITHHOLDS a satisfaction claim, which the
                // exhausted decision then reports as Unknown rather than as an answer.
                st.clique_exhausted.set(true);
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[global_allocator]
    static GLOBAL: purrdf_alloc_probe::CountingAllocator = purrdf_alloc_probe::CountingAllocator;

    /// THE PERSISTENT VECTOR IS A VECTOR: pushes, reads and writes across every height the
    /// radix tree grows through agree with a plain one, and a clone taken at any point is
    /// untouched by the writes made to the original after it.
    #[test]
    fn a_persistent_vector_behaves_as_a_vector_and_its_clones_stay_put() {
        let mut draw = purrdf_testkit::rng::SplitMix64::new(0x0050_FEC5);
        let mut persistent: PVec<u64> = PVec::default();
        let mut plain: Vec<u64> = Vec::new();
        let mut snapshots: Vec<(PVec<u64>, Vec<u64>)> = Vec::new();
        for step in 0..300_000_u64 {
            if plain.is_empty() || draw.below(4) != 0 {
                persistent.push(step);
                plain.push(step);
            } else {
                let at = draw.below_usize(plain.len());
                persistent[at] = step;
                plain[at] = step;
            }
            if step % 37_000 == 0 {
                snapshots.push((persistent.clone(), plain.clone()));
            }
        }
        assert_eq!(persistent.len(), plain.len());
        assert!(
            persistent.len() > LEAF * BRANCH * 2,
            "the tree grew past two levels"
        );
        assert!(persistent.iter().eq(plain.iter()));
        assert_eq!(persistent.get(plain.len()), None);
        for (snapshot, expected) in &snapshots {
            assert!(snapshot.iter().eq(expected.iter()), "a clone moved");
        }
    }

    /// A bare tree node: no label, no incoming edge, not a root — the minimal shape these
    /// tests need to populate a [`State`] by hand rather than through a knowledge base.
    fn bare_node(root: bool) -> Node {
        Node {
            label: BTreeSet::new(),
            parent: None,
            incoming: None,
            root,
            nominal_id: None,
            nominals: BTreeSet::new(),
            neq: BTreeSet::new(),
            merged: None,
            concrete: false,
            value_class: None,
        }
    }

    /// A two-node state, source `0` reaching target `1` over `n` copies of the same edge —
    /// large enough that scanning every one of them is the cost these tests exist to bound.
    fn two_node_state_with_edges(n: usize, prop: u32) -> State {
        let mut st = State {
            nodes: NodeVec::default(),
            edges: PVec::default(),
            adjacency: PVec::default(),
            edges_seen: 0,
            deferred: Vec::new(),
            closures: RefCell::default(),
            blocking: Blocking::default(),
            children: PVec::default(),
            open: NodeSet::default(),
            merged_in: PVec::default(),
            root_of: std::rc::Rc::default(),
            generated_root_of: std::rc::Rc::default(),
            clash: false,
            clique_exhausted: std::cell::Cell::new(false),
            support: crate::owl_dl::support::Cursor::Disabled,
        };
        st.nodes.push(bare_node(true));
        st.nodes.push(bare_node(true));
        for _ in 0..n {
            st.push_edge(0, 1, prop);
        }
        st
    }

    /// The neighbourhood read as it was written before the stamped scratch: per-read
    /// `BTreeSet`s and vectors, the same step order, the same charges. Kept here only to be
    /// compared against.
    fn reference_neighbors(g: &Graph<'_>, st: &State, x: usize, role: Role) -> Vec<usize> {
        fn step(
            g: &Graph<'_>,
            st: &State,
            x: usize,
            ach: &[(u32, bool)],
            seen: &mut BTreeSet<usize>,
            out: &mut Vec<usize>,
        ) {
            let x = find(st, x);
            g.work.charge(st.class_edges(x).len() as u64 + 1);
            if g.work.exhausted() {
                return;
            }
            for &edge in st.class_edges(x) {
                let (from, to, prop) = st.edges[edge];
                let (f, t) = (find(st, from), find(st, to));
                if realizes(ach, (prop, true)) && f == x && seen.insert(t) {
                    out.push(t);
                }
                if realizes(ach, (prop, false)) && t == x && seen.insert(f) {
                    out.push(f);
                }
            }
        }
        if g.work.exhausted() {
            return Vec::new();
        }
        let ach = g.achievers(role);
        let x = find(st, x);
        let mut out = Vec::new();
        let mut seen = BTreeSet::new();
        step(g, st, x, &ach, &mut seen, &mut out);
        for &(prop, dir) in ach.iter() {
            if !g.kb.transitive.contains(&prop) || g.work.exhausted() {
                continue;
            }
            let single = g.achievers(pattern_role(prop, dir));
            let mut frontier = Vec::new();
            step(g, st, x, &single, &mut BTreeSet::new(), &mut frontier);
            let mut visited: BTreeSet<usize> = frontier.iter().copied().collect();
            while let Some(y) = frontier.pop() {
                if g.work.exhausted() {
                    return out;
                }
                if seen.insert(y) {
                    out.push(y);
                }
                let mut next = Vec::new();
                step(g, st, y, &single, &mut BTreeSet::new(), &mut next);
                for z in next {
                    if visited.insert(z) {
                        frontier.push(z);
                    }
                }
            }
        }
        out
    }

    /// A random completion graph over `nodes` nodes and three properties, some of them merged,
    /// beside a random role hierarchy, inverse declarations and transitive set.
    fn random_graph(draw: &mut purrdf_testkit::rng::SplitMix64) -> (Kb, State) {
        let mut kb = Kb::empty();
        for p in 0..3_u32 {
            if draw.below(3) == 0 {
                kb.transitive.insert(p);
            }
            if draw.below(3) == 0 {
                let sub = u32::try_from(draw.below(3)).expect("below 3");
                kb.role_sub.entry(p).or_default().insert(sub);
            }
        }
        if draw.below(2) == 0 {
            let (a, b) = (
                u32::try_from(draw.below(3)).expect("below 3"),
                u32::try_from(draw.below(3)).expect("below 3"),
            );
            kb.inverses.entry(a).or_default().insert(b);
            kb.inverses.entry(b).or_default().insert(a);
        }
        let nodes = 2 + draw.below_usize(11);
        let mut st = two_node_state_with_edges(0, 0);
        for _ in 2..nodes {
            st.nodes.push(bare_node(false));
        }
        for _ in 0..draw.below_usize(3 * nodes) {
            let (from, to) = (draw.below_usize(nodes), draw.below_usize(nodes));
            st.push_edge(from, to, u32::try_from(draw.below(3)).expect("below 3"));
        }
        // A merge forwards a node through `find` and folds its edges into the keeper's.
        for _ in 0..draw.below_usize(3) {
            let (keep, discard) = (
                find(&st, draw.below_usize(nodes)),
                find(&st, draw.below_usize(nodes)),
            );
            if keep != discard {
                st.nodes[discard].merged = Some(keep);
                st.merge_adjacency(keep, discard);
            }
        }
        (kb, st)
    }

    /// `st` as it was before any read cached a closure.
    fn uncached(st: &State) -> State {
        let mut fresh = st.clone();
        fresh.closures = RefCell::default();
        fresh
    }

    /// THE STAMPED READ IS THE READ IT REPLACED: over random graphs, role hierarchies,
    /// inverses, transitive sets and merges, [`Graph::neighbors`] on a state that has cached
    /// nothing returns the reference read's neighbours in the reference read's ORDER and
    /// charges exactly its work — the order is what every match, branch point and ledger
    /// downstream depends on. And read AGAIN on a state that cached the first read's closures,
    /// it returns the same neighbours in the same order and charges no more.
    #[test]
    fn the_stamped_neighbourhood_read_matches_the_reference_read() {
        let mut draw = purrdf_testkit::rng::SplitMix64::new(0x00C0_FFEE);
        let mut transitive_reads = 0_u32;
        for case in 0..3_000 {
            let (kb, st) = random_graph(&mut draw);
            let reference = Graph::new(&kb, u64::MAX);
            let membership = Graph::new(&kb, u64::MAX);
            let caching = Graph::new(&kb, u64::MAX);
            let walked = Graph::new(&kb, u64::MAX);
            for x in 0..st.nodes.len() {
                for p in 0..3_u32 {
                    for role in [Role::Named(p), Role::Inv(p)] {
                        let before = (reference.work.spent(), walked.work.spent());
                        let expected = reference_neighbors(&reference, &st, x, role);
                        let got = walked.neighbors(&uncached(&st), x, role).to_vec();
                        assert_eq!(got, expected, "case {case}, node {x}, {role:?}");
                        assert_eq!(
                            walked.work.spent() - before.1,
                            reference.work.spent() - before.0,
                            "case {case}, node {x}, {role:?}: the charge moved"
                        );
                        for &y in &expected {
                            assert!(membership.is_neighbour(&st, x, role, y));
                        }
                        let first = caching.work.spent();
                        let cached_once = caching.neighbors(&st, x, role).to_vec();
                        let once = caching.work.spent() - first;
                        let cached_twice = caching.neighbors(&st, x, role).to_vec();
                        let twice = caching.work.spent() - first - once;
                        assert_eq!(cached_once, expected, "case {case}, node {x}, {role:?}");
                        assert_eq!(cached_twice, expected, "case {case}, node {x}, {role:?}");
                        assert!(twice <= once, "case {case}: a cached read charged more");
                        transitive_reads += u32::from(kb.transitive.contains(&p));
                    }
                }
            }
        }
        assert!(transitive_reads > 10_000, "{transitive_reads}");
    }

    /// THE CACHED CLOSURES FOLLOW THE GRAPH: reads interleaved with appended edges and merges
    /// return, from the cache, the neighbour SET a fresh walk of the graph as it now stands
    /// returns, each neighbour once.
    #[test]
    fn cached_closures_follow_appended_edges_and_merges() {
        let mut draw = purrdf_testkit::rng::SplitMix64::new(0x00D1_5EA5);
        let mut cached_reads = 0_u32;
        for case in 0..1_500 {
            let (kb, mut st) = random_graph(&mut draw);
            let g = Graph::new(&kb, u64::MAX);
            for step in 0..40 {
                let nodes = st.nodes.len();
                match draw.below(10) {
                    0..=5 => {
                        let x = draw.below_usize(nodes);
                        let p = u32::try_from(draw.below(3)).expect("below 3");
                        let role = if draw.below(2) == 0 {
                            Role::Named(p)
                        } else {
                            Role::Inv(p)
                        };
                        let mut got = g.neighbors(&st, x, role).to_vec();
                        let mut expected = reference_neighbors(
                            &Graph::new(&kb, u64::MAX),
                            &uncached(&st),
                            x,
                            role,
                        );
                        got.sort_unstable();
                        expected.sort_unstable();
                        let mut unique = got.clone();
                        unique.dedup();
                        assert_eq!(got, unique, "case {case} step {step}: a neighbour twice");
                        assert_eq!(got, expected, "case {case} step {step}, node {x}, {role:?}");
                        cached_reads += u32::from(st.closures.borrow().cached > 0);
                    }
                    6..=8 => {
                        let (from, to) = (draw.below_usize(nodes), draw.below_usize(nodes));
                        st.push_edge(from, to, u32::try_from(draw.below(3)).expect("below 3"));
                    }
                    _ => {
                        let (a, b) = (draw.below_usize(nodes), draw.below_usize(nodes));
                        g.merge_nodes(&mut st, a, b);
                    }
                }
            }
        }
        assert!(cached_reads > 10_000, "{cached_reads}");
    }

    /// A read that stops at its first witness charges no more than the full read, and — the
    /// neighbouring case — one that finds nothing charges exactly the full read.
    #[test]
    fn an_early_stopping_read_never_charges_more_than_the_full_read() {
        let mut draw = purrdf_testkit::rng::SplitMix64::new(0x0BAD_CAFE);
        for case in 0..1_000 {
            let (kb, st) = random_graph(&mut draw);
            for x in 0..st.nodes.len() {
                let role = Role::Named(0);
                let full = Graph::new(&kb, u64::MAX);
                let all = full.neighbors(&uncached(&st), x, role).len();
                let early = Graph::new(&kb, u64::MAX);
                let found = early.any_neighbour(&uncached(&st), x, role, &mut |_| true);
                assert_eq!(found, all > 0, "case {case}");
                assert!(early.work.spent() <= full.work.spent(), "case {case}");
                let none = Graph::new(&kb, u64::MAX);
                assert!(!none.any_neighbour(&uncached(&st), x, role, &mut |_| false));
                assert_eq!(none.work.spent(), full.work.spent(), "case {case}");
            }
        }
    }

    /// NEIGHBOURHOOD READS ALLOCATE NOTHING once the graph's scratch has grown to the graph:
    /// a transitive closure over a sub-role and an inverse partner, read every way the
    /// calculus reads one — into a caller's buffer, through a pooled buffer, as a membership
    /// test and as an early-stopping search — counted by the workspace's allocator.
    #[test]
    fn neighbourhood_reads_allocate_nothing_in_steady_state() {
        const R: u32 = 1;
        const S: u32 = 2;
        const T: u32 = 3;
        let mut kb = Kb::empty();
        kb.transitive.insert(R);
        kb.role_sub.entry(R).or_default().insert(S);
        kb.inverses.entry(R).or_default().insert(T);
        kb.inverses.entry(T).or_default().insert(R);
        let mut st = two_node_state_with_edges(0, R);
        for _ in 0..64 {
            st.nodes.push(bare_node(false));
        }
        for x in 0..65 {
            match x % 3 {
                0 => st.push_edge(x, x + 1, R),
                1 => st.push_edge(x, x + 1, S),
                _ => st.push_edge(x + 1, x, T),
            }
        }
        let g = Graph::new(&kb, u64::MAX);
        let mut out = Vec::new();
        let read = |out: &mut Vec<usize>| {
            let mut found = 0;
            for x in 0..st.nodes.len() {
                for role in [Role::Named(R), Role::Inv(R), Role::Named(S), Role::Inv(T)] {
                    g.neighbors_into(&st, x, role, out);
                    found += out.len();
                    let pooled = g.neighbors(&st, x, role);
                    found += pooled.len();
                    drop(pooled);
                    found += usize::from(g.is_neighbour(&st, x, role, 0));
                    found += usize::from(g.any_neighbour(&st, x, role, &mut |y| y > x));
                }
            }
            found
        };
        // Warm-up: the stamps grow to the graph, the closures are cached, the pool fills.
        let warm = read(&mut out);
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        let again = read(&mut out);
        let measured = window.close();
        assert_eq!(warm, again);
        assert!(warm > 4_000, "the fixture reads long closures: {warm}");
        assert_eq!(
            measured.allocations, 0,
            "a neighbourhood read allocated in steady state: {measured:?}"
        );
    }

    // --- FB-1: `max_clique`/`Clique::extend` poll the shared meter DURING the search -----------

    /// [`Clique::extend`]'s CANDIDATE loop — not only its recursive calls — must stop the moment
    /// the shared meter is exhausted, because a candidate set most of which is pairwise
    /// INCOMPATIBLE never recurses past depth one: the whole cost is one call's `for` loop
    /// over the remaining candidates, calling `compat` once each. Before this fix that loop
    /// carried no charge and no poll at all, so neither the shared meter nor the search's
    /// own [`MAX_CLIQUE_WORK`] ceiling ever saw it — a search over a multi-million item slice
    /// would run to completion under a cap of ONE.
    #[test]
    fn max_clique_stops_a_wide_incompatible_scan_when_the_meter_is_narrow() {
        let items: Vec<usize> = (0..2_000_000).collect();
        let compat_calls = std::cell::Cell::new(0u64);
        // Every pair incompatible: `current` never grows past depth one, so the whole search
        // is one call's linear scan over `items` — exactly the shape a per-call-only poll
        // cannot see.
        let compat = |_a: usize, _b: usize| {
            compat_calls.set(compat_calls.get() + 1);
            false
        };
        let meter = Work::new(50);

        let result = max_clique(&items, &compat, &meter);

        assert!(
            result.is_none(),
            "a meter exhausted mid-search must never be read as a decided maximum clique"
        );
        assert!(meter.exhausted());
        assert_eq!(
            meter.spent(),
            50,
            "the meter clamps exactly at its cap, whatever the search still had left to do"
        );
        assert!(
            compat_calls.get() < 200,
            "a narrow meter must cut the candidate scan off near its cap ({}), not after \
             walking all {} items compat was given: {} calls",
            meter.spent(),
            items.len(),
            compat_calls.get()
        );
    }

    /// The comparison the test above needs to be evidence rather than an accident: the SAME
    /// fixture under a cap ample for the whole scan lets `compat` see (almost) every item, so
    /// the narrow-cap test's small count is the meter cutting the search off — not a fixture
    /// that never drove `compat` past a handful of calls regardless of the cap.
    #[test]
    fn max_clique_runs_the_whole_wide_scan_when_the_meter_is_ample() {
        // Every pair incompatible is the ADVERSARIAL mixed-`≠`-graph shape the module docs
        // name: with nothing ever compatible, `current` empties on every backtrack and the
        // search re-tries every suffix from every starting index, which is quadratic in
        // `items.len()` (about `n²/2` candidate charges) rather than linear. Sized so that
        // quadratic cost stays comfortably under [`MAX_CLIQUE_WORK`] — the search's own local
        // ceiling, independent of the meter — so what stops this run is the meter cap chosen
        // below finishing the search, not that unrelated one.
        let items: Vec<usize> = (0..1_000).collect();
        let compat_calls = std::cell::Cell::new(0u64);
        let compat = |_a: usize, _b: usize| {
            compat_calls.set(compat_calls.get() + 1);
            false
        };
        let meter = Work::new(1_000_000);

        let result = max_clique(&items, &compat, &meter);

        assert_eq!(
            result,
            Some(vec![0]),
            "the first candidate is the whole clique"
        );
        assert!(!meter.exhausted());
        assert!(
            compat_calls.get() as usize >= items.len() - 1,
            "an ample meter must let the scan reach (almost) every one of the {} items: only \
             {} compat calls",
            items.len(),
            compat_calls.get()
        );
        assert!(
            compat_calls.get() > 10 * items.len() as u64,
            "the quadratic shape this fixture is FOR: far more compat calls than items, \
             because every backtrack re-scans a suffix: {} calls over {} items",
            compat_calls.get(),
            items.len()
        );
    }

    /// Two runs of the SAME exhausting search agree exactly — on the verdict (`None`), on the
    /// meter's own reading, and on how many candidates it got through — because every charge
    /// [`Clique::extend`] makes is a pure function of `items` and `compat`, never of a clock or a
    /// hash iteration order.
    #[test]
    fn max_clique_exhaustion_is_deterministic_run_to_run() {
        let items: Vec<usize> = (0..500_000).collect();
        let compat = |_a: usize, _b: usize| false;
        let meter_a = Work::new(37);
        let meter_b = Work::new(37);

        let first = max_clique(&items, &compat, &meter_a);
        let again = max_clique(&items, &compat, &meter_b);

        assert!(first.is_none() && again.is_none());
        assert_eq!(
            meter_a.spent(),
            meter_b.spent(),
            "two runs, one work figure"
        );
        assert_eq!(
            meter_a.spent(),
            37,
            "the meter clamps at its cap on every run"
        );
    }

    // --- FB-1: `Graph::neighbors`'s edge scan stops when its own bulk charge exhausts the
    // meter --------------------------------------------------------------------------------

    /// A neighbourhood step charges the edges it is about to read up front, in one bulk charge,
    /// so a NARROW cap sees the true cost of the walk it is about to refuse before that walk
    /// runs even one comparison. Without the check right after that charge, the loop would
    /// walk every one of the class's edges regardless (here, all two million: both nodes touch
    /// every edge) — which is exactly the gap this test pins shut: a cap far smaller than the
    /// edge count must come back with NO neighbours rather than the true one, because it never
    /// got to look.
    #[test]
    fn neighbors_stops_the_edge_scan_when_the_bulk_charge_exhausts_the_meter() {
        const PROP: u32 = 7;
        let kb = Kb::empty();
        let st = two_node_state_with_edges(2_000_000, PROP);

        let narrow = Graph::new(&kb, 5);
        let truncated = narrow.neighbors(&st, 0, Role::Named(PROP));
        assert!(
            truncated.is_empty(),
            "a cap of 5 against two million edges must see none of them: {:?}",
            *truncated
        );
        assert!(narrow.work().exhausted());
        assert_eq!(
            narrow.work().spent(),
            5,
            "the meter clamps exactly at its cap"
        );

        // The control: the identical graph under a cap ample for the whole scan finds the
        // one true neighbour, so the truncation above is the meter's doing and not a fixture
        // that could never have found it.
        let ample = Graph::new(&kb, 10_000_000);
        let full = ample.neighbors(&st, 0, Role::Named(PROP));
        assert_eq!(
            *full,
            vec![1],
            "the true neighbour, once the scan is allowed to run"
        );
    }

    // --- FB-1: `Graph::achievers`'s role-closure walk polls per expansion, and never caches a
    // truncated closure ----------------------------------------------------------------------

    /// A role hierarchy that chains `n` super/sub-property links closes over `n` expansions.
    /// [`Graph::achievers`] used to charge and check that cost only ONCE, after the whole
    /// stack-walk finished — so a cap far smaller than the chain would still walk the whole
    /// chain before reporting itself exhausted, and it would then MEMOIZE the (complete, in
    /// that old code) closure. This test pins the fixed behaviour: the walk stops near the
    /// cap, and — because what it stopped with is a PARTIAL closure — the cache must not hold
    /// it, or a later call in the same search would silently read fewer achievers than the
    /// role hierarchy actually has.
    #[test]
    fn achievers_polls_the_role_closure_walk_and_never_caches_a_truncated_one() {
        const N: u32 = 200_000;
        let mut kb = Kb::empty();
        for i in 0..N {
            kb.role_sub.entry(i).or_default().insert(i + 1);
        }
        let role = Role::Named(0);

        let narrow = Graph::new(&kb, 10);
        let closure = narrow.achievers(role);
        assert!(narrow.work().exhausted());
        assert!(
            closure.len() < 40,
            "a cap of 10 against a {N}-link chain must stop near it, not after closing the \
             whole chain: {} achievers",
            closure.len()
        );
        assert!(
            narrow.achiever_cache.borrow().is_empty(),
            "a closure the walk could not finish must never be memoized"
        );

        // The control: an ample cap closes the whole chain (N + 1 roles: 0..=N, each in the
        // forward direction) and DOES cache it.
        let ample = Graph::new(&kb, 10_000_000);
        let full = ample.achievers(role);
        assert_eq!(
            full.len(),
            (N + 1) as usize,
            "the whole chain, once the walk is allowed to finish"
        );
        assert!(!ample.work().exhausted());
        assert!(ample.achiever_cache.borrow().contains_key(role));
    }

    // --- The per-node edge index agrees with the full edge scan ----------------------------

    /// What a neighbourhood step once read off the whole edge vector for the root `x`: the indices
    /// of every edge with an endpoint resolving to `x`, in ascending order.
    fn full_scan(st: &State, x: usize) -> Vec<usize> {
        (0..st.edges.len())
            .filter(|&e| {
                let (from, to, _) = st.edges[e];
                find(st, from) == x || find(st, to) == x
            })
            .collect()
    }

    /// Every root's indexed edge list equals the full-scan filter, and every forwarded node's
    /// list is empty, after random interleavings of node creation, edge pushes (from forwarded
    /// endpoints too, and self-loops) and merges in either direction.
    ///
    /// A neighbourhood step walks [`State::class_edges`] instead of the edge vector, so the two
    /// agreeing on every root at every point of a search is exactly what keeps every
    /// neighbourhood, verdict and proof unchanged. Each step is checked, not only the end, so
    /// a merge that lost, duplicated or reordered an edge is caught at the merge that did it.
    #[test]
    fn the_adjacency_index_equals_the_full_edge_scan_under_random_pushes_and_merges() {
        use purrdf_testkit::rng::SplitMix64;
        let mut merges = 0usize;
        let mut shared = 0usize;
        for seed in 0..400u64 {
            let mut rng = SplitMix64::new(0x0442_ad1a_c000 ^ seed);
            let mut st = two_node_state_with_edges(0, 0);
            for _ in 0..rng.below_usize(4) {
                st.nodes.push(bare_node(false));
            }
            for _ in 0..60 {
                match rng.below(5) {
                    0 => st.nodes.push(bare_node(false)),
                    1 | 2 => {
                        let from = rng.below_usize(st.nodes.len());
                        let to = rng.below_usize(st.nodes.len());
                        let prop = u32::try_from(rng.below(3)).expect("below 3");
                        st.push_edge(from, to, prop);
                    }
                    _ => {
                        let a = find(&st, rng.below_usize(st.nodes.len()));
                        let b = find(&st, rng.below_usize(st.nodes.len()));
                        if a != b {
                            // An edge between the two classes is listed under both roots, so
                            // the fold must keep it once.
                            shared += full_scan(&st, a)
                                .iter()
                                .filter(|e| full_scan(&st, b).contains(e))
                                .count();
                            st.forward(a, b);
                            merges += 1;
                        }
                    }
                }
                for x in 0..st.nodes.len() {
                    if st.nodes[x].merged.is_some() {
                        assert!(
                            st.class_edges(x).is_empty(),
                            "seed {seed}: forwarded node {x} still lists edges"
                        );
                    } else {
                        assert_eq!(
                            st.class_edges(x),
                            full_scan(&st, x).as_slice(),
                            "seed {seed}: root {x}'s indexed edges differ from the full scan"
                        );
                    }
                }
            }
        }
        // The corpus actually exercised what it exists to check.
        assert!(merges > 4_000, "only {merges} merges were generated");
        assert!(
            shared > 1_000,
            "only {shared} edges were shared across a merge"
        );
    }
}
