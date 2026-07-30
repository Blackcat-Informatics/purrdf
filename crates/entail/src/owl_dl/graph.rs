// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

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

use std::collections::{BTreeMap, BTreeSet};

use crate::owl_dl::Kb;
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
    /// Whether this is a root (named-individual / nominal) node — never blocked.
    pub(crate) root: bool,
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

/// A completion graph under construction.
#[derive(Clone)]
pub(crate) struct State {
    /// All nodes ever created (merged-away ones remain, forwarded via `merged`).
    pub(crate) nodes: Vec<Node>,
    /// Directed role edges `(from, to, property)`; endpoints resolved via [`find`].
    pub(crate) edges: Vec<(usize, usize, u32)>,
    /// Named individual term id → its root node index.
    pub(crate) root_of: BTreeMap<u32, usize>,
    /// A clash has been detected (e.g. a forced `≠` merge).
    pub(crate) clash: bool,
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

/// What one decision procedure run decided, and what it consumed deciding it.
///
/// `consistent` is meaningful only when `exhausted` is false: a run that stopped at its
/// cap has closed some branches and not others, and reporting the "no branch succeeded
/// *yet*" state as `false` would turn a resource limit into an entailment. Every consumer
/// in this crate reads `exhausted` first.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Decision {
    /// Whether a clash-free completion was found. Only meaningful when `!exhausted`.
    pub(crate) consistent: bool,
    /// Derivation rounds consumed, summed over every branch the search explored.
    pub(crate) steps: u64,
    /// Whether the search stopped because it reached its step cap.
    pub(crate) exhausted: bool,
}

/// The search reached its step cap. A private marker rather than an
/// [`EntailError`](crate::EntailError): it is not a failure at this layer, it is one of the
/// three things a decision reports.
pub(crate) struct Exhausted;

/// The step cap for a knowledge base: generous and size-proportional.
///
/// Blocking bounds the real work far below this, so reaching it means a termination bug or
/// an adversarial instance rather than an ordinary ontology. It is a pure function of the
/// knowledge base — same input, same cap — so a [`Decision`] is reproducible run to run, and
/// it is a STEP count rather than a clock reading, which is what keeps it reproducible on
/// wasm32 (where there is no clock to read).
pub(crate) fn step_cap(kb: &Kb) -> u64 {
    let base =
        (kb.abox_types.len() + kb.abox_roles.len() + kb.tbox.len() + kb.individuals.len() + 16)
            as u64;
    100_000 + base.saturating_mul(base).saturating_mul(64)
}

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

/// A maximum pairwise-compatible subset of `items` (a max clique under `compat`).
///
/// `compat(a, b)` is `true` when `a` and `b` may coexist (here: are forced `≠`).
/// Deterministic: prefers lower-indexed members. `items` are tiny in practice.
pub(crate) fn max_clique(items: &[usize], compat: &dyn Fn(usize, usize) -> bool) -> Vec<usize> {
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

/// The knowledge base plus the internalized TBox, and every operation on a completion graph
/// over them.
///
/// Read-only in the knowledge base and stateless in itself: a decision procedure owns its own
/// budget and its own rule set, and borrows this for the graph operations both procedures must
/// perform identically.
pub(crate) struct Graph<'a> {
    /// The knowledge base (concept table, role hierarchy, inverses).
    kb: &'a Kb,
    /// The internalized TBox: meta-concept ids placed in every abstract node's label.
    meta: BTreeSet<u32>,
}

impl<'a> Graph<'a> {
    /// Build the graph operations over `kb`, snapshotting the internalized TBox.
    pub(crate) fn new(kb: &'a Kb) -> Self {
        Self {
            kb,
            meta: kb.meta.iter().copied().collect(),
        }
    }

    /// The knowledge base every rule reads.
    pub(crate) const fn kb(&self) -> &'a Kb {
        self.kb
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
            st.edges.push((ra, rb, p));
        }
        if !fresh_types.is_empty() {
            let mut label = self.seed_label();
            label.extend(fresh_types.iter().copied());
            st.nodes.push(Node {
                label,
                parent: None,
                incoming: None,
                root: true,
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
        st.nodes.push(Node {
            label: if concrete {
                BTreeSet::new()
            } else {
                self.seed_label()
            },
            parent: None,
            incoming: None,
            root: true,
            nominals: std::iter::once(a).collect(),
            neq: BTreeSet::new(),
            merged: None,
            concrete,
            value_class: self.kb.literal_class.get(&a).copied(),
        });
        st.root_of.insert(a, idx);
        idx
    }

    /// Merge `discard` into `keep`, identifying the two nodes.
    ///
    /// Orientation keeps a root over a tree node, else the lower index. A forced merge of
    /// a `≠` pair sets [`State::clash`].
    ///
    /// # Why this needs the internalized TBox
    ///
    /// Identifying an abstract node with a literal's node says the abstract node WAS that
    /// literal value all along, and every meta-concept the internalized TBox put on it
    /// therefore never applied: a general concept inclusion quantifies over `owl:Thing`, and no
    /// literal value is in it. Dropping them needs [`Graph::meta`] in scope, which is what
    /// makes this a method rather than a free function over the state.
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
        // Carry every nominal identity onto the keeper; repoint the root map. The keeper
        // now denotes *both* names, which is exactly what the absence of a unique name
        // assumption permits.
        let disc_nominals = st.nodes[discard].nominals.clone();
        for &a in &disc_nominals {
            st.root_of.insert(a, keep);
        }
        st.nodes[keep].nominals.extend(disc_nominals);
        if st.nodes[discard].root {
            st.nodes[keep].root = true;
        }
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
        // The keeper is now known to inhabit the DATA domain, so the internalized TBox never
        // constrained it. Withdrawing those meta-concepts can only remove a clash, never add
        // one, which is the direction an identification is allowed to move the answer in.
        if st.nodes[keep].concrete {
            for meta in &self.meta {
                st.nodes[keep].label.remove(meta);
            }
        }
        st.nodes[discard].merged = Some(keep);
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
            nominals: BTreeSet::new(),
            neq: BTreeSet::new(),
            merged: None,
            concrete,
            value_class: None,
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
    /// Counting a transitive role's neighbours in a `≤n` restriction is only meaningful
    /// because OWL 2 DL forbids exactly that combination; an ontology that states it is not
    /// OWL 2 DL and the reverse mapping raises
    /// [`Construct::NonSimpleRole`](crate::Construct::NonSimpleRole) for it.
    pub(crate) fn neighbors(&self, st: &State, x: usize, role: Role) -> Vec<usize> {
        let ach = self.achievers(role);
        let x = find(st, x);
        let mut out: Vec<usize> = Vec::new();
        let mut seen: BTreeSet<usize> = BTreeSet::new();
        self.step(st, x, &ach, &mut seen, &mut out);
        for &(prop, dir) in &ach {
            if !self.kb.transitive.contains(&prop) {
                continue;
            }
            let single: BTreeSet<(u32, bool)> = std::iter::once((prop, dir)).collect();
            // Breadth-first over this one transitive role, seeded from `x`'s own step.
            let mut frontier: Vec<usize> = Vec::new();
            self.step(st, x, &single, &mut BTreeSet::new(), &mut frontier);
            let mut visited: BTreeSet<usize> = frontier.iter().copied().collect();
            while let Some(y) = frontier.pop() {
                if seen.insert(y) {
                    out.push(y);
                }
                let mut next: Vec<usize> = Vec::new();
                self.step(st, y, &single, &mut BTreeSet::new(), &mut next);
                for z in next {
                    if visited.insert(z) {
                        frontier.push(z);
                    }
                }
            }
        }
        out
    }

    /// One edge step from `x` over the `(property, forward?)` patterns `ach`, appending
    /// newly seen endpoints to `out` in first-seen edge order.
    fn step(
        &self,
        st: &State,
        x: usize,
        ach: &BTreeSet<(u32, bool)>,
        seen: &mut BTreeSet<usize>,
        out: &mut Vec<usize>,
    ) {
        let x = find(st, x);
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

    /// Whether `x` has a `role`-edge to itself, read through the role hierarchy and the
    /// inverse-role closure.
    pub(crate) fn has_self_loop(&self, st: &State, x: usize, role: Role) -> bool {
        let x = find(st, x);
        self.neighbors(st, x, role).contains(&x)
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
        st.edges.push((x, x, property));
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
            for &other in &st.nodes[x].label {
                if let Decomp::All(universal_role, universal_filler) = *self.kb.table.decomp(other)
                    && universal_role == role
                    && let Decomp::Data(narrowed) = *self.kb.table.decomp(universal_filler)
                {
                    demanded.push(narrowed);
                }
            }
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
        let with_filler: Vec<usize> = self
            .neighbors(st, x, role)
            .into_iter()
            .filter(|&y| self.has_concept(st, y, filler))
            .collect();
        let mut clique = max_clique(&with_filler, &|a, b| are_distinct(st, a, b));
        if clique.len() >= n {
            return false;
        }
        while clique.len() < n {
            let y = self.new_successor(st, x, role, &[filler]);
            clique.push(y);
        }
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
        let with_filler: Vec<usize> = self
            .neighbors(st, x, role)
            .into_iter()
            .filter(|&y| self.has_concept(st, y, filler))
            .collect();
        max_clique(&with_filler, &|a, b| are_distinct(st, a, b)).len() >= n as usize
    }
}
