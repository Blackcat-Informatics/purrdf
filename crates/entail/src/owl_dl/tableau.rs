// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The CONCEPT-TREE `SHOIQ(D)` tableau — the hypertableau's differential reference.
//!
//! A from-scratch implementation of the standard completion-graph algorithm
//! (Horrocks & Sattler, "A Tableau Decision Procedure for SHOIQ", 2007; Baader et
//! al., "The Description Logic Handbook", ch. 3) over the `SHOIQ(D)` fragment: the
//! boolean connectives, existential/universal restrictions, transitive roles (`S`),
//! role hierarchies (`H`), qualified number restrictions (`Q`), inverse roles (`I`),
//! and nominals (`O`), over a CONCRETE DOMAIN (`D`) of datatype values. Beyond the
//! letters it also decides self-restrictions (`owl:hasSelf`, and through them the
//! reflexive/irreflexive role axioms), role disjointness and asymmetry; the one
//! `SROIQ` role feature it does NOT decide is `owl:propertyChainAxiom`, which is a
//! named [`Construct::PropertyChain`](crate::report::Construct) boundary rather than
//! a silent drop. Algorithms are not copyrightable; this code is original.
//!
//! # Why it is still here
//!
//! [`crate::owl_dl::hyper`] decides every question this crate asks; this module decides
//! none of them. It is kept, compiled under `cfg(test)`, as the DIFFERENTIAL REFERENCE
//! for that calculus — the same pattern as
//! [`Subsumptions::decide_by_tableau`](crate::reasoner::classify::Subsumptions::decide_by_tableau).
//! Two implementations of one contract can be held against each other; one
//! implementation can only be held against itself. The two read the SAME clause-free
//! input (the concept table, the internalized TBox [`Kb::meta`], the absorbed
//! [`Kb::unfold`]) and build the SAME completion graph through [`Graph`], so a verdict
//! difference between them is a difference of CALCULUS — which is exactly what the
//! differential test exists to find, and why no divergence may be ledgered.
//!
//! Every generated knowledge base of [`crate::owl_dl::oracle`] (5,700 per run) and every
//! hand-written knowledge base in this module is decided by both.
//!
//! # Shape of the search
//!
//! A [`State`] is a completion graph: nodes carry a `BTreeSet` label of concept ids,
//! directed role edges connect them, and a `≠` (distinctness) relation records forced
//! inequalities. The TBox is *internalized*: every general concept inclusion `C ⊑ D`
//! becomes the meta-concept `nnf(¬C ⊔ D)`, and the union of all such meta-concepts is
//! placed in every node's label at creation. [`Tableau::solve`] runs the deterministic
//! completion rules to a fixpoint, then branches (depth-first, in a fully deterministic
//! order) on the non-deterministic rules (`⊔`, the `≤`-choose rule, `≤`-merges, and the
//! multi-member `o`-rule), cloning the state per branch. A branch that reaches a
//! clash-free fixpoint witnesses consistency.
//!
//! Each rule reads the STRUCTURE of the concepts in a label at search time, which is the
//! difference the hypertableau removes: there, the structure is compiled into DL-clauses
//! once and a rule instance is a clause body matching the graph. The two rule sets decide
//! the same fragment, and the module documentation of [`crate::owl_dl::hyper`] maps each of
//! these rules onto the clause form that replaced it.
//!
//! # Termination
//!
//! Tree nodes are subject to **pairwise (double) ANCESTOR blocking**: a tree node is
//! blocked by an ancestor when their labels *and* their predecessors' labels *and* the
//! connecting edge roles all coincide. Nominal/root nodes (one per named individual) are
//! never blocked. The hypertableau relaxes the ancestor requirement to ANYWHERE blocking,
//! which blocks strictly more; see its module docs for why that is sound and why it is
//! sufficient here. A generous per-run step cap is a hard backstop: a termination bug
//! surfaces as an [`EntailError::Build`] rather than a hang.
//!
//! # No unique name assumption
//!
//! OWL 2 does not assume distinct names denote distinct elements. Nominals are
//! therefore handled by *identification*, never by name comparison: `{a} ∈ L(x)` merges
//! `x` with `a`'s root whatever `x` is already called, and a nominal set with more than
//! one member branches over the members. Two named individuals become distinct only
//! when something forces it — an explicit `≠` recorded by the `≥`-rule, or a `¬{a}` in
//! a label — and only then can a nominal constraint clash. The merge itself is
//! [`Graph::merge_nodes`], shared with the hypertableau, so neither calculus can grow a
//! second answer to the identity question.

use crate::EntailError;
use crate::owl_dl::Kb;
use crate::owl_dl::concept::{Decomp, Role};
use crate::owl_dl::graph::{
    Assumptions, Decision, Exhausted, Graph, State, are_distinct, find, max_clique, set_distinct,
    step_cap,
};

/// A non-deterministic expansion alternative.
#[derive(Clone)]
enum Branch {
    /// Add a concept id to a node's label.
    AddConcept(usize, u32),
    /// Merge two nodes (identify them).
    Merge(usize, usize),
    /// Identify a node with the root of the named individual with this term id — the
    /// `o`-rule's choice for a nominal set with more than one member.
    MergeNominal(usize, u32),
}

/// The tableau driver: the shared completion graph, and a step cap.
struct Tableau<'a> {
    /// The completion-graph operations over the knowledge base and its internalized TBox.
    g: Graph<'a>,
    /// Steps consumed so far.
    steps: u64,
    /// Hard step cap; exceeding it is a hard error (a termination-bug backstop).
    cap: u64,
}

/// Decide whether the knowledge base plus `assumptions` has a consistent completion,
/// spending at most `cap` steps.
pub(crate) fn decide(kb: &Kb, assumptions: &Assumptions<'_>, cap: u64) -> Decision {
    let mut t = Tableau::new(kb, cap);
    let st = t.g.init_state(assumptions);
    match t.solve(st) {
        Ok(consistent) => Decision {
            consistent,
            steps: t.steps,
            exhausted: false,
        },
        Err(Exhausted) => Decision {
            consistent: false,
            steps: t.steps,
            exhausted: true,
        },
    }
}

/// Decide whether the knowledge base plus `assumptions` has a consistent completion.
///
/// # Errors
///
/// [`EntailError::Build`] if the step cap is exceeded (a termination-bug backstop).
pub(crate) fn consistent(kb: &Kb, assumptions: &Assumptions<'_>) -> Result<bool, EntailError> {
    let decision = decide(kb, assumptions, step_cap(kb));
    if decision.exhausted {
        return Err(EntailError::Build(
            "OWL-Direct tableau exceeded its step cap (possible non-termination)".to_owned(),
        ));
    }
    Ok(decision.consistent)
}

impl<'a> Tableau<'a> {
    /// Build a driver over `kb` bounded by `cap` steps.
    fn new(kb: &'a Kb, cap: u64) -> Self {
        Self {
            g: Graph::new(kb),
            steps: 0,
            cap,
        }
    }

    /// The depth-first, deterministic search: saturate, then branch.
    fn solve(&mut self, mut st: State) -> Result<bool, Exhausted> {
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
    fn saturate(&mut self, st: &mut State) -> Result<bool, Exhausted> {
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
    fn tick(&mut self) -> Result<(), Exhausted> {
        if self.steps >= self.cap {
            return Err(Exhausted);
        }
        self.steps += 1;
        Ok(())
    }

    /// Structural clash detection over the current representatives.
    fn detect_clash(&self, st: &mut State) {
        let n = st.nodes.len();
        for i in 0..n {
            if find(st, i) != i {
                continue;
            }
            if self.node_clashes(st, i) || self.role_axiom_clashes(st, i) {
                st.clash = true;
                return;
            }
        }
    }

    /// Whether a ROLE axiom that constrains edges rather than labels is violated at `x`.
    ///
    /// Two of OWL 2's role axioms say nothing about any node's concept label, so neither
    /// can be internalized as a GCI and neither is caught by [`Tableau::node_clashes`]:
    ///
    /// * `owl:AsymmetricProperty` — `r(x, y)` and `r(y, x)` cannot both hold. A self-loop
    ///   is the case `y = x`, so asymmetry subsumes irreflexivity exactly as OWL 2 says it
    ///   does.
    /// * `owl:propertyDisjointWith` / `owl:AllDisjointProperties` — one pair `(x, y)`
    ///   cannot carry both roles.
    ///
    /// Both are decided over [`Tableau::neighbors`], so both see the role hierarchy and the
    /// inverse-role closure: a sub-role edge of an asymmetric role violates the axiom, which
    /// is what makes the check about the role's EXTENSION rather than about its spelling.
    fn role_axiom_clashes(&self, st: &State, x: usize) -> bool {
        for &property in &self.g.kb().asymmetric {
            let role = Role::Named(property);
            for y in self.g.neighbors(st, x, role) {
                if self.g.neighbors(st, y, role).contains(&x) {
                    return true;
                }
            }
        }
        for &(left, right) in &self.g.kb().disjoint_roles {
            let shared = self.g.neighbors(st, x, Role::Named(left));
            if shared.is_empty() {
                continue;
            }
            let other = self.g.neighbors(st, x, Role::Named(right));
            if shared.iter().any(|y| other.contains(y)) {
                return true;
            }
        }
        false
    }

    /// Whether representative node `x` carries a clash.
    ///
    /// The clash triggers are `⊥`, a complementary concept pair `C, ¬C`, a negated
    /// nominal `¬{…}` naming one of the node's *own* individuals, a `≤n r.C`
    /// violated by more than `n` pairwise-distinct `C`-neighbours, and an unsatisfiable
    /// CONCRETE-domain constraint set ([`Tableau::data_clashes`]).
    ///
    /// A **positive** nominal set `{a₁,…,aₙ} ∈ L(x)` is deliberately *not* a clash
    /// trigger, even when `x` already denotes some other name. OWL 2 makes no unique
    /// name assumption, so `x` may simply *be* one of the `aᵢ` under a second name;
    /// the sound response is the identification performed by [`Tableau::rule_nominal`]
    /// (and, for a set with more than one member, the `o`-branch produced by
    /// [`Tableau::find_branch`]), which clashes only when every such identification is
    /// blocked by a recorded `≠`.
    fn node_clashes(&self, st: &State, x: usize) -> bool {
        let node = &st.nodes[x];
        if node.label.contains(&self.g.kb().bottom) {
            return true;
        }
        for &cid in &node.label {
            if node.label.contains(&self.g.kb().table.negate(cid)) {
                return true;
            }
            if let Decomp::NegNominal(w) = self.g.kb().table.decomp(cid)
                && w.iter().any(|a| node.nominals.contains(a))
            {
                return true;
            }
            // `¬∃r.Self` on a node that HAS an `r`-loop. This is the whole content of
            // `owl:IrreflexiveProperty` (`⊤ ⊑ ¬∃r.Self`) as well as of a negated
            // `owl:hasSelf`, so both are decided here rather than by a second mechanism.
            if let Decomp::NegSelfRestriction(role) = *self.g.kb().table.decomp(cid)
                && self.g.has_self_loop(st, x, role)
            {
                return true;
            }
        }
        self.max_clash(st, x) || self.g.data_clashes(st, x)
    }

    /// Whether some `≤n r.C` on node `x` is violated by `> n` pairwise-`≠` neighbours.
    fn max_clash(&self, st: &State, x: usize) -> bool {
        let cids: Vec<u32> = st.nodes[x].label.iter().copied().collect();
        for cid in cids {
            if let Decomp::Max(n, role, c) = *self.g.kb().table.decomp(cid) {
                let filler = c;
                let neigh = self.g.neighbors(st, x, role);
                let with_c: Vec<usize> = neigh
                    .into_iter()
                    .filter(|&y| self.g.has_concept(st, y, filler))
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
            changed |= self.rule_self(st, i);
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
            if let Some(sups) = self.g.kb().unfold.get(&cid) {
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
            if let Decomp::And(cs) = self.g.kb().table.decomp(cid) {
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
            .filter_map(|&cid| match *self.g.kb().table.decomp(cid) {
                Decomp::All(role, c) => Some((role, c)),
                _ => None,
            })
            .collect();
        let mut changed = false;
        for (role, c) in alls {
            for y in self.g.neighbors(st, x, role) {
                changed |= self.g.add_concept(st, y, c);
            }
        }
        changed
    }

    /// `∃`-rule: `∃r.C ∈ L(x)` with no `r`-neighbour satisfying `C` creates one.
    fn rule_exists(&self, st: &mut State, x: usize) -> bool {
        let somes: Vec<(Role, u32)> = st.nodes[x]
            .label
            .iter()
            .filter_map(|&cid| match *self.g.kb().table.decomp(cid) {
                Decomp::Some(role, c) => Some((role, c)),
                _ => None,
            })
            .collect();
        let mut changed = false;
        for (role, c) in somes {
            let has = self
                .g
                .neighbors(st, x, role)
                .into_iter()
                .any(|y| self.g.has_concept(st, y, c));
            if !has {
                self.g.new_successor(st, x, role, &[c]);
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
            .filter_map(|&cid| match *self.g.kb().table.decomp(cid) {
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
                .g
                .neighbors(st, x, role)
                .into_iter()
                .filter(|&y| self.g.has_concept(st, y, c))
                .collect();
            let mut clique = max_clique(&with_c, &|a, b| are_distinct(st, a, b));
            if clique.len() >= n {
                continue;
            }
            while clique.len() < n {
                let y = self.g.new_successor(st, x, role, &[c]);
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

    /// `o`-rule (singleton nominal): identify `x` with the root of its individual.
    ///
    /// `{a} ∈ L(x)` says `x` **is** `a`. Since OWL 2 makes no unique name assumption,
    /// this holds however `x` was named: the rule merges the two nodes rather than
    /// asking whether the names agree. The merge is what turns a genuine
    /// non-membership into a clash — [`merge`] refuses to identify a pair already
    /// recorded as `≠` and sets [`State::clash`] instead — so no separate "not a
    /// member" test is needed, and none may be added without assuming unique names.
    fn rule_nominal(&self, st: &mut State, x: usize) -> bool {
        let singletons: Vec<u32> = st.nodes[x]
            .label
            .iter()
            .filter_map(|&cid| match self.g.kb().table.decomp(cid) {
                Decomp::Nominal(v) if v.len() == 1 => Some(v[0]),
                _ => None,
            })
            .collect();
        let mut changed = false;
        for a in singletons {
            let rx = find(st, x);
            // Already denotes `a`: nothing to identify, and reporting a change here
            // would spin `saturate` forever.
            if st.nodes[rx].nominals.contains(&a) {
                continue;
            }
            let ra = self.g.root(st, a);
            self.g.merge_nodes(st, ra, rx);
            changed = true;
            if st.clash {
                return changed;
            }
        }
        changed
    }

    /// `Self`-rule: `∃r.Self ∈ L(x)` gives `x` an `r`-edge to itself.
    ///
    /// The edge, not a fresh successor: `∃r.Self` says the node is its OWN `r`-successor,
    /// which is why it is an atomic leaf rather than a quantifier and why the rule is
    /// deterministic and terminating — a node has at most one loop per role, so the rule
    /// fires at most once per `(node, role)` pair.
    ///
    /// This is also `owl:ReflexiveProperty`: that axiom is internalized as the GCI
    /// `⊤ ⊑ ∃r.Self`, so the meta-concept lands in every node's label at creation and this
    /// rule puts the loop on every node.
    fn rule_self(&self, st: &mut State, x: usize) -> bool {
        let selves: Vec<Role> = st.nodes[x]
            .label
            .iter()
            .filter_map(|&cid| match *self.g.kb().table.decomp(cid) {
                Decomp::SelfRestriction(role) => Some(role),
                _ => None,
            })
            .collect();
        let mut changed = false;
        for role in selves {
            changed |= self.g.add_self_loop(st, x, role);
        }
        changed
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
            if !st.nodes[y].root
                && let Some(py) = st.nodes[y].parent
            {
                let py = find(st, py);
                if st.nodes[x].label == st.nodes[y].label
                    && st.nodes[px].label == st.nodes[py].label
                    && incoming_x == st.nodes[y].incoming
                {
                    return true;
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
                match *self.g.kb().table.decomp(cid) {
                    Decomp::Or(ref cs) => {
                        if !cs.iter().any(|c| st.nodes[i].label.contains(c)) {
                            return Some(cs.iter().map(|&c| Branch::AddConcept(i, c)).collect());
                        }
                    }
                    // `o`-rule, non-deterministic form: `{a₁,…,aₙ} ∈ L(x)` with `n > 1`
                    // and `x` denoting none of the `aᵢ` yet. `x` must be *one* of
                    // them, so each identification is an alternative; the whole set
                    // failing is what makes non-membership a clash. (The RDF reverse
                    // mapping pre-splits `owl:oneOf` into a disjunction of singletons,
                    // so this fires only on a directly built multi-member nominal —
                    // but soundness here must not rest on that parser convention.)
                    Decomp::Nominal(ref v) if v.len() > 1 => {
                        if !v.iter().any(|a| st.nodes[i].nominals.contains(a)) {
                            return Some(v.iter().map(|&a| Branch::MergeNominal(i, a)).collect());
                        }
                    }
                    Decomp::Max(nmax, role, filler) => {
                        let neigh = self.g.neighbors(st, i, role);
                        // `≤`-choose rule: some neighbour lacks both `C` and `¬C`.
                        for &y in &neigh {
                            if !self.g.has_concept(st, y, filler)
                                && !self.g.has_concept(st, y, self.g.kb().table.negate(filler))
                            {
                                return Some(vec![
                                    Branch::AddConcept(y, filler),
                                    Branch::AddConcept(y, self.g.kb().table.negate(filler)),
                                ]);
                            }
                        }
                        // `≤`-merge rule: too many C-neighbours, some pair mergeable.
                        let with_c: Vec<usize> = neigh
                            .into_iter()
                            .filter(|&y| self.g.has_concept(st, y, filler))
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
                self.g.merge_nodes(st, a, b);
                !st.clash
            }
            Branch::MergeNominal(x, a) => {
                let x = find(st, x);
                let ra = self.g.root(st, a);
                self.g.merge_nodes(st, ra, x);
                !st.clash
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use purrdf_core::{BlankScope, RdfDatasetBuilder, TermId};

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

    /// Whether `kb` is consistent, DECIDED BY BOTH cores, which must agree.
    ///
    /// Every hand-written knowledge base below is a differential case: the hypertableau is the
    /// production answer and this module's concept-tree tableau is the reference, so a verdict
    /// they disagree on fails here rather than being silently attributed to the newer calculus.
    fn consistent_by_both(kb: &Kb) -> bool {
        let hyper = kb
            .is_consistent()
            .expect("the hypertableau decides this fixture");
        let concept_tree = kb
            .is_consistent_by_concept_tree()
            .expect("the concept-tree tableau decides this fixture");
        assert_eq!(
            hyper, concept_tree,
            "the two decision cores disagree about this knowledge base: hypertableau {hyper}, \
             concept-tree tableau {concept_tree}"
        );
        hyper
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
        assert!(!consistent_by_both(&kb), "A ⊓ ¬A must be unsatisfiable");
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
        assert!(!consistent_by_both(&kb), "∃r.⊤ ⊓ ∀r.⊥ must be unsat");
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
        assert!(!consistent_by_both(&kb), "≥2 r.⊤ ⊓ ≤1 r.⊤ must be unsat");
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
            consistent_by_both(&kb),
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
            !consistent_by_both(&kb),
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
            !consistent_by_both(&kb),
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
        assert!(consistent_by_both(&kb), "≥1 r.{{a}} is satisfiable");
    }

    /// The `example.org` fixture namespace for the `owl:oneOf` regression pair below.
    const NS: &str = "http://example.org/oneof#";

    /// An `owl:oneOf` enumeration builder over `example.org` terms.
    ///
    /// Emits `ex:Enum owl:oneOf (ex:small ex:medium ex:large)` plus
    /// `ex:myT rdf:type ex:Enum`, and then, for each name in `apart_from`, an
    /// `ex:myT rdf:type [ owl:complementOf [ owl:oneOf (ex:<name>) ] ]` — the class
    /// expression whose DL reading is `¬{name}`, i.e. exactly the content of
    /// `ex:myT owl:differentFrom ex:<name>`. (The `owl:differentFrom` *spelling* is a
    /// separate, still-ledgered gap in the reverse mapping's vocabulary; this fixture
    /// states the distinctness the tableau is meant to act on, not the syntax the
    /// parser is meant to recognize.)
    fn one_of_kb(apart_from: &[&str]) -> Kb {
        const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
        const RDF_FIRST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#first";
        const RDF_REST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#rest";
        const RDF_NIL: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#nil";
        const OWL_ONEOF: &str = "http://www.w3.org/2002/07/owl#oneOf";
        const OWL_COMPLEMENTOF: &str = "http://www.w3.org/2002/07/owl#complementOf";

        let mut b = RdfDatasetBuilder::new();
        let ty = b.intern_iri(RDF_TYPE);
        let first = b.intern_iri(RDF_FIRST);
        let rest = b.intern_iri(RDF_REST);
        let nil = b.intern_iri(RDF_NIL);
        let one_of = b.intern_iri(OWL_ONEOF);
        let complement_of = b.intern_iri(OWL_COMPLEMENTOF);
        let mut cell = 0usize;
        // Write `members` as an RDF list, returning its head.
        let mut list = |b: &mut RdfDatasetBuilder, members: &[TermId]| -> TermId {
            let mut head = nil;
            for &m in members.iter().rev() {
                cell += 1;
                let node = b.intern_blank(&format!("cell{cell}"), BlankScope::DEFAULT);
                b.push_quad(node, first, m, None);
                b.push_quad(node, rest, head, None);
                head = node;
            }
            head
        };

        let members: Vec<TermId> = ["small", "medium", "large"]
            .iter()
            .map(|n| b.intern_iri(&format!("{NS}{n}")))
            .collect();
        let enum_class = b.intern_iri(&format!("{NS}Enum"));
        let head = list(&mut b, &members);
        b.push_quad(enum_class, one_of, head, None);
        let my_t = b.intern_iri(&format!("{NS}myT"));
        b.push_quad(my_t, ty, enum_class, None);

        for (i, name) in apart_from.iter().enumerate() {
            let member = b.intern_iri(&format!("{NS}{name}"));
            let singleton = b.intern_blank(&format!("only{i}"), BlankScope::DEFAULT);
            let head = list(&mut b, &[member]);
            b.push_quad(singleton, one_of, head, None);
            let not_member = b.intern_blank(&format!("not{i}"), BlankScope::DEFAULT);
            b.push_quad(not_member, complement_of, singleton, None);
            b.push_quad(my_t, ty, not_member, None);
        }

        let ds = b.freeze().expect("freeze");
        Kb::from_dataset(&ds).expect("parse")
    }

    #[test]
    fn oneof_membership_makes_no_unique_name_assumption() {
        // `myT : {small, medium, large}` with nothing saying `myT` differs from any of
        // them. OWL 2 has no unique name assumption, so `myT` may simply BE one of the
        // three under a second name — the ontology is satisfiable. Reporting a clash
        // here (as the tableau once did, purely because `myT` is not syntactically in
        // the enumeration) is an unsoundness, not a missing feature.
        let kb = one_of_kb(&[]);
        assert!(
            consistent_by_both(&kb),
            "an individual typed into an owl:oneOf it is not syntactically a member of \
             is satisfiable: it may denote a member under another name"
        );
    }

    #[test]
    fn oneof_membership_clashes_when_apart_from_every_member() {
        // The dual. Add `myT ≠ small`, `myT ≠ medium`, `myT ≠ large`; now every
        // identification the `o`-rule could make is blocked, so `myT` is provably
        // outside an enumeration it is typed into and the ontology IS unsatisfiable.
        // This is what separates the fix from simply deleting the rule.
        let kb = one_of_kb(&["small", "medium", "large"]);
        assert!(
            !consistent_by_both(&kb),
            "an individual known distinct from every enumeration member cannot be typed \
             into that enumeration"
        );
    }

    #[test]
    fn multi_member_nominal_branches_over_its_members() {
        // A nominal set built directly (not via the parser's disjunction pre-split), so
        // the `o`-rule's non-deterministic form is what has to decide it. `x : {a, b}`
        // with `x ≠ a` still has the `x = b` alternative...
        let mut b = Builder::new();
        b.ty(1, Concept::Nominal(vec![98, 99]));
        b.ty(1, Concept::Not(Box::new(Concept::Nominal(vec![98]))));
        let kb = b.finish();
        assert!(
            consistent_by_both(&kb),
            "x : {{a,b}} with x ≠ a is satisfiable by identifying x with b"
        );

        // ...and ruling that out too leaves nothing to identify `x` with.
        let mut b = Builder::new();
        b.ty(1, Concept::Nominal(vec![98, 99]));
        b.ty(1, Concept::Not(Box::new(Concept::Nominal(vec![98]))));
        b.ty(1, Concept::Not(Box::new(Concept::Nominal(vec![99]))));
        let kb = b.finish();
        assert!(
            !consistent_by_both(&kb),
            "x : {{a,b}} with x ≠ a and x ≠ b is unsatisfiable"
        );
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
