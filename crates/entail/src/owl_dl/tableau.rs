// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! An `ALCOIQ` tableau consistency procedure (the OWL-Direct decision core).
//!
//! This is a from-scratch implementation of the standard completion-graph algorithm
//! (Horrocks & Sattler, "A Tableau Decision Procedure for SHOIQ", 2007; Baader et
//! al., "The Description Logic Handbook", ch. 3) restricted to the `ALCOIQ` fragment
//! the OWL-Direct fixtures exercise: the boolean connectives, existential/universal
//! restrictions, qualified number restrictions (`Q`), inverse roles (`I`), and
//! nominals (`O`). Algorithms are not copyrightable; this code is original.
//!
//! ## Shape of the search
//!
//! A [`State`] is a completion graph: nodes carry a `BTreeSet` label of concept ids,
//! directed role edges connect them, and a `≠` (distinctness) relation records forced
//! inequalities. The TBox is *internalized*: every general concept inclusion `C ⊑ D`
//! becomes the meta-concept `nnf(¬C ⊔ D)`, and the union of all such meta-concepts is
//! placed in every node's label at creation. [`Tableau::solve`] runs the deterministic
//! completion rules to a fixpoint, then branches (depth-first, in a fully deterministic
//! order) on the non-deterministic rules (`⊔`, the `≤`-choose rule, and `≤`-merges),
//! cloning the state per branch. A branch that reaches a clash-free fixpoint witnesses
//! consistency.
//!
//! ## Termination
//!
//! Tree nodes are subject to **pairwise (double) blocking** — the sound blocking
//! discipline for logics with inverse roles: a tree node is blocked by an ancestor
//! when their labels *and* their predecessors' labels *and* the connecting edge roles
//! all coincide. Nominal/root nodes (one per named individual) are never blocked. A
//! generous per-run step cap is a hard backstop: a termination bug surfaces as an
//! [`EntailError::Build`] rather than a hang.

use std::collections::{BTreeMap, BTreeSet};

use crate::owl_dl::concept::{Decomp, Role};
use crate::owl_dl::Kb;
use crate::EntailError;

/// A single completion-graph node.
#[derive(Clone)]
struct Node {
    /// The concept-id label set (ordered; drives no result via hash iteration).
    label: BTreeSet<u32>,
    /// The generating predecessor (tree parent); `None` for root/nominal nodes.
    parent: Option<usize>,
    /// The role `(property, inverted)` on the edge from `parent` to this node.
    incoming: Option<(u32, bool)>,
    /// Whether this is a root (named-individual / nominal) node — never blocked.
    root: bool,
    /// The individual term id this root stands for, if any.
    nominal: Option<u32>,
    /// Nodes this node is forced to be distinct from (`≠`), by node index.
    neq: BTreeSet<usize>,
    /// Union-find forward pointer once merged away (`None` while a representative).
    merged: Option<usize>,
}

/// A completion graph under construction.
#[derive(Clone)]
struct State {
    /// All nodes ever created (merged-away ones remain, forwarded via `merged`).
    nodes: Vec<Node>,
    /// Directed role edges `(from, to, property)`; endpoints resolved via `find`.
    edges: Vec<(usize, usize, u32)>,
    /// Named individual term id → its root node index.
    root_of: BTreeMap<u32, usize>,
    /// A clash has been detected (e.g. a forced `≠` merge).
    clash: bool,
}

/// A non-deterministic expansion alternative.
#[derive(Clone)]
enum Branch {
    /// Add a concept id to a node's label.
    AddConcept(usize, u32),
    /// Merge two nodes (identify them).
    Merge(usize, usize),
}

/// The tableau driver: read-only knowledge base, the internalized TBox, a step cap.
struct Tableau<'a> {
    /// The knowledge base (concept table, role hierarchy, inverses).
    kb: &'a Kb,
    /// The internalized TBox: meta-concept ids placed in every node's label.
    meta: BTreeSet<u32>,
    /// Steps consumed so far.
    steps: u64,
    /// Hard step cap; exceeding it is a hard error (a termination-bug backstop).
    cap: u64,
}

/// Decide whether the knowledge base (optionally with `extra` typed individuals and a
/// single `fresh` root carrying `fresh_types`) has a consistent completion.
///
/// `include_abox` pulls in the ABox (individual roots, role edges, `owl:sameAs`
/// merges); subsumption checks pass `false` to reason purely over the TBox.
///
/// # Errors
///
/// [`EntailError::Build`] if the step cap is exceeded (a termination-bug backstop).
pub(crate) fn consistent(
    kb: &Kb,
    include_abox: bool,
    extra: &[(u32, u32)],
    fresh_types: &[u32],
) -> Result<bool, EntailError> {
    let mut t = Tableau::new(kb);
    let st = t.init_state(include_abox, extra, fresh_types);
    t.solve(st)
}

/// Resolve a node index to its union-find representative.
fn find(st: &State, mut x: usize) -> usize {
    while let Some(n) = st.nodes[x].merged {
        x = n;
    }
    x
}

/// Whether `a` and `b` are forced distinct (`a ≠ b`), resolving representatives.
fn are_distinct(st: &State, a: usize, b: usize) -> bool {
    let a = find(st, a);
    let b = find(st, b);
    if a == b {
        return false;
    }
    st.nodes[a].neq.iter().any(|&w| find(st, w) == b)
        || st.nodes[b].neq.iter().any(|&w| find(st, w) == a)
}

/// Record `a ≠ b`.
fn set_distinct(st: &mut State, a: usize, b: usize) {
    let a = find(st, a);
    let b = find(st, b);
    if a == b {
        st.clash = true;
        return;
    }
    st.nodes[a].neq.insert(b);
    st.nodes[b].neq.insert(a);
}

/// Merge `discard` into `keep`, identifying the two nodes.
///
/// Orientation keeps a root over a tree node, else the lower index. A forced merge of
/// a `≠` pair sets [`State::clash`].
fn merge(st: &mut State, keep: usize, discard: usize) {
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
    // Carry the nominal identity onto the keeper; repoint the root map.
    if let Some(a) = st.nodes[discard].nominal {
        if st.nodes[keep].nominal.is_none() {
            st.nodes[keep].nominal = Some(a);
        }
        st.root_of.insert(a, keep);
    }
    if st.nodes[discard].root {
        st.nodes[keep].root = true;
    }
    st.nodes[discard].merged = Some(keep);
}

impl<'a> Tableau<'a> {
    /// Build a driver over `kb`, snapshotting the internalized TBox.
    fn new(kb: &'a Kb) -> Self {
        let meta: BTreeSet<u32> = kb.meta.iter().copied().collect();
        // A generous, size-proportional cap: pairwise blocking bounds the real work
        // far below this, so hitting it means a bug, not a hard instance.
        let base =
            (kb.abox_types.len() + kb.abox_roles.len() + kb.tbox.len() + kb.individuals.len() + 16)
                as u64;
        Self {
            kb,
            meta,
            steps: 0,
            cap: 100_000 + base.saturating_mul(base).saturating_mul(64),
        }
    }

    /// A fresh label seeded with the internalized TBox.
    fn seed_label(&self) -> BTreeSet<u32> {
        self.meta.clone()
    }

    /// Build the initial completion graph.
    fn init_state(&self, include_abox: bool, extra: &[(u32, u32)], fresh_types: &[u32]) -> State {
        let mut st = State {
            nodes: Vec::new(),
            edges: Vec::new(),
            root_of: BTreeMap::new(),
            clash: false,
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
                st.edges.push((ra, rb, p));
            }
            for &(a, b) in &self.kb.same_as {
                let ra = self.root(&mut st, a);
                let rb = self.root(&mut st, b);
                merge(&mut st, ra, rb);
            }
        }
        for &(a, c) in extra {
            let ra = self.root(&mut st, a);
            st.nodes[ra].label.insert(c);
        }
        if !fresh_types.is_empty() {
            let mut label = self.seed_label();
            label.extend(fresh_types.iter().copied());
            st.nodes.push(Node {
                label,
                parent: None,
                incoming: None,
                root: true,
                nominal: None,
                neq: BTreeSet::new(),
                merged: None,
            });
        }
        st
    }

    /// Get or create the root node for individual term id `a`.
    fn root(&self, st: &mut State, a: u32) -> usize {
        if let Some(&n) = st.root_of.get(&a) {
            return find(st, n);
        }
        let idx = st.nodes.len();
        st.nodes.push(Node {
            label: self.seed_label(),
            parent: None,
            incoming: None,
            root: true,
            nominal: Some(a),
            neq: BTreeSet::new(),
            merged: None,
        });
        st.root_of.insert(a, idx);
        idx
    }

    /// The depth-first, deterministic search: saturate, then branch.
    fn solve(&mut self, mut st: State) -> Result<bool, EntailError> {
        if !self.saturate(&mut st)? {
            return Ok(false);
        }
        if let Some(branches) = self.find_branch(&st) {
            for br in branches {
                let mut s2 = st.clone();
                if self.apply_branch(&mut s2, &br) && self.solve(s2)? {
                    return Ok(true);
                }
            }
            return Ok(false);
        }
        Ok(true)
    }

    /// Apply the deterministic completion rules to a fixpoint.
    ///
    /// Returns `Ok(false)` on a clash, `Ok(true)` at a clash-free fixpoint.
    fn saturate(&mut self, st: &mut State) -> Result<bool, EntailError> {
        loop {
            self.tick()?;
            self.detect_clash(st);
            if st.clash {
                return Ok(false);
            }
            let changed = self.apply_deterministic(st);
            if st.clash {
                return Ok(false);
            }
            if !changed {
                return Ok(true);
            }
        }
    }

    /// Consume one step against the cap.
    fn tick(&mut self) -> Result<(), EntailError> {
        self.steps += 1;
        if self.steps > self.cap {
            return Err(EntailError::Build(
                "OWL-Direct tableau exceeded its step cap (possible non-termination)".to_owned(),
            ));
        }
        Ok(())
    }

    /// Structural clash detection over the current representatives.
    fn detect_clash(&self, st: &mut State) {
        let bottom = self.kb.bottom;
        let n = st.nodes.len();
        for i in 0..n {
            if find(st, i) != i {
                continue;
            }
            let nominal = st.nodes[i].nominal;
            // Immutable scan of the label for atomic / nominal clashes.
            let label = &st.nodes[i].label;
            if label.contains(&bottom) {
                st.clash = true;
                return;
            }
            for &cid in label {
                if label.contains(&self.kb.table.negate(cid)) {
                    st.clash = true;
                    return;
                }
                match self.kb.table.decomp(cid) {
                    Decomp::NegNominal(w) => {
                        if let Some(a) = nominal {
                            if w.binary_search(&a).is_ok() {
                                st.clash = true;
                                return;
                            }
                        }
                    }
                    Decomp::Nominal(v) => {
                        if let Some(a) = nominal {
                            if v.binary_search(&a).is_err() {
                                st.clash = true;
                                return;
                            }
                        }
                    }
                    _ => {}
                }
            }
            // A `≤n r.C` violated by more than `n` pairwise-distinct C-neighbours.
            if self.max_clash(st, i) {
                st.clash = true;
                return;
            }
        }
    }

    /// Whether some `≤n r.C` on node `x` is violated by `> n` pairwise-`≠` neighbours.
    fn max_clash(&self, st: &State, x: usize) -> bool {
        let cids: Vec<u32> = st.nodes[x].label.iter().copied().collect();
        for cid in cids {
            if let Decomp::Max(n, role, c) = *self.kb.table.decomp(cid) {
                let filler = c;
                let neigh = self.neighbors(st, x, role);
                let with_c: Vec<usize> = neigh
                    .into_iter()
                    .filter(|&y| self.has_concept(st, y, filler))
                    .collect();
                let clique = max_clique(&with_c, &|a, b| are_distinct(st, a, b));
                if clique.len() > n as usize {
                    return true;
                }
            }
        }
        false
    }

    /// Apply every deterministic rule once across all representative nodes.
    fn apply_deterministic(&self, st: &mut State) -> bool {
        let mut changed = false;
        let n = st.nodes.len();
        for i in 0..n {
            if find(st, i) != i {
                continue;
            }
            changed |= self.rule_unfold(st, i);
            changed |= self.rule_and(st, i);
            changed |= self.rule_all(st, i);
            changed |= self.rule_nominal(st, i);
            if !self.blocked(st, i) {
                changed |= self.rule_exists(st, i);
                changed |= self.rule_min(st, i);
            }
            if st.clash {
                return changed;
            }
        }
        changed
    }

    /// Absorption (lazy-unfolding) rule: a named class `A ∈ L(x)` adds every `D` with
    /// an absorbed GCI `A ⊑ D`. This replaces branching a `¬A ⊔ D` disjunction on every
    /// node with a deterministic add triggered only where `A` actually holds.
    fn rule_unfold(&self, st: &mut State, x: usize) -> bool {
        let mut adds: Vec<u32> = Vec::new();
        for &cid in &st.nodes[x].label {
            if let Some(sups) = self.kb.unfold.get(&cid) {
                for &s in sups {
                    if !st.nodes[x].label.contains(&s) {
                        adds.push(s);
                    }
                }
            }
        }
        let changed = !adds.is_empty();
        st.nodes[x].label.extend(adds);
        changed
    }

    /// `⊓`-rule: `C₁ ⊓ … ⊓ Cₙ ∈ L(x)` adds each `Cᵢ`.
    fn rule_and(&self, st: &mut State, x: usize) -> bool {
        let mut adds: Vec<u32> = Vec::new();
        for &cid in &st.nodes[x].label {
            if let Decomp::And(cs) = self.kb.table.decomp(cid) {
                for &c in cs {
                    if !st.nodes[x].label.contains(&c) {
                        adds.push(c);
                    }
                }
            }
        }
        let changed = !adds.is_empty();
        st.nodes[x].label.extend(adds);
        changed
    }

    /// `∀`-rule: `∀r.C ∈ L(x)` adds `C` to every `r`-neighbour of `x`.
    fn rule_all(&self, st: &mut State, x: usize) -> bool {
        let alls: Vec<(Role, u32)> = st.nodes[x]
            .label
            .iter()
            .filter_map(|&cid| match *self.kb.table.decomp(cid) {
                Decomp::All(role, c) => Some((role, c)),
                _ => None,
            })
            .collect();
        let mut changed = false;
        for (role, c) in alls {
            for y in self.neighbors(st, x, role) {
                changed |= self.add_concept(st, y, c);
            }
        }
        changed
    }

    /// `∃`-rule: `∃r.C ∈ L(x)` with no `r`-neighbour satisfying `C` creates one.
    fn rule_exists(&self, st: &mut State, x: usize) -> bool {
        let somes: Vec<(Role, u32)> = st.nodes[x]
            .label
            .iter()
            .filter_map(|&cid| match *self.kb.table.decomp(cid) {
                Decomp::Some(role, c) => Some((role, c)),
                _ => None,
            })
            .collect();
        let mut changed = false;
        for (role, c) in somes {
            let has = self
                .neighbors(st, x, role)
                .into_iter()
                .any(|y| self.has_concept(st, y, c));
            if !has {
                self.new_successor(st, x, role, &[c]);
                changed = true;
            }
        }
        changed
    }

    /// `≥`-rule: `≥n r.C ∈ L(x)` ensures `n` pairwise-`≠` `r`-neighbours with `C`.
    fn rule_min(&self, st: &mut State, x: usize) -> bool {
        let mins: Vec<(u32, Role, u32)> = st.nodes[x]
            .label
            .iter()
            .filter_map(|&cid| match *self.kb.table.decomp(cid) {
                Decomp::Min(n, role, c) => Some((n, role, c)),
                _ => None,
            })
            .collect();
        let mut changed = false;
        for (n, role, c) in mins {
            let n = n as usize;
            if n == 0 {
                continue;
            }
            let with_c: Vec<usize> = self
                .neighbors(st, x, role)
                .into_iter()
                .filter(|&y| self.has_concept(st, y, c))
                .collect();
            let mut clique = max_clique(&with_c, &|a, b| are_distinct(st, a, b));
            if clique.len() >= n {
                continue;
            }
            while clique.len() < n {
                let y = self.new_successor(st, x, role, &[c]);
                clique.push(y);
            }
            // Force the whole witness set pairwise distinct.
            for a in 0..clique.len() {
                for b in (a + 1)..clique.len() {
                    set_distinct(st, clique[a], clique[b]);
                }
            }
            changed = true;
        }
        changed
    }

    /// `o`-rule (singleton nominal): merge `x` with the root of its individual.
    fn rule_nominal(&self, st: &mut State, x: usize) -> bool {
        let singletons: Vec<u32> = st.nodes[x]
            .label
            .iter()
            .filter_map(|&cid| match self.kb.table.decomp(cid) {
                Decomp::Nominal(v) if v.len() == 1 => Some(v[0]),
                _ => None,
            })
            .collect();
        let mut changed = false;
        for a in singletons {
            if st.nodes[find(st, x)].nominal == Some(a) {
                continue;
            }
            let ra = self.root(st, a);
            merge(st, ra, x);
            changed = true;
            if st.clash {
                return changed;
            }
        }
        changed
    }

    /// Add concept `c` to node `y`'s label; `⊤` is trivially present. Returns whether
    /// the label grew.
    fn add_concept(&self, st: &mut State, y: usize, c: u32) -> bool {
        if matches!(self.kb.table.decomp(c), Decomp::Top) {
            return false;
        }
        let y = find(st, y);
        st.nodes[y].label.insert(c)
    }

    /// Whether node `y` satisfies concept `c` (with `⊤` always satisfied).
    fn has_concept(&self, st: &State, y: usize, c: u32) -> bool {
        matches!(self.kb.table.decomp(c), Decomp::Top) || st.nodes[find(st, y)].label.contains(&c)
    }

    /// Create a fresh tree successor of `x` under `role`, labelled with `fillers`.
    fn new_successor(&self, st: &mut State, x: usize, role: Role, fillers: &[u32]) -> usize {
        let mut label = self.seed_label();
        for &c in fillers {
            if !matches!(self.kb.table.decomp(c), Decomp::Top) {
                label.insert(c);
            }
        }
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
            nominal: None,
            neq: BTreeSet::new(),
            merged: None,
        });
        // A forward role stores `x → y`; an inverse role stores `y → x`.
        if inverted {
            st.edges.push((idx, x, prop));
        } else {
            st.edges.push((x, idx, prop));
        }
        idx
    }

    /// The `role`-neighbours of `x` (deterministic, first-seen edge order).
    fn neighbors(&self, st: &State, x: usize, role: Role) -> Vec<usize> {
        let ach = self.achievers(role);
        let x = find(st, x);
        let mut out: Vec<usize> = Vec::new();
        let mut seen: BTreeSet<usize> = BTreeSet::new();
        for &(from, to, prop) in &st.edges {
            let f = find(st, from);
            let t = find(st, to);
            if ach.contains(&(prop, true)) && f == x && seen.insert(t) {
                out.push(t);
            }
            if ach.contains(&(prop, false)) && t == x && seen.insert(f) {
                out.push(f);
            }
        }
        out
    }

    /// The `(property, forward?)` edge patterns that realize `role`, closed under the
    /// role hierarchy and inverse-role declarations.
    fn achievers(&self, role: Role) -> BTreeSet<(u32, bool)> {
        let start = match role {
            Role::Named(p) => (p, true),
            Role::Inv(p) => (p, false),
        };
        let mut set: BTreeSet<(u32, bool)> = BTreeSet::new();
        let mut stack = vec![start];
        while let Some((q, dir)) = stack.pop() {
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
        set
    }

    /// Whether tree node `x` is blocked (directly or via a blocked ancestor).
    fn blocked(&self, st: &State, x: usize) -> bool {
        let x = find(st, x);
        if st.nodes[x].root {
            return false;
        }
        if self.directly_blocked(st, x) {
            return true;
        }
        match st.nodes[x].parent {
            Some(p) => self.blocked(st, find(st, p)),
            None => false,
        }
    }

    /// Pairwise (double) blocking: some strict ancestor `y` matches `x` on label, on
    /// predecessor label, and on the connecting edge role.
    fn directly_blocked(&self, st: &State, x: usize) -> bool {
        let px = match st.nodes[x].parent {
            Some(p) => find(st, p),
            None => return false,
        };
        let incoming_x = st.nodes[x].incoming;
        let mut y = px;
        loop {
            if !st.nodes[y].root {
                if let Some(py) = st.nodes[y].parent {
                    let py = find(st, py);
                    if st.nodes[x].label == st.nodes[y].label
                        && st.nodes[px].label == st.nodes[py].label
                        && incoming_x == st.nodes[y].incoming
                    {
                        return true;
                    }
                }
            }
            if st.nodes[y].root {
                return false;
            }
            match st.nodes[y].parent {
                Some(p) => y = find(st, p),
                None => return false,
            }
        }
    }

    /// Find the next non-deterministic expansion (the alternatives to try in order),
    /// or `None` if the graph is complete.
    fn find_branch(&self, st: &State) -> Option<Vec<Branch>> {
        let n = st.nodes.len();
        for i in 0..n {
            if find(st, i) != i {
                continue;
            }
            let cids: Vec<u32> = st.nodes[i].label.iter().copied().collect();
            for cid in cids {
                match *self.kb.table.decomp(cid) {
                    Decomp::Or(ref cs) => {
                        if !cs.iter().any(|c| st.nodes[i].label.contains(c)) {
                            return Some(cs.iter().map(|&c| Branch::AddConcept(i, c)).collect());
                        }
                    }
                    Decomp::Max(nmax, role, filler) => {
                        let neigh = self.neighbors(st, i, role);
                        // `≤`-choose rule: some neighbour lacks both `C` and `¬C`.
                        for &y in &neigh {
                            if !self.has_concept(st, y, filler)
                                && !self.has_concept(st, y, self.kb.table.negate(filler))
                            {
                                return Some(vec![
                                    Branch::AddConcept(y, filler),
                                    Branch::AddConcept(y, self.kb.table.negate(filler)),
                                ]);
                            }
                        }
                        // `≤`-merge rule: too many C-neighbours, some pair mergeable.
                        let with_c: Vec<usize> = neigh
                            .into_iter()
                            .filter(|&y| self.has_concept(st, y, filler))
                            .collect();
                        if with_c.len() > nmax as usize {
                            let mut branches: Vec<Branch> = Vec::new();
                            for a in 0..with_c.len() {
                                for b in (a + 1)..with_c.len() {
                                    if !are_distinct(st, with_c[a], with_c[b]) {
                                        branches.push(Branch::Merge(with_c[a], with_c[b]));
                                    }
                                }
                            }
                            if !branches.is_empty() {
                                return Some(branches);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        None
    }

    /// Apply one branch alternative; returns `false` if it clashes immediately.
    fn apply_branch(&self, st: &mut State, br: &Branch) -> bool {
        match *br {
            Branch::AddConcept(x, c) => {
                let x = find(st, x);
                st.nodes[x].label.insert(c);
                true
            }
            Branch::Merge(a, b) => {
                merge(st, a, b);
                !st.clash
            }
        }
    }
}

/// A maximum pairwise-compatible subset of `items` (a max clique under `compat`).
///
/// `compat(a, b)` is `true` when `a` and `b` may coexist (here: are forced `≠`).
/// Deterministic: prefers lower-indexed members. `items` are tiny in practice.
fn max_clique(items: &[usize], compat: &dyn Fn(usize, usize) -> bool) -> Vec<usize> {
    let mut best: Vec<usize> = Vec::new();
    let mut current: Vec<usize> = Vec::new();
    rec_clique(items, compat, 0, &mut current, &mut best);
    best
}

/// Backtracking helper for [`max_clique`].
fn rec_clique(
    items: &[usize],
    compat: &dyn Fn(usize, usize) -> bool,
    start: usize,
    current: &mut Vec<usize>,
    best: &mut Vec<usize>,
) {
    if current.len() > best.len() {
        *best = current.clone();
    }
    for i in start..items.len() {
        let cand = items[i];
        if current.iter().all(|&m| compat(m, cand)) {
            current.push(cand);
            rec_clique(items, compat, i + 1, current, best);
            current.pop();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::owl_dl::concept::{Concept, Role};

    /// A minimal KB builder for tableau primitives (no RDF parsing).
    struct Builder {
        kb: Kb,
    }

    impl Builder {
        fn new() -> Self {
            Self { kb: Kb::empty() }
        }

        fn concept(&mut self, c: Concept) -> u32 {
            self.kb.table.intern(c)
        }

        fn gci(&mut self, sub: Concept, sup: Concept) {
            self.kb.push_gci(sub, sup);
        }

        fn ty(&mut self, ind: u32, c: Concept) {
            let cid = self.kb.table.intern(c);
            self.kb.abox_types.push((ind, cid));
            self.kb.individuals.insert(ind);
        }

        fn role(&mut self, a: u32, p: u32, b: u32) {
            self.kb.abox_roles.push((a, p, b));
            self.kb.individuals.insert(a);
            self.kb.individuals.insert(b);
        }

        fn finish(mut self) -> Kb {
            self.kb.finalize();
            self.kb
        }
    }

    fn role(p: u32) -> Role {
        Role::Named(p)
    }

    #[test]
    fn atomic_contradiction_is_unsat() {
        let mut b = Builder::new();
        b.ty(
            1,
            Concept::And(vec![
                Concept::Named(10),
                Concept::Not(Box::new(Concept::Named(10))),
            ]),
        );
        let kb = b.finish();
        assert!(!kb.is_consistent().unwrap(), "A ⊓ ¬A must be unsatisfiable");
    }

    #[test]
    fn some_and_all_bottom_is_unsat() {
        // ∃r.⊤ ⊓ ∀r.⊥
        let mut b = Builder::new();
        b.ty(
            1,
            Concept::And(vec![
                Concept::Some(role(5), Box::new(Concept::Top)),
                Concept::All(role(5), Box::new(Concept::Bottom)),
            ]),
        );
        let kb = b.finish();
        assert!(!kb.is_consistent().unwrap(), "∃r.⊤ ⊓ ∀r.⊥ must be unsat");
    }

    #[test]
    fn min_two_max_one_is_unsat() {
        // ≥2 r.⊤ ⊓ ≤1 r.⊤
        let mut b = Builder::new();
        b.ty(
            1,
            Concept::And(vec![
                Concept::Min(2, role(5), Box::new(Concept::Top)),
                Concept::Max(1, role(5), Box::new(Concept::Top)),
            ]),
        );
        let kb = b.finish();
        assert!(
            !kb.is_consistent().unwrap(),
            "≥2 r.⊤ ⊓ ≤1 r.⊤ must be unsat"
        );
    }

    #[test]
    fn cyclic_gci_is_consistent_and_terminates() {
        // C ⊑ ∃r.C with an instance of C: consistent, and blocking makes it terminate.
        let mut b = Builder::new();
        let c = Concept::Named(10);
        b.gci(c.clone(), Concept::Some(role(5), Box::new(c.clone())));
        b.ty(1, c);
        let kb = b.finish();
        assert!(
            kb.is_consistent().unwrap(),
            "cyclic C ⊑ ∃r.C is consistent (pairwise blocking terminates)"
        );
    }

    #[test]
    fn disjointness_clash() {
        // A ⊓ B ⊑ ⊥, x : A, x : B
        let mut b = Builder::new();
        b.gci(
            Concept::And(vec![Concept::Named(10), Concept::Named(11)]),
            Concept::Bottom,
        );
        b.ty(1, Concept::Named(10));
        b.ty(1, Concept::Named(11));
        let kb = b.finish();
        assert!(
            !kb.is_consistent().unwrap(),
            "disjoint A,B with a common instance is unsat"
        );
    }

    #[test]
    fn min_two_over_single_nominal_is_unsat() {
        // ≥2 r.{a}: only one nominal filler exists, so two distinct fillers clash.
        let mut b = Builder::new();
        b.ty(
            1,
            Concept::Min(2, role(5), Box::new(Concept::Nominal(vec![99]))),
        );
        let kb = b.finish();
        assert!(
            !kb.is_consistent().unwrap(),
            "≥2 r.{{a}} must be unsat (one nominal)"
        );
    }

    #[test]
    fn min_one_over_nominal_is_consistent() {
        // ≥1 r.{a} is fine.
        let mut b = Builder::new();
        b.ty(
            1,
            Concept::Min(1, role(5), Box::new(Concept::Nominal(vec![99]))),
        );
        let kb = b.finish();
        assert!(kb.is_consistent().unwrap(), "≥1 r.{{a}} is satisfiable");
    }

    #[test]
    fn instance_check_via_role_and_some() {
        // x r y, y : B  ⇒  x : ∃r.B
        let mut b = Builder::new();
        let bcls = b.concept(Concept::Named(11));
        b.ty(2, Concept::Named(11)); // y : B
        b.role(1, 5, 2); // x r y
        let some_rb = b.concept(Concept::Some(role(5), Box::new(Concept::Named(11))));
        let _ = bcls;
        let kb = b.finish();
        assert!(
            kb.entails_instance(1, some_rb).unwrap(),
            "x with an r-edge to a B is an instance of ∃r.B"
        );
    }
}
