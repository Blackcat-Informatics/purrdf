// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! ABSORPTION: the terminology, clausified in the hypertableau's own style.
//!
//! A general concept inclusion `C ⊑ D` has two possible encodings for a completion-graph
//! search, and the whole cost of a terminology is which one it gets.
//!
//! * **Internalized.** `nnf(¬C ⊔ D)` is placed in EVERY abstract node's label
//!   ([`Kb::meta`](crate::owl_dl::Kb::meta)), where its `⊔`-clause branches on every node of
//!   every completion whether or not anything made the axiom relevant. One axiom, one
//!   don't-know case split per node per branch — multiplicative in the ontology's size.
//! * **Absorbed.** `guard(x…) → D(x)` is a GUARDED CLAUSE: hyperresolution derives `D`
//!   exactly where the guard matches, deterministically, with no case split at all.
//!
//! This module decides, per inclusion, which one it gets — and the answer is a clause set,
//! not a special case for named classes. The incumbent absorbed exactly `A ⊑ D` for a single
//! named `A`; that is the DEGENERATE case of the criterion below (a one-atom guard), and it
//! is the reason a terminology written with `∃r.C ⊑ D`, `A ⊓ B ⊑ D`, `{a} ⊑ D` or the
//! `⊤ ⊑ D` that every `rdfs:range` axiom is paid for every one of those axioms on every node.
//!
//! # The faithful-antecedent criterion
//!
//! An inclusion `C ⊑ D` may become the guarded clause `guard(x…) → D(x)` only when
//!
//! > `C^I ⊆ { x | guard matches at x }`
//!
//! in the interpretation `I` READ OFF a clash-free completion graph — labels interpreting the
//! named classes, edges and filler labels interpreting the existentials, and
//! [`Node::nominals`](crate::owl_dl::graph::Node::nominals) interpreting the nominals. That is
//! the COMPLETENESS direction, and it is the one that is easy to get wrong: a guard that
//! matches too few elements silently withholds the axiom on the rest, and the search then
//! reports a model of something the ontology forbids. (The soundness direction — the guard
//! matching only elements of `C^I` — holds of every form below by the same reading, since each
//! is the read-off interpretation's own definition of its concept.)
//!
//! | antecedent | faithful? | why |
//! |---|---|---|
//! | `⊤` | yes | `⊤^I` is the whole ABSTRACT domain, and an unguarded clause fires at every abstract node |
//! | `A` (named) | yes | `A^I` IS `{ x | A ∈ L(x) }` — that is what reading a model off a graph means |
//! | `{a}` (singleton nominal) | yes | `{a}^I` is the node denoting `a`, which [`BodyAtom::Denotes`] tests |
//! | `∃r.F`, `≥1 r.F` with `F` faithful | yes | `x ∈ (∃r.F)^I` iff some `r`-neighbour of `x` is in `F^I`, which the edge join is |
//! | `C₁ ⊓ … ⊓ Cₙ`, every `Cᵢ` faithful | yes | the conjunction of the guards |
//! | `∀r.F` | **no** | a node with no `r`-edge is in `(∀r.F)^I` and nothing in the graph says so |
//! | `≤n r.F` | **no** | likewise: the restriction holds by ABSENCE of successors |
//! | `≥n r.F`, `n ≥ 2` | **no** | an `∃r.F` guard matches strictly more elements than `(≥n r.F)^I` |
//! | `¬A`, `¬{a}`, `¬∃r.Self` | **no** | a label that does not contain `A` does not mean `¬A` holds — the search has not decided |
//! | `C₁ ⊔ … ⊔ Cₙ` | **no** (bare) | a disjunctive guard is not a conjunction of atoms; SPLIT instead |
//! | `∃r.Self` | **no** | faithful in fact, but deliberately outside the criterion: see below |
//! | data range | **no** | its extension is the CONCRETE domain, which no abstract guard ranges over |
//!
//! `∃r.Self` is the one form this table refuses that the criterion would admit — a node's own
//! `r`-loop is exactly its membership in `(∃r.Self)^I`. It is left out because the criterion is
//! the thing being defended and a form nothing in the corpora exercises is a form nothing would
//! catch. Refusing a faithful antecedent costs absorption and never soundness, which is the
//! direction this module is allowed to be wrong in.
//!
//! ## What is NOT the criterion, and was refuted
//!
//! A tempting reading of `¬A ⊑ D` is "fire it on every node whose label carries `¬A`". It is
//! WRONG in the completeness direction: `(¬A)^I` is every element the completion graph did not
//! put `A` on, and only some of those carry the decided `¬A` — so the axiom would go
//! unenforced on the rest and the search would report a model where none exists. Nothing
//! resembling a negative guard appears below; a negative antecedent is internalized, which is
//! what makes the `⊔`-branch decide it.
//!
//! # The dispositions, per shape
//!
//! Two rewrites run to a fixpoint first, because they turn axioms nothing absorbs into axioms
//! that absorb:
//!
//! | rewrite | from | to |
//! |---|---|---|
//! | conjunctive consequent | `C ⊑ D ⊓ E` | `C ⊑ D`, `C ⊑ E` |
//! | disjunctive antecedent | `C ⊔ D ⊑ E` | `C ⊑ E`, `D ⊑ E` |
//! | enumerated antecedent | `{a, b} ⊑ E` | `{a} ⊑ E`, `{b} ⊑ E` |
//! | trivial | `⊥ ⊑ D`, `C ⊑ ⊤`, `C ⊑ C` | dropped — valid in every interpretation |
//!
//! and then each surviving inclusion is disposed of:
//!
//! | inclusion | disposition |
//! |---|---|
//! | `⊤ ⊑ ∀r.C` (`rdfs:range`) | `r(x,y) → C(y)` — NOTHING enters a label |
//! | `⊤ ⊑ D` otherwise | `→ D(x)`, an unguarded clause over the abstract nodes |
//! | `A ⊑ D` | `A(x) → D(x)` — the incumbent's whole absorption, as one guard atom |
//! | `A₁ ⊓ … ⊓ Aₙ ⊑ D` | `A₁(x) ∧ … ∧ Aₙ(x) → D(x)` |
//! | `{a} ⊑ D` | `denotes_a(x) → D(x)` |
//! | `∃r.C ⊑ D` (`rdfs:domain` at `C = ⊤`) | `C(y) ∧ r⁻(y,x) → D(x)`, RE-ROOTED at the filler |
//! | `C₁ ⊓ C₂ ⊑ D`, `C₁` faithful, `C₂` not | `guard(C₁) → D ⊔ nnf(¬C₂)` — PARTIAL absorption |
//! | anything else | internalized as `nnf(¬C ⊔ D)` |
//!
//! ## Re-rooting, and why it is what makes `∃r.C ⊑ D` cheap
//!
//! A guard is a TREE of variables: concept and nominal atoms constrain a variable, and each
//! `∃r.F` introduces a child variable reached by `r`. The inclusion constrains the tree's
//! HEAD variable — the one `C`'s own extension is about — but a clause is matched by binding
//! variable `0` and joining outward, and
//! [`ClauseSet`](crate::owl_dl::clause::ClauseSet) can only INDEX a clause whose first body
//! atom is a concept atom on variable `0`. Authored head-first, `∃r.C ⊑ D` opens with a role
//! atom and lands in the untriggered set, retried at every node of every round.
//!
//! So the tree is re-rooted at the first variable carrying a named-class atom, and the path
//! back to the head variable is walked through [`Role::inverse`]: `∃r.C ⊑ D` is authored
//! `C(x₀) ∧ r⁻(x₀, x₁) → D(x₁)`, which the concept `C` triggers and which fires only where
//! `C` actually holds. `∃r.∃s.B ⊑ D` roots at `B` and inverts twice. An antecedent with no
//! named class anywhere — `⊤ ⊑ D`, `∃r.⊤ ⊑ D`, `{a} ⊑ D` — has nothing to trigger on and stays
//! untriggered by necessity; [`crate::owl_dl::clause`] states the bound that makes that
//! affordable.
//!
//! # Determinism
//!
//! The inclusions are processed in [`Kb::tbox`](crate::owl_dl::Kb::tbox) order, which is parse
//! order; the rewrite worklist is a FIFO, so the derived inclusions keep that order; a guard
//! tree is built in the antecedent's own pre-order and emitted in a depth-first walk from a
//! root chosen by first position. Nothing is read out of a hash map, so the clause table — and
//! therefore the branch order of every search over it — is a pure function of the parse.

use std::collections::VecDeque;

use crate::owl_dl::clause::{BodyAtom, Var};
use crate::owl_dl::concept::{Concept, ConceptTable, Decomp, Role};

/// One clause of the ABSORBED TBox: `⋀ body → head(head_var)`.
///
/// The head is ONE concept id on ONE variable, which is what keeps every absorbed clause
/// deterministic: a disjunctive consequent is interned as a `⊔` concept and the branch is the
/// `⊔`-clause the concept table already derives for it, so absorption never introduces a case
/// split of its own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GuardedClause {
    /// The guard, in the order the matcher joins it: variable `0` is bound by the caller and
    /// every later atom either filters a bound variable or binds one through a role.
    pub(crate) body: Vec<BodyAtom>,
    /// The variable the consequent is asserted on — not always `0`, because a re-rooted guard
    /// reaches the constrained node through an inverse edge.
    pub(crate) head_var: Var,
    /// The consequent concept id.
    pub(crate) head: u32,
}

/// How a knowledge base's general concept inclusions are encoded for the search.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Encoding {
    /// Clausify every faithful antecedent; internalize what is left. The encoding every
    /// decision this crate takes runs under.
    Absorbing,
    /// Internalize EVERY inclusion and absorb none — the textbook encoding, kept reachable
    /// only so a test can decide one knowledge base both ways and compare the verdicts.
    #[cfg(test)]
    Internalizing,
}

/// The two encodings of one TBox, as the search consumes them.
#[derive(Debug, Default)]
pub(crate) struct Absorption {
    /// The guarded clauses, in derivation order.
    pub(crate) clauses: Vec<GuardedClause>,
    /// The internalized remainder: meta-concept ids seeded into every abstract node's label.
    pub(crate) meta: Vec<u32>,
}

/// Clausify `tbox` under `encoding`, interning whatever concepts the dispositions derive.
///
/// `poll` is the caller's work boundary, taken once per inclusion in each of the two passes,
/// so a terminology large enough to be worth absorbing is also large enough to be stopped
/// while it is being absorbed.
///
/// The concept table's negation cache must be populated ([`ConceptTable::finalize`]): partial
/// absorption negates the conjuncts it could not guard, and a conjunct with no cached negation
/// makes the whole inclusion internalized rather than a panic — one fewer absorbed axiom,
/// never an invented guard.
pub(crate) fn absorb<E>(
    table: &mut ConceptTable,
    tbox: &[(u32, u32)],
    top: u32,
    bottom: u32,
    encoding: Encoding,
    mut poll: impl FnMut() -> Result<(), E>,
) -> Result<Absorption, E> {
    let mut out = Absorption::default();
    match encoding {
        Encoding::Absorbing => {
            for (sub, sup) in rewrite(table, tbox, top, bottom, &mut poll)? {
                poll()?;
                dispose(table, &mut out, sub, sup, top);
            }
        }
        #[cfg(test)]
        Encoding::Internalizing => {
            for &(sub, sup) in tbox {
                poll()?;
                internalize(table, &mut out, sub, sup, top);
            }
        }
    }
    Ok(out)
}

/// The two structural rewrites, run to a fixpoint — see the disposition table in the
/// [module docs](self).
///
/// A FIFO worklist rather than a recursion, because one rewrite feeds the other: splitting
/// `C ⊔ D ⊑ E ⊓ F` on its consequent produces two inclusions whose antecedents then split
/// again. Each step strictly shrinks the syntax tree of the pair it replaces, so the queue
/// drains.
fn rewrite<E>(
    table: &mut ConceptTable,
    tbox: &[(u32, u32)],
    top: u32,
    bottom: u32,
    poll: &mut impl FnMut() -> Result<(), E>,
) -> Result<Vec<(u32, u32)>, E> {
    /// Which rewrite one inclusion admits.
    enum Split {
        /// The consequent is a `⊓`: one inclusion per conjunct.
        Consequent(Vec<u32>),
        /// The antecedent is a `⊔`: one inclusion per disjunct.
        Antecedent(Vec<u32>),
        /// The antecedent enumerates several individuals: one inclusion per member.
        Members(Vec<u32>),
        /// Terminal.
        None,
    }

    let mut pending: VecDeque<(u32, u32)> = tbox.iter().copied().collect();
    let mut out: Vec<(u32, u32)> = Vec::new();
    while let Some((sub, sup)) = pending.pop_front() {
        poll()?;
        // Valid in every interpretation, so it constrains no completion graph.
        if sub == bottom || sup == top || sub == sup {
            continue;
        }
        let split = if let Decomp::And(conjuncts) = table.decomp(sup) {
            Split::Consequent(conjuncts.clone())
        } else {
            match table.decomp(sub) {
                Decomp::Or(disjuncts) => Split::Antecedent(disjuncts.clone()),
                Decomp::Nominal(members) if members.len() > 1 => Split::Members(members.clone()),
                _ => Split::None,
            }
        };
        match split {
            Split::Consequent(conjuncts) => {
                for conjunct in conjuncts {
                    pending.push_back((sub, conjunct));
                }
            }
            Split::Antecedent(disjuncts) => {
                for disjunct in disjuncts {
                    pending.push_back((disjunct, sup));
                }
            }
            Split::Members(members) => {
                for member in members {
                    let singleton = table.intern(Concept::Nominal(vec![member]));
                    pending.push_back((singleton, sup));
                }
            }
            Split::None => out.push((sub, sup)),
        }
    }
    Ok(out)
}

/// Dispose of one rewritten inclusion: a guarded clause where the antecedent is faithful, the
/// internalized disjunction where it is not.
fn dispose(table: &mut ConceptTable, out: &mut Absorption, sub: u32, sup: u32, top: u32) {
    // `rdfs:range`: `⊤ ⊑ ∀r.C` becomes the edge clause, so the universal never enters a label
    // and never widens a blocking signature. Blocking then blocks strictly more, which
    // withholds work and never a derivation — see the blocking notes in
    // [`crate::owl_dl::hyper`].
    if sub == top
        && let Decomp::All(role, filler) = *table.decomp(sup)
    {
        // `⊤ ⊑ ∀r.⊤` holds of every interpretation, and the clause for it would be an
        // untriggered edge scan whose head is always satisfied.
        if filler == top {
            return;
        }
        out.clauses.push(GuardedClause {
            body: vec![BodyAtom::Role {
                from: 0,
                to: 1,
                role,
            }],
            head_var: 1,
            head: filler,
        });
        return;
    }
    let Some((tree, residue)) = guard_of(table, sub) else {
        internalize(table, out, sub, sup, top);
        return;
    };
    let head = if residue.is_empty() {
        sup
    } else {
        // PARTIAL absorption: `C₁ ⊓ C₂ ⊑ D` is `C₁ ⊑ D ⊔ ¬C₂`, so the conjuncts no guard
        // covers move to the consequent negated.
        let Some(head) = residual(table, sup, &residue) else {
            internalize(table, out, sub, sup, top);
            return;
        };
        head
    };
    out.clauses.push(emit(&tree, head));
}

/// Record `nnf(¬sub ⊔ sup)` as an internalized meta-concept.
///
/// A meta-concept that normalizes to `⊤` constrains nothing, and seeding `⊤` into every label
/// would put an id in every blocking signature for an axiom that says nothing, so it is
/// dropped rather than recorded.
fn internalize(table: &mut ConceptTable, out: &mut Absorption, sub: u32, sup: u32, top: u32) {
    let antecedent = table.concept(sub).clone();
    let consequent = table.concept(sup).clone();
    let meta = Concept::Or(vec![Concept::Not(Box::new(antecedent)), consequent]);
    let id = table.intern(meta);
    if id != top {
        out.meta.push(id);
    }
}

/// The consequent of a PARTIAL absorption: `sup ⊔ ⋁ ¬residue`.
///
/// `None` when a residual conjunct has no cached negation, which makes the caller internalize
/// the whole inclusion instead.
fn residual(table: &mut ConceptTable, sup: u32, residue: &[u32]) -> Option<u32> {
    let mut members: Vec<Concept> = vec![table.concept(sup).clone()];
    for &conjunct in residue {
        members.push(table.concept(table.negation(conjunct)?).clone());
    }
    Some(table.intern(Concept::Or(members)))
}

/// One variable of a guard tree: the atoms constraining it, and the edges to the variables it
/// introduces.
#[derive(Debug, Clone, Default)]
struct GuardNode {
    /// Named-class concept ids that must be in this variable's label.
    concepts: Vec<u32>,
    /// Individuals this variable must denote.
    denotes: Vec<u32>,
    /// `(role, index)` — each child is a `role`-successor of this variable.
    children: Vec<(Role, usize)>,
    /// The `(role, index)` this variable was reached BY; `None` for the head variable.
    parent: Option<(Role, usize)>,
}

/// The guard tree of `sub`, plus the conjuncts of it that are NOT faithful.
///
/// `None` when nothing about `sub` is absorbable. A conjunction absorbs conjunct by conjunct —
/// which is what makes partial absorption possible — while any other antecedent is
/// all-or-nothing, because there is no sound way to guard PART of an existential or a
/// negation.
fn guard_of(table: &ConceptTable, sub: u32) -> Option<(Vec<GuardNode>, Vec<u32>)> {
    let mut nodes = vec![GuardNode::default()];
    if let Decomp::And(conjuncts) = table.decomp(sub) {
        let mut residue: Vec<u32> = Vec::new();
        let mut guarded = false;
        for &conjunct in conjuncts {
            // On a trial copy: a conjunct that fails part-way through leaves atoms behind, and
            // a guard holding half of an existential would match nodes the antecedent excludes.
            let mut trial = nodes.clone();
            if extend(&mut trial, 0, table, conjunct) {
                nodes = trial;
                guarded = true;
            } else {
                residue.push(conjunct);
            }
        }
        return guarded.then_some((nodes, residue));
    }
    extend(&mut nodes, 0, table, sub).then_some((nodes, Vec::new()))
}

/// Constrain variable `at` to `id`'s extension, allocating a variable per existential.
///
/// Answers whether `id` is FAITHFUL — the criterion in the [module docs](self) — and the atoms
/// left behind on a `false` are the caller's to discard.
fn extend(nodes: &mut Vec<GuardNode>, at: usize, table: &ConceptTable, id: u32) -> bool {
    match *table.decomp(id) {
        // `⊤` constrains nothing: an unguarded clause fires at every abstract node, which is
        // exactly `⊤`'s extension.
        Decomp::Top => true,
        Decomp::Named => {
            nodes[at].concepts.push(id);
            true
        }
        // A multi-member enumeration is a DISJUNCTION of guards, which `rewrite` has already
        // split into singletons; one that reaches here anyway is refused rather than read as
        // a conjunction of its members.
        Decomp::Nominal(ref members) if members.len() == 1 => {
            nodes[at].denotes.push(members[0]);
            true
        }
        Decomp::And(ref conjuncts) => conjuncts
            .iter()
            .all(|&conjunct| extend(nodes, at, table, conjunct)),
        // `≥1 r.F` IS `∃r.F`; `≥n r.F` for `n ≥ 2` is not, and an `∃`-shaped guard for it
        // would match elements the antecedent excludes.
        Decomp::Some(role, filler) | Decomp::Min(1, role, filler) => {
            let child = nodes.len();
            nodes.push(GuardNode {
                parent: Some((role, at)),
                ..GuardNode::default()
            });
            nodes[at].children.push((role, child));
            extend(nodes, child, table, filler)
        }
        _ => false,
    }
}

/// Order a guard tree into a matchable clause, re-rooted at the first variable carrying a
/// named-class atom.
///
/// The walk is depth-first from that root over the tree read UNDIRECTED: a child edge keeps
/// its role and the parent edge is inverted, so every variable is bound by the atom that
/// reaches it and the head variable is wherever the walk arrives at the tree's own root.
fn emit(nodes: &[GuardNode], head: u32) -> GuardedClause {
    let root = nodes
        .iter()
        .position(|node| !node.concepts.is_empty())
        .unwrap_or(0);
    let mut body: Vec<BodyAtom> = Vec::new();
    let mut var_of: Vec<Option<Var>> = vec![None; nodes.len()];
    var_of[root] = Some(0);
    let mut next: Var = 1;
    walk(nodes, root, &mut body, &mut var_of, &mut next);
    GuardedClause {
        body,
        head_var: var_of[0].expect("the walk reaches every variable of a tree"),
        head,
    }
}

/// Emit `at`'s own atoms, then bind and walk every variable it is joined to.
fn walk(
    nodes: &[GuardNode],
    at: usize,
    body: &mut Vec<BodyAtom>,
    var_of: &mut Vec<Option<Var>>,
    next: &mut Var,
) {
    let var = var_of[at].expect("a variable is numbered before it is walked");
    for &concept in &nodes[at].concepts {
        body.push(BodyAtom::Concept { var, concept });
    }
    for &individual in &nodes[at].denotes {
        body.push(BodyAtom::Denotes { var, individual });
    }
    // Children keep their role; the edge back to the parent is the same edge read from the
    // other end, which is what [`Role::inverse`] is.
    for &(role, other) in &nodes[at].children {
        join(nodes, var, role, other, body, var_of, next);
    }
    if let Some((role, other)) = nodes[at].parent {
        join(nodes, var, role.inverse(), other, body, var_of, next);
    }
}

/// Bind `other` through a `role`-edge from `var` and continue the walk, unless the walk has
/// already been there — which, over a tree, is only ever the variable it arrived from.
fn join(
    nodes: &[GuardNode],
    var: Var,
    role: Role,
    other: usize,
    body: &mut Vec<BodyAtom>,
    var_of: &mut Vec<Option<Var>>,
    next: &mut Var,
) {
    if var_of[other].is_some() {
        return;
    }
    let to = *next;
    var_of[other] = Some(to);
    *next += 1;
    body.push(BodyAtom::Role {
        from: var,
        to,
        role,
    });
    walk(nodes, other, body, var_of, next);
}

#[cfg(test)]
mod tests {
    use super::{Encoding, GuardedClause, absorb};
    use crate::owl_dl::clause::BodyAtom;
    use crate::owl_dl::concept::{Concept, ConceptTable, Role};

    /// A class term id.
    const A: u32 = 10;
    /// A second class term id.
    const B: u32 = 11;
    /// A third class term id.
    const C: u32 = 12;
    /// A role term id.
    const R: u32 = 20;
    /// An individual term id.
    const IND: u32 = 30;

    /// Absorb one inclusion over a fresh table, answering the clauses, the meta ids, and the
    /// table itself — so an assertion can name the very concept ids the pass emitted.
    fn absorbed(sub: Concept, sup: Concept) -> (Vec<GuardedClause>, Vec<u32>, ConceptTable) {
        let mut table = ConceptTable::default();
        let top = table.top();
        let bottom = table.bottom();
        let sub_id = table.intern(sub);
        let sup_id = table.intern(sup);
        table.finalize();
        let out = absorb(
            &mut table,
            &[(sub_id, sup_id)],
            top,
            bottom,
            Encoding::Absorbing,
            || Ok::<(), std::convert::Infallible>(()),
        )
        .expect("an infallible poll cannot stop the pass");
        table.finalize();
        (out.clauses, out.meta, table)
    }

    /// `A ⊑ B` — the incumbent's whole absorption, now the one-atom degenerate case.
    #[test]
    fn a_named_antecedent_becomes_a_one_atom_guard() {
        let (clauses, meta, mut table) = absorbed(Concept::Named(A), Concept::Named(B));
        assert!(meta.is_empty(), "a named antecedent internalizes nothing");
        assert_eq!(clauses.len(), 1);
        let a = table.intern(Concept::Named(A));
        let b = table.intern(Concept::Named(B));
        assert_eq!(
            clauses[0],
            GuardedClause {
                body: vec![BodyAtom::Concept { var: 0, concept: a }],
                head_var: 0,
                head: b,
            }
        );
    }

    /// `∃r.C ⊑ D` is authored RE-ROOTED at the filler, so the filler concept TRIGGERS it and
    /// the axiom's own node is reached through `r⁻`. Authored head-first it would open with a
    /// role atom and be retried at every node of every round.
    #[test]
    fn an_existential_antecedent_is_rerooted_at_its_filler() {
        let (clauses, meta, mut table) = absorbed(
            Concept::Some(Role::Named(R), Box::new(Concept::Named(C))),
            Concept::Named(A),
        );
        assert!(meta.is_empty());
        assert_eq!(clauses.len(), 1);
        let c = table.intern(Concept::Named(C));
        let a = table.intern(Concept::Named(A));
        assert_eq!(
            clauses[0],
            GuardedClause {
                body: vec![
                    BodyAtom::Concept { var: 0, concept: c },
                    BodyAtom::Role {
                        from: 0,
                        to: 1,
                        role: Role::Inv(R),
                    },
                ],
                head_var: 1,
                head: a,
            }
        );
    }

    /// `∃r.∃r.C ⊑ D` inverts TWICE: the walk leaves the filler, steps back to the middle
    /// variable and then to the constrained one, so the head lands two edges away.
    #[test]
    fn a_nested_existential_antecedent_inverts_the_whole_path() {
        let (clauses, _, mut table) = absorbed(
            Concept::Some(
                Role::Named(R),
                Box::new(Concept::Some(Role::Named(R), Box::new(Concept::Named(C)))),
            ),
            Concept::Named(A),
        );
        let c = table.intern(Concept::Named(C));
        assert_eq!(clauses.len(), 1);
        assert_eq!(
            clauses[0].body,
            vec![
                BodyAtom::Concept { var: 0, concept: c },
                BodyAtom::Role {
                    from: 0,
                    to: 1,
                    role: Role::Inv(R),
                },
                BodyAtom::Role {
                    from: 1,
                    to: 2,
                    role: Role::Inv(R),
                },
            ]
        );
        assert_eq!(clauses[0].head_var, 2);
    }

    /// `rdfs:domain` is `∃r.⊤ ⊑ D`: nothing in the antecedent names a class, so the clause
    /// stays rooted at the node it constrains and opens with the role atom.
    #[test]
    fn a_domain_axiom_stays_rooted_at_the_node_it_constrains() {
        let (clauses, meta, mut table) = absorbed(
            Concept::Some(Role::Named(R), Box::new(Concept::Top)),
            Concept::Named(A),
        );
        assert!(meta.is_empty());
        let a = table.intern(Concept::Named(A));
        assert_eq!(
            clauses,
            vec![GuardedClause {
                body: vec![BodyAtom::Role {
                    from: 0,
                    to: 1,
                    role: Role::Named(R),
                }],
                head_var: 0,
                head: a,
            }]
        );
    }

    /// `rdfs:range` is `⊤ ⊑ ∀r.C`, and it becomes the edge clause outright — NOTHING enters a
    /// node label, so no blocking signature widens for it.
    #[test]
    fn a_range_axiom_becomes_an_edge_clause_and_seeds_no_label() {
        let (clauses, meta, mut table) = absorbed(
            Concept::Top,
            Concept::All(Role::Named(R), Box::new(Concept::Named(C))),
        );
        assert!(meta.is_empty(), "a range axiom internalizes nothing");
        let c = table.intern(Concept::Named(C));
        assert_eq!(
            clauses,
            vec![GuardedClause {
                body: vec![BodyAtom::Role {
                    from: 0,
                    to: 1,
                    role: Role::Named(R),
                }],
                head_var: 1,
                head: c,
            }]
        );
    }

    /// A conjunctive consequent SPLITS, and each half is absorbed on its own — which is what
    /// turns `A ≡ B ⊓ C` into two guarded clauses instead of one branch point per node.
    #[test]
    fn a_conjunctive_consequent_splits() {
        let (clauses, meta, _) = absorbed(
            Concept::Named(A),
            Concept::And(vec![Concept::Named(B), Concept::Named(C)]),
        );
        assert!(meta.is_empty());
        assert_eq!(clauses.len(), 2, "one clause per conjunct: {clauses:?}");
        assert!(clauses.iter().all(|clause| clause.body.len() == 1));
    }

    /// A disjunctive antecedent SPLITS, so `B ⊔ C ⊑ A` absorbs where the bare disjunction
    /// could not be guarded at all.
    #[test]
    fn a_disjunctive_antecedent_splits() {
        let (clauses, meta, _) = absorbed(
            Concept::Or(vec![Concept::Named(B), Concept::Named(C)]),
            Concept::Named(A),
        );
        assert!(meta.is_empty());
        assert_eq!(clauses.len(), 2, "one clause per disjunct: {clauses:?}");
    }

    /// A multi-member enumeration is a disjunction of singletons, and splits like one.
    #[test]
    fn an_enumerated_antecedent_splits_into_one_clause_per_member() {
        let (clauses, meta, _) = absorbed(Concept::nominal(vec![IND, IND + 1]), Concept::Named(A));
        assert!(meta.is_empty());
        assert_eq!(clauses.len(), 2);
        assert!(
            clauses
                .iter()
                .all(|clause| matches!(clause.body.as_slice(), [BodyAtom::Denotes { var: 0, .. }]))
        );
    }

    /// PARTIAL absorption: the faithful conjunct guards, and the one nothing can guard moves
    /// to the consequent negated. The clause is still deterministic — the branch is the
    /// `⊔`-clause of the derived consequent, at the nodes the guard reached.
    #[test]
    fn a_mixed_conjunction_absorbs_partially() {
        let unguardable = Concept::All(Role::Named(R), Box::new(Concept::Named(B)));
        let (clauses, meta, mut table) = absorbed(
            Concept::And(vec![Concept::Named(A), unguardable.clone()]),
            Concept::Named(C),
        );
        assert!(meta.is_empty(), "the inclusion absorbed: {meta:?}");
        assert_eq!(clauses.len(), 1);
        let a = table.intern(Concept::Named(A));
        assert_eq!(
            clauses[0].body,
            vec![BodyAtom::Concept { var: 0, concept: a }]
        );
        let expected = table.intern(
            Concept::Or(vec![Concept::Named(C), Concept::Not(Box::new(unguardable))]).nnf(),
        );
        assert_eq!(clauses[0].head, expected, "the head is D ⊔ ¬C₂");
    }

    /// A NEGATIVE antecedent is internalized, never guarded — the refuted design. A label
    /// that does not carry `A` does not mean `¬A` was decided, so a `¬A`-triggered clause
    /// would leave the axiom unenforced on every node the search never decided.
    #[test]
    fn a_negative_antecedent_is_internalized() {
        let (clauses, meta, _) =
            absorbed(Concept::Not(Box::new(Concept::Named(A))), Concept::Named(B));
        assert!(clauses.is_empty(), "no guard for ¬A: {clauses:?}");
        assert_eq!(meta.len(), 1);
    }

    /// The three antecedents that hold by ABSENCE — `∀r.C`, `≤n r.C` and `≥2 r.C` — are all
    /// internalized. Guarding any of them would match too few nodes.
    #[test]
    fn the_absence_shaped_antecedents_are_internalized() {
        for sub in [
            Concept::All(Role::Named(R), Box::new(Concept::Named(A))),
            Concept::Max(1, Role::Named(R), Box::new(Concept::Named(A))),
            Concept::Min(2, Role::Named(R), Box::new(Concept::Named(A))),
        ] {
            let (clauses, meta, _) = absorbed(sub.clone(), Concept::Named(B));
            assert!(clauses.is_empty(), "{sub:?} was guarded: {clauses:?}");
            assert_eq!(meta.len(), 1, "{sub:?}");
        }
    }

    /// A VALID inclusion constrains no completion graph, so it produces neither a clause nor a
    /// meta-concept — which is what keeps `⊥ ⊑ D` from seeding `⊤` into every label.
    #[test]
    fn a_valid_inclusion_produces_nothing() {
        for (sub, sup) in [
            (Concept::Bottom, Concept::Named(A)),
            (Concept::Named(A), Concept::Top),
            (Concept::Named(A), Concept::Named(A)),
        ] {
            let (clauses, meta, _) = absorbed(sub.clone(), sup.clone());
            assert!(clauses.is_empty() && meta.is_empty(), "{sub:?} ⊑ {sup:?}");
        }
    }

    /// The INTERNALIZING encoding absorbs nothing at all — the second encoding the oracle's
    /// differential decides every generated knowledge base under.
    #[test]
    fn the_internalizing_encoding_absorbs_nothing() {
        let mut table = ConceptTable::default();
        let top = table.top();
        let bottom = table.bottom();
        let sub = table.intern(Concept::Named(A));
        let sup = table.intern(Concept::Named(B));
        table.finalize();
        let out = absorb(
            &mut table,
            &[(sub, sup)],
            top,
            bottom,
            Encoding::Internalizing,
            || Ok::<(), std::convert::Infallible>(()),
        )
        .expect("infallible");
        assert!(out.clauses.is_empty());
        assert_eq!(out.meta.len(), 1);
    }
}
