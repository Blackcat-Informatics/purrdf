// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! SLG-tabled backward resolution over Horn(+negation) programs of compound
//! terms, with a three-valued well-founded-semantics (WFS) verdict for every atom
//! touched and a checkable proof for every `True` answer.
//!
//! # Backward, not forward
//!
//! [`crate::seminaive`] answers "what is the whole model" by materialising it.
//! This module answers a narrower, cheaper question — "does this one goal hold,
//! and why" — by working from the goal toward the facts that support it,
//! DEMANDING a subgoal only when some clause body actually needs it. The
//! grounding phase (`ground`) is exactly the SLG "answer clause resolution"
//! idea: a positive body literal is never re-solved by native recursion, it is
//! looked up in a per-call-pattern TABLE that is itself grown round by round,
//! which is what turns what would otherwise be unbounded SLD recursion into a
//! terminating, memoized fixpoint — the same discipline [`crate::seminaive`]
//! applies forward, applied here backward and driven by demand instead of by a
//! stratum.
//!
//! # Three-valued, not two
//!
//! A goal that recurses through its own negation — `p :- not(p)` — has no
//! two-valued model. The van Gelder alternating fixpoint (`well_founded`)
//! computes the WFS's `True`/`Undefined` split (and, by absence, `False`) exactly
//! this way, over the GROUND rule instances `ground` collected, and
//! [`FolOutcome::truth_of`] is the three-valued verdict a caller reads it back
//! through.
//!
//! # Every `True` answer carries a checkable proof
//!
//! [`FolProof`] is deliberately not hash-consed into the term arena and does not
//! carry a content-addressed rule identity — unlike its upstream ancestor. This
//! crate already has an answer to "a small, independently checkable proof
//! structure that names its rule by a plain index": [`crate::proof::ProofArena`].
//! There is no RDF-projection need here that would justify a digest-addressed
//! rule name, so [`FolProof::ByRule`] simply carries a `usize` rule index, and
//! [`check_fol_proof`] RE-DERIVES the stated conclusion from the premises and the
//! named clause exactly as [`crate::proof::ProofArena::check`] does — a step the
//! rule does not license fails to check however well-formed the record of it is.
//!
//! # No caller-supplied sort context
//!
//! [`crate::unify::SortContext`] threads an order-sorted discipline through
//! unification, but nothing in this crate currently needs order-sorted
//! GOAL-DIRECTED resolution, so every unification this module performs uses
//! [`crate::unify::SortContext::default`] internally — a deliberate scope
//! reduction versus an upstream that threaded a caller-supplied context through.
//! A future caller that needs one can extend [`FolProgram`]; today, doing so
//! would only be plumbing nobody exercises.
//!
//! # The bridge to this crate's own Datalog IR
//!
//! [`solve_datalog_goal`] lowers a flat, atomic-head-only [`crate::clause`]
//! program into this module's compound-term representation and answers one goal
//! by SLG resolution instead of the forward semi-naive fixpoint — so this
//! capability is real, tested machinery over the crate's actual data, not a
//! standalone algorithm nobody calls.

use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use crate::clause::{ClauseAtom, ClauseTerm, DlClause, NonDatalogClause};
use crate::id::{MetaId, NodeId};
use crate::term::{NodeData, TermDag};
use crate::unify::{self, SortContext, Subst, Unified};

/// A clause body literal cannot exceed this many atoms.
///
/// [`solve_body`] recurses one native stack frame per body literal, so this
/// ceiling bounds recursion depth per clause firing; [`resolve_fol`] refuses a
/// wider clause up front, before any grounding happens, with
/// [`FolUnsupported::ClauseBodyTooWide`].
const MAX_BODY_LITERALS: usize = 64;

// ── Public program IR ─────────────────────────────────────────────────────────────

/// One body literal: a positive atom, or a negation-as-failure filter over one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FolLit {
    /// An ordinary positive body atom.
    Pos(NodeId),
    /// A negation-as-failure filter: the clause fires only where this atom is
    /// not derivable.
    Neg(NodeId),
}

impl FolLit {
    /// The atom this literal carries, regardless of polarity.
    fn atom(self) -> NodeId {
        match self {
            Self::Pos(atom) | Self::Neg(atom) => atom,
        }
    }
}

/// One Horn(+negation) clause over compound terms: `head :- body`.
#[derive(Debug, Clone)]
pub struct FolClause {
    /// The clause head — a single atom (this module has no head disjunction or
    /// existential; those belong to [`crate::clause`]'s richer DL-clause shape).
    pub head: NodeId,
    /// The body literals, in authored order.
    pub body: Vec<FolLit>,
    /// The clause's index in AUTHORED program order.
    ///
    /// A plain index, not a content-addressed identity — see the [module
    /// docs](self) for why this deliberately differs from an upstream that
    /// minted a digest-addressed rule-firing IRI.
    pub rule: usize,
}

/// A program: a clause set, a goal atom, and the goal's own named variables.
#[derive(Debug, Clone)]
pub struct FolProgram {
    /// The clause set, in authored order (an authored [`FolClause::rule`] index
    /// addresses this order).
    pub clauses: Vec<FolClause>,
    /// The atom [`resolve_fol`] is asked to decide.
    pub goal: NodeId,
    /// The goal's own metavariables, paired with the name they were authored
    /// under, in first-occurrence order — this is what lets an answer's
    /// substitution be projected back to named bindings rather than raw
    /// [`NodeId`]s.
    pub goal_vars: Vec<(NodeId, String)>,
}

/// A three-valued well-founded-semantics verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Truth {
    /// The atom is in the well-founded model's true set.
    True,
    /// The atom is confirmed absent from every ground rule this resolution
    /// reached — the closed-world default.
    False,
    /// The atom depends on its own negation (directly or through a cycle) and
    /// the well-founded semantics assigns it neither truth value.
    Undefined,
}

/// A proof tree over [`FolClause`]s.
///
/// Not hash-consed into the term arena and not content-addressed by digest — see
/// the [module docs](self) for why this deliberately differs from its upstream
/// ancestor: this crate already has [`crate::proof::ProofArena`] for that job,
/// and there is no RDF-projection need here to justify duplicating it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FolProof {
    /// `goal` follows from `premises` (one per POSITIVE body literal, in
    /// authored body order) by the clause at `rule`.
    ByRule {
        /// The producing clause's authored index.
        rule: usize,
        /// The STATED conclusion — compared against, never trusted, by
        /// [`check_fol_proof`].
        goal: NodeId,
        /// One proof per positive body literal, in authored body order.
        premises: Vec<Self>,
    },
}

/// One projected answer: the goal's named variables bound to rendered values,
/// the fully-ground goal instance, and its proof.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolBinding {
    /// Goal variable name → its rendered value under this answer.
    pub bindings: BTreeMap<String, String>,
    /// The fully-ground goal instance this answer is for.
    pub atom: NodeId,
    /// A checkable proof of `atom`.
    pub proof: FolProof,
}

/// Whether a resolution ran to completion or was cut short by its step budget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FolStatus {
    /// Grounding reached its own fixpoint before the budget was spent.
    Complete,
    /// The step budget was exhausted before grounding reached a fixpoint. See
    /// `well_founded` for the soundness demotion this triggers: negation is no
    /// longer trusted, so nothing dependent on it can be reported [`Truth::True`].
    Partial,
}

/// The decided outcome of [`resolve_fol`]: every projected answer, the
/// completion status, and the three-valued verdict for any atom via
/// [`Self::truth_of`].
#[derive(Debug, Clone)]
pub struct FolOutcome {
    /// Every distinct, fully-ground answer to the program's goal, sorted for a
    /// deterministic order.
    pub answers: Vec<FolBinding>,
    /// Whether grounding ran to completion or was budget-cut.
    pub status: FolStatus,
    /// Canon-keys of every atom confirmed NOT false (the alternating fixpoint's
    /// `not_false` set).
    not_false: BTreeSet<String>,
    /// Canon-keys of every atom in the well-founded model's true set.
    true_set: BTreeSet<String>,
}

impl FolOutcome {
    /// The three-valued verdict for `node`.
    pub fn truth_of(&self, dag: &TermDag, node: NodeId) -> Truth {
        let key = canon(dag, node);
        if self.true_set.contains(&key) {
            Truth::True
        } else if self.not_false.contains(&key) {
            Truth::Undefined
        } else {
            Truth::False
        }
    }

    /// Whether `node` is confirmed NOT false — the adapter [`check_fol_proof`]'s
    /// caller-supplied `not_false` predicate is built from.
    pub fn confirms_not_false(&self, dag: &TermDag, node: NodeId) -> bool {
        self.not_false.contains(&canon(dag, node))
    }
}

/// Why [`resolve_fol`] could not decide the program at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FolUnsupported {
    /// A negative body literal's instantiated atom is still non-ground under
    /// every selectable order — the safe-computation rule found no safe literal
    /// to select at some point in the resolution.
    Floundering,
    /// A clause body exceeds `MAX_BODY_LITERALS`.
    ClauseBodyTooWide {
        /// The offending clause's authored index.
        rule: usize,
        /// Its actual body length.
        literals: usize,
    },
}

/// What [`resolve_fol`] returns: a decided outcome, or a typed reason it could
/// not decide the program at all.
#[derive(Debug, Clone)]
pub enum FolControl {
    /// The program was resolved (possibly only partially grounded — see
    /// [`FolOutcome::status`]).
    Decided(Box<FolOutcome>),
    /// The program could not be resolved at all.
    Unsupported(FolUnsupported),
}

/// The step budget [`resolve_fol`]'s grounding phase is charged against.
#[derive(Debug, Clone, Copy)]
pub struct FolBudget {
    /// The maximum amount of WORK grounding may do before it is cut short and the
    /// outcome demoted to [`FolStatus::Partial`].
    ///
    /// A work unit is one candidate answer unified against a body literal, or one newly
    /// committed ground rule instance. Both are charged, because either alone leaves the
    /// other unbounded: an earlier revision counted commits only and tested the total
    /// between expansion rounds, so a single round could enumerate an entire
    /// cross-product before the budget was ever consulted. On a rule whose head is all
    /// variables — the shape a meta-rule table needs, where the predicate is carried as
    /// data — that round does not finish, and a budget that cannot fire is not a budget.
    pub max_steps: u64,
}

// ── Rendering and canonical content keys ────────────────────────────────────────

/// A human-facing rendering of `node`.
///
/// `op(arg1, arg2, ...)` for an application, the interned symbol text for a leaf
/// or free variable, `_b{debruijn}.{slot}` for a bound occurrence, and
/// `op[sort1,sort2,...].{body}` for a binder. This is NOT injective over content
/// containing commas or brackets and is therefore never used as a dedup or
/// identity key — only [`canon`] is. It exists solely to give a [`FolBinding`]'s
/// values and a [`FolProofError`] diagnostic something readable.
pub fn render(dag: &TermDag, node: NodeId) -> String {
    match dag.data(node) {
        NodeData::Leaf(sym) | NodeData::Free(sym) => dag.symbol(*sym).to_owned(),
        NodeData::Meta(m) => format!("_m{}", m.index()),
        NodeData::Bound { debruijn, slot } => format!("_b{debruijn}.{slot}"),
        NodeData::App { op, args } => {
            let op_text = render(dag, *op);
            let arg_text: Vec<String> = args.iter().map(|&arg| render(dag, arg)).collect();
            format!("{op_text}({})", arg_text.join(", "))
        }
        NodeData::Binder { op, sorts, body } => {
            let op_text = render(dag, *op);
            let sort_text: Vec<String> = sorts.iter().map(|&sort| render(dag, sort)).collect();
            format!("{op_text}[{}].{}", sort_text.join(","), render(dag, *body))
        }
    }
}

/// Length-prefix `s` into `out`, so no concatenation of two variable-length
/// fields can be confused with a different split of the same bytes.
fn frame(out: &mut String, s: &str) {
    write!(out, "{}:{}", s.len(), s).expect("writing to a String never fails");
}

/// The canonical content key of `node`: the "call pattern" the tabling engine
/// indexes on.
///
/// Every DISTINCT metavariable `node` mentions is numbered in FIRST-VISIT order
/// starting from 0, so two structurally-identical-up-to-variable-naming atoms
/// produce the SAME key. Every variable-length field (a symbol's text, a
/// number's decimal rendering) is length-prefixed via `frame`, mirroring
/// [`crate::proof::ProofArena::encode`]'s wire discipline, so distinct shapes can
/// never collide through concatenation ambiguity.
pub fn canon(dag: &TermDag, node: NodeId) -> String {
    let mut numbering: Vec<MetaId> = Vec::new();
    let mut out = String::new();
    canon_at(dag, node, &mut numbering, &mut out);
    out
}

/// The recursive core of [`canon`].
fn canon_at(dag: &TermDag, node: NodeId, numbering: &mut Vec<MetaId>, out: &mut String) {
    match dag.data(node) {
        NodeData::Leaf(sym) => {
            out.push('L');
            frame(out, dag.symbol(*sym));
        }
        NodeData::Free(sym) => {
            out.push('F');
            frame(out, dag.symbol(*sym));
        }
        NodeData::Meta(m) => {
            let index = match numbering.iter().position(|candidate| candidate == m) {
                Some(index) => index,
                None => {
                    numbering.push(*m);
                    numbering.len() - 1
                }
            };
            out.push('M');
            frame(out, &index.to_string());
        }
        NodeData::Bound { debruijn, slot } => {
            out.push('B');
            frame(out, &debruijn.to_string());
            frame(out, &slot.to_string());
        }
        NodeData::App { op, args } => {
            out.push('A');
            frame(out, &args.len().to_string());
            canon_at(dag, *op, numbering, out);
            for &arg in args {
                canon_at(dag, arg, numbering, out);
            }
        }
        NodeData::Binder { op, sorts, body } => {
            out.push('D');
            frame(out, &sorts.len().to_string());
            canon_at(dag, *op, numbering, out);
            for &sort in sorts {
                canon_at(dag, sort, numbering, out);
            }
            canon_at(dag, *body, numbering, out);
        }
    }
}

// ── The tabling engine (private) ────────────────────────────────────────────────

/// One committed ground rule instance.
#[derive(Debug, Clone)]
struct GroundRule {
    /// The ground head atom.
    head: NodeId,
    /// The ground positive premises, in authored body order.
    pos: Vec<NodeId>,
    /// The ground negative atoms, in authored body order.
    neg: Vec<NodeId>,
    /// The producing clause's authored index.
    rule: usize,
}

/// The SLG tabling state: demanded calls, their tabled answers, and every ground
/// rule instance committed so far.
#[derive(Debug, Default)]
struct Engine {
    /// Call key → (answer key → ground answer atom): each demanded call's
    /// currently-tabled answers.
    tables: BTreeMap<String, BTreeMap<String, NodeId>>,
    /// Call key → the (possibly partially bound) demanded call node.
    calls: BTreeMap<String, NodeId>,
    /// Ground-rule key → the committed ground rule.
    ground_rules: BTreeMap<String, GroundRule>,
    /// Answer/atom key → the ground atom node — a shared dictionary across every
    /// table.
    atoms: BTreeMap<String, NodeId>,
    /// Total ground rule instances committed so far, charged against a
    /// [`FolBudget`].
    steps: u64,
    /// Whether grounding was cut short by the step budget.
    exhausted: bool,
    /// Whether a negative body literal ever had no safe selection order.
    floundered: bool,
}

impl Engine {
    /// Register `atom` as demanded, returning its call key. A call already
    /// demanded under the same key keeps its ORIGINAL call node.
    fn register_call(&mut self, dag: &TermDag, atom: NodeId) -> String {
        let key = canon(dag, atom);
        if let Entry::Vacant(slot) = self.calls.entry(key.clone()) {
            slot.insert(atom);
            // A NEW demanded call is work. Charging only answer unifications leaves this
            // dimension free, and it is the one that grows without bound when a body atom
            // carries free variables: early rounds demand ever more general call patterns
            // while no answers exist yet, so nothing is charged and the fixpoint recedes.
            self.steps += 1;
        }
        key
    }

    /// Record `atom` (keyed by `atom_key`) as an answer to `call_key`.
    fn record_answer(&mut self, call_key: &str, atom_key: String, atom: NodeId) {
        let table = self.tables.entry(call_key.to_owned()).or_default();
        table.entry(atom_key).or_insert(atom);
    }
}

/// The union of every distinct [`MetaId`] any node in `nodes` mentions, sorted
/// and deduplicated.
fn distinct_metas(dag: &TermDag, nodes: &[NodeId]) -> Vec<MetaId> {
    let mut acc: Vec<MetaId> = Vec::new();
    for &node in nodes {
        acc.extend_from_slice(dag.free_meta(node));
    }
    acc.sort_unstable();
    acc.dedup();
    acc
}

/// Can `head` possibly unify with `call`, judged WITHOUT freshening either?
///
/// A cheap, SOUND pre-filter. [`expand_round`] freshens every clause against every
/// demanded call before trying to unify, and freshening mints a metavariable per distinct
/// variable and rebuilds the clause's nodes — so on a rule table it is the dominant cost,
/// paid in full for pairs that could never have matched.
///
/// Two distinct interned constants never unify, so comparing argument positions where BOTH
/// sides are already `Leaf` rejects a pair for free. Anything else — a metavariable on
/// either side, a nested application, a differing arity — returns `true` and is decided by
/// the real unifier, so this can only skip work, never an answer.
fn may_unify(dag: &TermDag, head: NodeId, call: NodeId) -> bool {
    let (
        NodeData::App {
            op: hop,
            args: hargs,
        },
        NodeData::App {
            op: cop,
            args: cargs,
        },
    ) = (dag.data(head), dag.data(call))
    else {
        return true;
    };
    if hargs.len() != cargs.len() {
        return false;
    }
    if let (NodeData::Leaf(h), NodeData::Leaf(c)) = (dag.data(*hop), dag.data(*cop))
        && h != c
    {
        return false;
    }
    for (&h, &c) in hargs.iter().zip(cargs.iter()) {
        if let (NodeData::Leaf(hs), NodeData::Leaf(cs)) = (dag.data(h), dag.data(c))
            && hs != cs
        {
            return false;
        }
    }
    true
}

/// Freshen `clause`: mint a brand-new metavariable for every distinct
/// metavariable its head and body mention, and return the renamed head and body.
///
/// Every clause firing needs its own fresh variables — two simultaneous uses of
/// the same authored clause must not share a binding just because they share
/// authored variable names.
fn freshen_clause(dag: &mut TermDag, clause: &FolClause) -> (NodeId, Vec<FolLit>) {
    let mut nodes: Vec<NodeId> = vec![clause.head];
    nodes.extend(clause.body.iter().map(|lit| lit.atom()));
    let metas = distinct_metas(dag, &nodes);

    let mut renaming = Subst::new();
    for old in metas {
        let (_, fresh) = dag.fresh_meta();
        renaming.bind_renaming(old, fresh);
    }

    let head = unify::apply(dag, &renaming, clause.head);
    let body = clause
        .body
        .iter()
        .map(|lit| match lit {
            FolLit::Pos(atom) => FolLit::Pos(unify::apply(dag, &renaming, *atom)),
            FolLit::Neg(atom) => FolLit::Neg(unify::apply(dag, &renaming, *atom)),
        })
        .collect();
    (head, body)
}

/// Whether `atom` is fully ground under `s` — i.e. has no remaining free
/// (unbound) metavariable once `s` is applied.
fn is_ground(dag: &mut TermDag, s: &Subst, atom: NodeId) -> bool {
    let applied = unify::apply(dag, s, atom);
    dag.free_meta(applied).is_empty()
}

/// Enumerate every way `body` can be satisfied against the CURRENTLY tabled
/// answers, extending `subst`.
///
/// At each step, the first (ascending original index) unselected literal that
/// is SAFE to select is chosen: a positive literal is always safe; a negative
/// literal is safe only once it is fully ground under the accumulated
/// substitution. A selected positive literal is instantiated, demanded as a
/// call, and joined against every one of that call's answers CURRENTLY on
/// file — never by re-solving it natively — which is what makes this a
/// round-based fixpoint rather than unbounded SLD recursion. A selected
/// negative literal is simply carried forward (negation is decided later, in
/// the well-founded fixpoint): selection continues over the remaining literals
/// with the SAME substitution. If no remaining literal is safe, the clause
/// floundered.
fn solve_body(
    dag: &mut TermDag,
    engine: &mut Engine,
    body: &[FolLit],
    selected: &mut [bool],
    subst: &Subst,
    cap: u64,
    out: &mut Vec<Subst>,
) {
    // THE SEARCH IS WHERE THE WORK IS. Charging only committed rules bounds the OUTPUT
    // and leaves the enumeration that produces it unbounded, which is how a fully-variable
    // rule head turns one round into a cross-product over every constant in the program.
    if engine.steps >= cap {
        engine.exhausted = true;
        return;
    }
    let remaining: Vec<usize> = (0..body.len()).filter(|&i| !selected[i]).collect();
    if remaining.is_empty() {
        out.push(subst.clone());
        return;
    }

    let mut chosen = None;
    for &i in &remaining {
        let safe = match &body[i] {
            FolLit::Pos(_) => true,
            FolLit::Neg(atom) => is_ground(dag, subst, *atom),
        };
        if safe {
            chosen = Some(i);
            break;
        }
    }
    let Some(i) = chosen else {
        engine.floundered = true;
        return;
    };

    selected[i] = true;
    match &body[i] {
        FolLit::Pos(atom) => {
            let instantiated = unify::apply(dag, subst, *atom);
            let call_key = engine.register_call(dag, instantiated);
            let answers: Vec<NodeId> = engine
                .tables
                .get(&call_key)
                .map(|table| table.values().copied().collect())
                .unwrap_or_default();
            let ctx = SortContext::default();
            for answer in answers {
                if engine.steps >= cap {
                    engine.exhausted = true;
                    break;
                }
                engine.steps += 1;
                let mut candidate = subst.clone();
                if matches!(
                    unify::unify_sorted(dag, instantiated, answer, &mut candidate, &ctx),
                    Unified::Ok
                ) {
                    solve_body(dag, engine, body, selected, &candidate, cap, out);
                }
            }
        }
        FolLit::Neg(_) => {
            solve_body(dag, engine, body, selected, subst, cap, out);
        }
    }
    selected[i] = false;
}

/// One newly-derivable ground rule instance a single [`expand_round`] pass found.
struct Produced {
    /// The call this instance answers (and therefore tables its head under).
    call_key: String,
    /// The producing clause's authored index.
    rule: usize,
    /// The ground head atom.
    head: NodeId,
    /// The ground positive premises, in authored body order.
    pos: Vec<NodeId>,
    /// The ground negative atoms, in authored body order.
    neg: Vec<NodeId>,
}

/// One grounding round: cross every currently-demanded call against every
/// program clause, and return every newly-derivable ground rule instance found.
fn expand_round(
    dag: &mut TermDag,
    engine: &mut Engine,
    program: &FolProgram,
    cap: u64,
) -> Vec<Produced> {
    let mut produced = Vec::new();
    let call_keys: Vec<(String, NodeId)> = engine
        .calls
        .iter()
        .map(|(key, &node)| (key.clone(), node))
        .collect();

    for (call_key, call_node) in call_keys {
        if engine.exhausted {
            break;
        }
        for (rule_idx, clause) in program.clauses.iter().enumerate() {
            if engine.exhausted {
                break;
            }
            // Reject impossible pairs BEFORE paying for freshening — the dominant cost on
            // a rule table, and pure waste for a clause whose head cannot match this call.
            if !may_unify(dag, clause.head, call_node) {
                continue;
            }
            let (head, body) = freshen_clause(dag, clause);
            let mut base = Subst::new();
            let ctx = SortContext::default();
            if !matches!(
                unify::unify_sorted(dag, head, call_node, &mut base, &ctx),
                Unified::Ok
            ) {
                continue;
            }

            let mut selected = vec![false; body.len()];
            let mut solutions = Vec::new();
            solve_body(
                dag,
                engine,
                &body,
                &mut selected,
                &base,
                cap,
                &mut solutions,
            );

            for subst in &solutions {
                let ground_head = unify::apply(dag, subst, head);
                if !dag.free_meta(ground_head).is_empty() {
                    continue;
                }
                let mut pos = Vec::with_capacity(body.len());
                let mut neg = Vec::new();
                let mut all_ground = true;
                for lit in &body {
                    let ground_atom = unify::apply(dag, subst, lit.atom());
                    if !dag.free_meta(ground_atom).is_empty() {
                        all_ground = false;
                        break;
                    }
                    match lit {
                        FolLit::Pos(_) => pos.push(ground_atom),
                        FolLit::Neg(_) => neg.push(ground_atom),
                    }
                }
                if !all_ground {
                    continue;
                }
                produced.push(Produced {
                    call_key: call_key.clone(),
                    rule: rule_idx,
                    head: ground_head,
                    pos,
                    neg,
                });
            }
        }
    }
    produced
}

/// The content key of one ground rule instance: `(head, pos atoms, neg atoms,
/// rule index)`, length-prefixed throughout.
fn ground_rule_key(
    dag: &TermDag,
    head: NodeId,
    pos: &[NodeId],
    neg: &[NodeId],
    rule: usize,
) -> String {
    let mut out = String::new();
    frame(&mut out, &canon(dag, head));
    frame(&mut out, &pos.len().to_string());
    for &p in pos {
        frame(&mut out, &canon(dag, p));
    }
    frame(&mut out, &neg.len().to_string());
    for &n in neg {
        frame(&mut out, &canon(dag, n));
    }
    frame(&mut out, &rule.to_string());
    out
}

/// Run [`expand_round`] to a fixpoint (or until `budget` is spent), seeding the
/// goal as the first demanded call.
///
/// Terminates when a round commits nothing new AND demands no new call — a
/// round that only demands a new call must still be given one more pass, since
/// a recursive rule's positive premise can only be satisfied once that call has
/// itself accumulated an answer. Returns `true` if the resolution floundered.
fn ground(dag: &mut TermDag, engine: &mut Engine, program: &FolProgram, budget: FolBudget) -> bool {
    let goal_key = canon(dag, program.goal);
    engine.calls.entry(goal_key).or_insert(program.goal);

    loop {
        let calls_before = engine.calls.len();
        let ground_rules_before = engine.ground_rules.len();

        if engine.steps > budget.max_steps {
            engine.exhausted = true;
            return false;
        }
        let produced = expand_round(dag, engine, program, budget.max_steps);
        if engine.floundered {
            return true;
        }

        let mut newly_committed: u64 = 0;
        for item in produced {
            let key = ground_rule_key(dag, item.head, &item.pos, &item.neg, item.rule);
            if let Entry::Vacant(entry) = engine.ground_rules.entry(key) {
                entry.insert(GroundRule {
                    head: item.head,
                    pos: item.pos,
                    neg: item.neg,
                    rule: item.rule,
                });
                newly_committed += 1;
            }
            let head_key = canon(dag, item.head);
            engine.atoms.entry(head_key.clone()).or_insert(item.head);
            engine.record_answer(&item.call_key, head_key, item.head);
        }

        engine.steps += newly_committed;
        if engine.exhausted || engine.steps > budget.max_steps {
            engine.exhausted = true;
            return false;
        }

        let calls_after = engine.calls.len();
        let ground_rules_after = engine.ground_rules.len();
        if newly_committed == 0
            && calls_after == calls_before
            && ground_rules_after == ground_rules_before
        {
            break;
        }
    }
    false
}

// ── The well-founded model ──────────────────────────────────────────────────────

/// The van Gelder immediate-consequence operator over the ground rule set: the
/// least model of the residual program obtained by keeping only ground rules
/// whose every negative atom's key is ABSENT from `s`, then computing the
/// ordinary positive bottom-up fixpoint.
fn gamma(dag: &TermDag, engine: &Engine, s: &BTreeSet<String>) -> BTreeSet<String> {
    let mut model: BTreeSet<String> = BTreeSet::new();
    loop {
        let mut changed = false;
        for rule in engine.ground_rules.values() {
            let head_key = canon(dag, rule.head);
            if model.contains(&head_key) {
                continue;
            }
            let neg_ok = rule.neg.iter().all(|&n| !s.contains(&canon(dag, n)));
            if !neg_ok {
                continue;
            }
            let pos_ok = rule.pos.iter().all(|&p| model.contains(&canon(dag, p)));
            if pos_ok {
                model.insert(head_key);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    model
}

/// The least model of the negation-FREE subset of the ground rule set — the
/// sound fallback used when grounding was budget-truncated (see
/// `well_founded`).
fn positive_least_model(dag: &TermDag, engine: &Engine) -> BTreeSet<String> {
    let mut model: BTreeSet<String> = BTreeSet::new();
    loop {
        let mut changed = false;
        for rule in engine.ground_rules.values() {
            if !rule.neg.is_empty() {
                continue;
            }
            let head_key = canon(dag, rule.head);
            if model.contains(&head_key) {
                continue;
            }
            if rule.pos.iter().all(|&p| model.contains(&canon(dag, p))) {
                model.insert(head_key);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    model
}

/// The well-founded model: `(true_set, not_false)`.
///
/// When grounding ran to completion, this is the genuine alternating fixpoint
/// `w = Γ(Γ(w))`. When it was budget-cut, negation cannot be trusted — a ground
/// rule this resolution never reached might have flipped a negative literal's
/// verdict — so `true_set` is demoted to the negation-free
/// [`positive_least_model`], and `not_false` is that set UNION every ground atom
/// actually reached, so nothing reached is ever reported definitely `False` and
/// no rule relying on negation can found a `True` answer under an incomplete
/// grounding.
fn well_founded(
    dag: &TermDag,
    engine: &Engine,
    exhausted: bool,
) -> (BTreeSet<String>, BTreeSet<String>) {
    if exhausted {
        let w = positive_least_model(dag, engine);
        let mut not_false = w.clone();
        for rule in engine.ground_rules.values() {
            not_false.insert(canon(dag, rule.head));
        }
        for key in engine.atoms.keys() {
            not_false.insert(key.clone());
        }
        (w, not_false)
    } else {
        let mut w: BTreeSet<String> = BTreeSet::new();
        loop {
            let inner = gamma(dag, engine, &w);
            let next = gamma(dag, engine, &inner);
            if next == w {
                break;
            }
            w = next;
        }
        let not_false = gamma(dag, engine, &w);
        (w, not_false)
    }
}

/// Build a [`FolProof`] for every ground atom in `w`, in a fixpoint over the
/// ground rule set (a ground rule may cite another ground rule's head as a
/// premise in either direction, so this repeats passes until no new proof is
/// added rather than assuming any particular order).
///
/// Every negative atom is independently re-checked against `not_false` here —
/// deliberately not simply trusted from `w`'s own membership, so a defect in
/// `w`'s computation cannot silently launder into a proof.
fn build_proofs(
    dag: &TermDag,
    engine: &Engine,
    w: &BTreeSet<String>,
    not_false: &BTreeSet<String>,
) -> BTreeMap<String, FolProof> {
    let mut proofs: BTreeMap<String, FolProof> = BTreeMap::new();
    loop {
        let mut changed = false;
        for rule in engine.ground_rules.values() {
            let head_key = canon(dag, rule.head);
            if proofs.contains_key(&head_key) || !w.contains(&head_key) {
                continue;
            }
            let neg_ok = rule
                .neg
                .iter()
                .all(|&n| !not_false.contains(&canon(dag, n)));
            if !neg_ok {
                continue;
            }
            let mut premises = Vec::with_capacity(rule.pos.len());
            let mut all_ready = true;
            for &p in &rule.pos {
                match proofs.get(&canon(dag, p)) {
                    Some(proof) => premises.push(proof.clone()),
                    None => {
                        all_ready = false;
                        break;
                    }
                }
            }
            if !all_ready {
                continue;
            }
            proofs.insert(
                head_key,
                FolProof::ByRule {
                    rule: rule.rule,
                    goal: rule.head,
                    premises,
                },
            );
            changed = true;
        }
        if !changed {
            break;
        }
    }
    proofs
}

/// Project every ground atom in `w` against `program.goal`, keeping only the
/// candidates under which the goal becomes FULLY ground, deduplicating by
/// content key and sorting for a deterministic answer order.
fn project(
    dag: &mut TermDag,
    engine: &Engine,
    program: &FolProgram,
    w: &BTreeSet<String>,
    proofs: &BTreeMap<String, FolProof>,
) -> Vec<FolBinding> {
    let mut candidates: Vec<(String, NodeId)> = engine
        .atoms
        .iter()
        .filter(|(key, _)| w.contains(*key))
        .map(|(key, &atom)| (key.clone(), atom))
        .collect();
    candidates.sort();

    let mut answers: Vec<FolBinding> = Vec::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    for (atom_key, atom) in candidates {
        let mut subst = Subst::new();
        let ctx = SortContext::default();
        if !matches!(
            unify::unify_sorted(dag, program.goal, atom, &mut subst, &ctx),
            Unified::Ok
        ) {
            continue;
        }
        let applied_goal = unify::apply(dag, &subst, program.goal);
        if !dag.free_meta(applied_goal).is_empty() {
            continue;
        }
        let content_key = canon(dag, applied_goal);
        if !seen.insert(content_key) {
            continue;
        }
        let Some(proof) = proofs.get(&atom_key) else {
            // Provably unreachable given `build_proofs` ran over the same `w`;
            // excluded defensively rather than trusted.
            continue;
        };

        let mut bindings = BTreeMap::new();
        for (var_node, name) in &program.goal_vars {
            let value = unify::apply(dag, &subst, *var_node);
            bindings.insert(name.clone(), render(dag, value));
        }
        answers.push(FolBinding {
            bindings,
            atom: applied_goal,
            proof: proof.clone(),
        });
    }

    answers.sort_by(|a, b| {
        a.bindings
            .cmp(&b.bindings)
            .then_with(|| a.atom.cmp(&b.atom))
    });
    answers
}

// ── Entry point ──────────────────────────────────────────────────────────────────

/// Decide `program`'s goal by SLG-tabled backward resolution under three-valued
/// well-founded semantics.
///
/// See the [module docs](self) for the algorithm's shape. Every clause body is
/// checked against `MAX_BODY_LITERALS` BEFORE any grounding happens, so a
/// program this module cannot represent is refused up front rather than after
/// partial work.
pub fn resolve_fol(dag: &mut TermDag, program: &FolProgram, budget: &FolBudget) -> FolControl {
    for (index, clause) in program.clauses.iter().enumerate() {
        if clause.body.len() > MAX_BODY_LITERALS {
            return FolControl::Unsupported(FolUnsupported::ClauseBodyTooWide {
                rule: index,
                literals: clause.body.len(),
            });
        }
    }

    let mut engine = Engine::default();
    let floundered = ground(dag, &mut engine, program, *budget);
    if floundered || engine.floundered {
        return FolControl::Unsupported(FolUnsupported::Floundering);
    }

    let (true_set, not_false) = well_founded(dag, &engine, engine.exhausted);
    let status = if engine.exhausted {
        FolStatus::Partial
    } else {
        FolStatus::Complete
    };
    let proofs = build_proofs(dag, &engine, &true_set, &not_false);
    let answers = project(dag, &engine, program, &true_set, &proofs);

    FolControl::Decided(Box::new(FolOutcome {
        answers,
        status,
        not_false,
        true_set,
    }))
}

// ── The independent checker ─────────────────────────────────────────────────────

/// Why a [`FolProof`] is not a proof of the conclusion it states.
///
/// Every variant is a NORMAL rejection of an invalid proof, never an engine
/// fault — mirroring [`crate::proof::ProofError`]'s own doctrine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FolProofError {
    /// The proof cites a clause index the program does not have.
    UnknownRule {
        /// The cited clause index.
        rule: usize,
    },
    /// The conclusion the checker DERIVED does not match the conclusion the
    /// proof STATED.
    HeadMismatch {
        /// The cited clause index.
        rule: usize,
        /// The rendered conclusion the checker derived.
        derived: String,
        /// The rendered conclusion the proof claimed.
        stated: String,
    },
    /// The proof supplies a premise count differing from the clause's positive
    /// body arity.
    PremiseCountMismatch {
        /// The cited clause index.
        rule: usize,
        /// The clause's positive body arity.
        body: usize,
        /// The number of premises the proof supplies.
        premises: usize,
    },
    /// A positive body literal does not unify with the corresponding premise's
    /// re-derived atom.
    PremiseMismatch {
        /// The cited clause index.
        rule: usize,
        /// The unmatched literal's position in the clause's AUTHORED body.
        position: usize,
    },
    /// A negative body literal's instantiated atom is not confirmed absent from
    /// the caller-supplied `not_false` set, so the negation is not licensed.
    NegatedPremiseNotExcluded {
        /// The cited clause index.
        rule: usize,
        /// The unlicensed literal's position in the clause's AUTHORED body.
        position: usize,
    },
}

/// Independently check `proof` against `clauses`, RE-DERIVING its conclusion
/// rather than trusting the one it states.
///
/// `not_false` answers "is this ground atom confirmed not false" — a caller
/// backs it with the SAME well-founded computation [`resolve_fol`] produced, via
/// [`FolOutcome::confirms_not_false`].
///
/// Every premise is checked FIRST, recursively bottom-up; the clause is
/// freshened exactly as `ground` freshens it, its positive body literals are
/// unified against the checked premises' re-derived atoms into one shared
/// substitution, its negative body literals are re-decided against
/// `not_false` under that substitution, and the clause head is instantiated
/// under the final substitution and compared — never assumed — against the
/// proof's stated `goal`.
pub fn check_fol_proof(
    dag: &mut TermDag,
    proof: &FolProof,
    clauses: &[FolClause],
    not_false: &impl Fn(&TermDag, NodeId) -> bool,
) -> Result<NodeId, FolProofError> {
    let FolProof::ByRule {
        rule,
        goal,
        premises,
    } = proof;
    let clause = clauses
        .get(*rule)
        .ok_or(FolProofError::UnknownRule { rule: *rule })?;

    let positive_count = clause
        .body
        .iter()
        .filter(|lit| matches!(lit, FolLit::Pos(_)))
        .count();
    if premises.len() != positive_count {
        return Err(FolProofError::PremiseCountMismatch {
            rule: *rule,
            body: positive_count,
            premises: premises.len(),
        });
    }

    let mut derived_premises = Vec::with_capacity(premises.len());
    for premise in premises {
        derived_premises.push(check_fol_proof(dag, premise, clauses, not_false)?);
    }

    let (head, body) = freshen_clause(dag, clause);
    let mut subst = Subst::new();
    let ctx = SortContext::default();
    let mut premise_iter = derived_premises.into_iter();
    for (position, lit) in body.iter().enumerate() {
        if let FolLit::Pos(atom) = lit {
            let derived_atom = premise_iter
                .next()
                .expect("premise count already matched the positive body arity");
            if !matches!(
                unify::unify_sorted(dag, *atom, derived_atom, &mut subst, &ctx),
                Unified::Ok
            ) {
                return Err(FolProofError::PremiseMismatch {
                    rule: *rule,
                    position,
                });
            }
        }
    }

    // A clause variable that occurs ONLY in the head (a genuinely universal
    // fact variable, e.g. the repeated `Y` in `add(z, Y, Y) :- .`, or a
    // never-constrained "wildcard" position) is not bound by any premise
    // unification above. Unifying the freshened head against the STATED goal —
    // which is always fully ground here, since `resolve_fol` only ever proves
    // fully-ground atoms — is what supplies that binding; a goal the clause
    // (under the premises already checked) cannot actually produce fails this
    // unification outright, which is exactly a `HeadMismatch`.
    if !matches!(
        unify::unify_sorted(dag, head, *goal, &mut subst, &ctx),
        Unified::Ok
    ) {
        let derived_head = unify::apply(dag, &subst, head);
        return Err(FolProofError::HeadMismatch {
            rule: *rule,
            derived: render(dag, derived_head),
            stated: render(dag, *goal),
        });
    }

    for (position, lit) in body.iter().enumerate() {
        if let FolLit::Neg(atom) = lit {
            let ground_atom = unify::apply(dag, &subst, *atom);
            if not_false(dag, ground_atom) {
                return Err(FolProofError::NegatedPremiseNotExcluded {
                    rule: *rule,
                    position,
                });
            }
        }
    }

    Ok(unify::apply(dag, &subst, head))
}

// ── The DlClause lowering adapter ───────────────────────────────────────────────

/// Look up (or mint, on first use within `scope`) the [`NodeId`] a clause `term`
/// lowers to. A constant always lowers to the same interned leaf; a variable
/// lowers to a fresh metavariable the first time `scope` sees its name, and to
/// that same metavariable every time after — clause-scoped, exactly like a
/// [`crate::clause::ClauseTerm::Var`] itself is.
fn lower_term(
    dag: &mut TermDag,
    scope: &mut BTreeMap<String, NodeId>,
    term: &ClauseTerm,
) -> NodeId {
    match term {
        ClauseTerm::Var(name) => *scope.entry(name.clone()).or_insert_with(|| {
            let (_, node) = dag.fresh_meta();
            node
        }),
        other => {
            let surface = other
                .surface()
                .expect("a non-variable clause term always has a lexical surface");
            dag.intern_leaf(&surface)
        }
    }
}

/// Lower one [`ClauseAtom`] into the 4-ary application `"triple"(s, p, o, g)`.
fn lower_atom(
    dag: &mut TermDag,
    triple_op: NodeId,
    scope: &mut BTreeMap<String, NodeId>,
    atom: &ClauseAtom,
) -> NodeId {
    let args: Vec<NodeId> = atom
        .terms()
        .into_iter()
        .map(|term| lower_term(dag, scope, term))
        .collect();
    dag.intern_app(triple_op, args)
}

/// Lower `clauses` into [`FolClause`]s over `dag`, sharing one metavariable per
/// distinct [`ClauseTerm::Var`] name WITHIN each clause (clause-scoped, cleared
/// between clauses — Datalog clause variables never cross a clause boundary).
///
/// # Errors
///
/// Returns the first [`NonDatalogClause`] refusal for a clause whose head is not
/// [`crate::clause::HeadForm::Atomic`] — this lowering is Datalog-only, exactly
/// like [`crate::seminaive::compile`].
fn lower_datalog_clauses(
    dag: &mut TermDag,
    triple_op: NodeId,
    clauses: &[DlClause],
) -> Result<Vec<FolClause>, NonDatalogClause> {
    let mut out = Vec::with_capacity(clauses.len());
    for (index, clause) in clauses.iter().enumerate() {
        let Some(head_atom) = clause.datalog_head() else {
            return Err(NonDatalogClause::new(index, clause.head_form()));
        };
        let mut scope: BTreeMap<String, NodeId> = BTreeMap::new();
        let head = lower_atom(dag, triple_op, &mut scope, head_atom);
        let body = clause
            .body()
            .iter()
            .map(|atom| {
                let lowered = lower_atom(dag, triple_op, &mut scope, atom);
                if atom.is_negated() {
                    FolLit::Neg(lowered)
                } else {
                    FolLit::Pos(lowered)
                }
            })
            .collect();
        out.push(FolClause {
            head,
            body,
            rule: index,
        });
    }
    Ok(out)
}

/// Lower `goal` into the compound-term IR, in its OWN fresh variable scope
/// (independent of every clause's), recording every distinct
/// [`ClauseTerm::Var`] it mentions, in FIRST-OCCURRENCE order.
fn lower_goal(
    dag: &mut TermDag,
    triple_op: NodeId,
    goal: &ClauseAtom,
) -> (NodeId, Vec<(NodeId, String)>) {
    let mut scope: BTreeMap<String, NodeId> = BTreeMap::new();
    let mut order: Vec<(NodeId, String)> = Vec::new();
    let args: Vec<NodeId> = goal
        .terms()
        .into_iter()
        .map(|term| match term {
            ClauseTerm::Var(name) => {
                if let Some(&node) = scope.get(name) {
                    node
                } else {
                    let (_, node) = dag.fresh_meta();
                    scope.insert(name.clone(), node);
                    order.push((node, name.clone()));
                    node
                }
            }
            other => {
                let surface = other
                    .surface()
                    .expect("a non-variable clause term always has a lexical surface");
                dag.intern_leaf(&surface)
            }
        })
        .collect();
    (dag.intern_app(triple_op, args), order)
}

/// Lower a flat, atomic-head-only Datalog program
/// ([`crate::clause::DlClause`]) into this module's compound-term
/// representation, treating each arity-4 atom `triple(s, p, o, g)` as a 4-ary
/// application `"triple"(s, p, o, g)`, and solve `goal` by SLG-tabled backward
/// resolution instead of this crate's forward semi-naive fixpoint — genuinely
/// useful when only ONE goal's answer (and its proof) is wanted, without
/// materialising the rest of the program's model.
///
/// Returns the [`TermDag`] the answers' [`NodeId`]s and [`FolControl`] are
/// addressed against, alongside the control value itself: a [`FolBinding`]'s
/// `bindings` map is already a plain rendered string map (everything a
/// [`DlClause`]-level caller usually needs), but a caller wanting
/// [`FolOutcome::truth_of`] or to re-check a proof via [`check_fol_proof`] needs
/// the arena the [`NodeId`]s were minted in.
///
/// # Errors
///
/// Returns `Err` naming the clause index and its [`crate::clause::HeadForm`]
/// for any clause that is not [`crate::clause::HeadForm::Atomic`] — this
/// adapter is Datalog-only, exactly like [`crate::seminaive::compile`].
pub fn solve_datalog_goal(
    clauses: &[DlClause],
    goal: &ClauseAtom,
    budget: &FolBudget,
) -> Result<(TermDag, FolControl), NonDatalogClause> {
    let mut dag = TermDag::new();
    let triple_op = dag.intern_leaf("triple");

    let fol_clauses = lower_datalog_clauses(&mut dag, triple_op, clauses)?;
    let (goal_node, goal_vars) = lower_goal(&mut dag, triple_op, goal);

    let program = FolProgram {
        clauses: fol_clauses,
        goal: goal_node,
        goal_vars,
    };
    let control = resolve_fol(&mut dag, &program, budget);
    Ok((dag, control))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Fixture helpers ──────────────────────────────────────────────────────

    /// A leaf constant.
    fn leaf(dag: &mut TermDag, symbol: &str) -> NodeId {
        dag.intern_leaf(symbol)
    }

    /// An application `op(args...)`.
    fn app(dag: &mut TermDag, op: &str, args: Vec<NodeId>) -> NodeId {
        let op_node = dag.intern_leaf(op);
        dag.intern_app(op_node, args)
    }

    /// An always-`Ok` `not_false` adapter, for programs with no negation to
    /// re-check.
    fn always_not_false(_dag: &TermDag, _node: NodeId) -> bool {
        false
    }

    // ── 0: canon/render sanity ───────────────────────────────────────────────

    /// `canon` numbers each DISTINCT metavariable in FIRST-VISIT order, so two
    /// structurally-identical-up-to-variable-naming atoms share the same
    /// key — and a genuinely different variable PATTERN does not.
    #[test]
    fn canon_numbers_metas_in_first_visit_order_so_variants_share_a_key() {
        let mut dag = TermDag::new();
        let (_, x) = dag.fresh_meta();
        let (_, y) = dag.fresh_meta();
        let atom1 = app(&mut dag, "p", vec![x, y, x]);

        let (_, a) = dag.fresh_meta();
        let (_, b) = dag.fresh_meta();
        let atom2 = app(&mut dag, "p", vec![a, b, a]);
        assert_eq!(
            canon(&dag, atom1),
            canon(&dag, atom2),
            "the same repeated-variable pattern must share a call-pattern key"
        );

        let (_, c) = dag.fresh_meta();
        let atom3 = app(&mut dag, "p", vec![a, b, c]);
        assert_ne!(
            canon(&dag, atom1),
            canon(&dag, atom3),
            "a genuinely different variable pattern (no repetition) must not share the key"
        );
    }

    /// `render` is human-facing only: it is NOT injective and is never used as a
    /// dedup or identity key, but `canon` still distinguishes structurally
    /// distinct atoms.
    #[test]
    fn render_is_human_readable_but_canon_is_the_identity_key() {
        let mut dag = TermDag::new();
        let a = leaf(&mut dag, "a");
        let b = leaf(&mut dag, "b");
        let term = app(&mut dag, "p", vec![a, b]);
        assert_eq!(render(&dag, term), "p(a, b)");
        assert_eq!(render(&dag, a), "a");

        let swapped = app(&mut dag, "p", vec![b, a]);
        assert_ne!(canon(&dag, term), canon(&dag, swapped));
    }

    // ── 1: Peano addition ────────────────────────────────────────────────────

    /// A Peano numeral `s(s(...s(z)...))`, `n` deep.
    fn peano(dag: &mut TermDag, z: NodeId, n: u32) -> NodeId {
        let mut node = z;
        for _ in 0..n {
            node = app(dag, "s", vec![node]);
        }
        node
    }

    /// `add(z, Y, Y).` `add(s(X), Y, s(Z)) :- add(X, Y, Z).` — resolving
    /// `add(s(s(z)), s(z), ?R)` finds exactly one answer, the correct successor
    /// numeral, with a proof [`check_fol_proof`] accepts.
    #[test]
    fn peano_add_returns_correct_answer_with_checkable_proofs() {
        let mut dag = TermDag::new();
        let z = leaf(&mut dag, "z");

        // add(z, Y, Y) :- .
        let (_, y0) = dag.fresh_meta();
        let head0 = app(&mut dag, "add", vec![z, y0, y0]);
        let clause0 = FolClause {
            head: head0,
            body: vec![],
            rule: 0,
        };

        // add(s(X), Y, s(Z)) :- add(X, Y, Z).
        let (_, x1) = dag.fresh_meta();
        let (_, y1) = dag.fresh_meta();
        let (_, z1) = dag.fresh_meta();
        let s_x = app(&mut dag, "s", vec![x1]);
        let s_z = app(&mut dag, "s", vec![z1]);
        let head1 = app(&mut dag, "add", vec![s_x, y1, s_z]);
        let premise = app(&mut dag, "add", vec![x1, y1, z1]);
        let clause1 = FolClause {
            head: head1,
            body: vec![FolLit::Pos(premise)],
            rule: 1,
        };

        let clauses = vec![clause0, clause1];

        // Goal: add(s(s(z)), s(z), ?R).
        let two = peano(&mut dag, z, 2);
        let one = peano(&mut dag, z, 1);
        let three = peano(&mut dag, z, 3);
        let (_, r_meta) = dag.fresh_meta();
        let goal = app(&mut dag, "add", vec![two, one, r_meta]);
        let program = FolProgram {
            clauses: clauses.clone(),
            goal,
            goal_vars: vec![(r_meta, "R".to_owned())],
        };

        let budget = FolBudget { max_steps: 10_000 };
        let control = resolve_fol(&mut dag, &program, &budget);
        let FolControl::Decided(outcome) = control else {
            panic!("Peano addition must be decidable");
        };
        assert_eq!(outcome.answers.len(), 1, "exactly one sum");
        let answer = &outcome.answers[0];
        assert_eq!(answer.bindings.get("R"), Some(&render(&dag, three)));
        assert_eq!(
            outcome.truth_of(&dag, answer.atom),
            Truth::True,
            "a projected answer is always in the true set"
        );

        let checked = check_fol_proof(&mut dag, &answer.proof, &clauses, &always_not_false)
            .expect("the derivation re-derives cleanly");
        assert_eq!(checked, answer.atom);
    }

    // ── 2: member/2 over a cons list ─────────────────────────────────────────

    /// `member(X, cons(X, T)).` `member(X, cons(H, T)) :- member(X, T).` — every
    /// element of a 3-element list is found, each with a checkable proof.
    #[test]
    fn member_over_cons_lists_enumerates_elements_with_proofs() {
        let mut dag = TermDag::new();
        let a = leaf(&mut dag, "a");
        let b = leaf(&mut dag, "b");
        let c = leaf(&mut dag, "c");
        let nil = leaf(&mut dag, "nil");

        // member(X, cons(X, T)) :- .
        let (_, x0) = dag.fresh_meta();
        let (_, t0) = dag.fresh_meta(); // the wildcard tail position
        let cons_x_t = app(&mut dag, "cons", vec![x0, t0]);
        let head0 = app(&mut dag, "member", vec![x0, cons_x_t]);
        let clause0 = FolClause {
            head: head0,
            body: vec![],
            rule: 0,
        };

        // member(X, cons(H, T)) :- member(X, T).
        let (_, x1) = dag.fresh_meta();
        let (_, h1) = dag.fresh_meta(); // the wildcard head position
        let (_, t1) = dag.fresh_meta();
        let cons_h_t = app(&mut dag, "cons", vec![h1, t1]);
        let head1 = app(&mut dag, "member", vec![x1, cons_h_t]);
        let premise = app(&mut dag, "member", vec![x1, t1]);
        let clause1 = FolClause {
            head: head1,
            body: vec![FolLit::Pos(premise)],
            rule: 1,
        };
        let clauses = vec![clause0, clause1];

        // The list cons(a, cons(b, cons(c, nil))).
        let tail = app(&mut dag, "cons", vec![c, nil]);
        let mid = app(&mut dag, "cons", vec![b, tail]);
        let list = app(&mut dag, "cons", vec![a, mid]);

        let (_, x_goal) = dag.fresh_meta();
        let goal = app(&mut dag, "member", vec![x_goal, list]);
        let program = FolProgram {
            clauses: clauses.clone(),
            goal,
            goal_vars: vec![(x_goal, "X".to_owned())],
        };

        let budget = FolBudget { max_steps: 10_000 };
        let control = resolve_fol(&mut dag, &program, &budget);
        let FolControl::Decided(outcome) = control else {
            panic!("member/2 over a finite ground list must be decidable");
        };
        let mut found: Vec<String> = outcome
            .answers
            .iter()
            .map(|answer| answer.bindings.get("X").expect("X is bound").clone())
            .collect();
        found.sort();
        assert_eq!(found, vec!["a".to_owned(), "b".to_owned(), "c".to_owned()]);

        for answer in &outcome.answers {
            let checked = check_fol_proof(&mut dag, &answer.proof, &clauses, &always_not_false)
                .unwrap_or_else(|error| panic!("{error:?}"));
            assert_eq!(checked, answer.atom);
        }
    }

    // ── 3/4: well-founded three-valued negation ──────────────────────────────

    /// `win(X) :- move(X, Y), not(win(Y)).` over disjoint move chains: a chain
    /// ending in a sink yields `True`; the sink itself, never derivable, yields
    /// `False`; and a genuine self-loop yields `Undefined`.
    #[test]
    fn well_founded_three_valued_negation_is_correct() {
        fn win_rule(dag: &mut TermDag) -> FolClause {
            let (_, x) = dag.fresh_meta();
            let (_, y) = dag.fresh_meta();
            let head = app(dag, "win", vec![x]);
            let move_xy = app(dag, "move", vec![x, y]);
            let win_y = app(dag, "win", vec![y]);
            FolClause {
                head,
                body: vec![FolLit::Pos(move_xy), FolLit::Neg(win_y)],
                rule: 1,
            }
        }

        fn fact(dag: &mut TermDag, pred: &str, args: Vec<NodeId>) -> FolClause {
            let head = app(dag, pred, args);
            FolClause {
                head,
                body: vec![],
                rule: 0,
            }
        }

        fn resolve(dag: &mut TermDag, facts: Vec<FolClause>, goal_atom: NodeId) -> FolOutcome {
            let mut clauses = facts;
            clauses.push(win_rule(dag));
            let program = FolProgram {
                clauses,
                goal: goal_atom,
                goal_vars: vec![],
            };
            let budget = FolBudget { max_steps: 10_000 };
            match resolve_fol(dag, &program, &budget) {
                FolControl::Decided(outcome) => *outcome,
                FolControl::Unsupported(reason) => panic!("expected a decision, got {reason:?}"),
            }
        }

        let mut dag = TermDag::new();
        let e = leaf(&mut dag, "e");
        let f = leaf(&mut dag, "f");
        let win_e = app(&mut dag, "win", vec![e]);
        let win_f = app(&mut dag, "win", vec![f]);
        let move_ef = fact(&mut dag, "move", vec![e, f]);
        let outcome_e = resolve(&mut dag, vec![move_ef], win_e);
        assert_eq!(outcome_e.truth_of(&dag, win_e), Truth::True);
        assert_eq!(
            outcome_e.truth_of(&dag, win_f),
            Truth::False,
            "the sink is never derivable, so its default verdict is False"
        );

        let c = leaf(&mut dag, "c");
        let win_c = app(&mut dag, "win", vec![c]);
        let move_cc = fact(&mut dag, "move", vec![c, c]);
        let outcome_c = resolve(&mut dag, vec![move_cc], win_c);
        assert_eq!(
            outcome_c.truth_of(&dag, win_c),
            Truth::Undefined,
            "a genuine self-loop through negation is undefined"
        );
    }

    /// `p :- not(p).` alone is the canonical direct negative loop: `Undefined`,
    /// never `True` or `False`.
    #[test]
    fn direct_negative_loop_is_undefined_not_true_or_false() {
        let mut dag = TermDag::new();
        let p = leaf(&mut dag, "p");
        let clause = FolClause {
            head: p,
            body: vec![FolLit::Neg(p)],
            rule: 0,
        };
        let program = FolProgram {
            clauses: vec![clause],
            goal: p,
            goal_vars: vec![],
        };
        let budget = FolBudget { max_steps: 10_000 };
        let FolControl::Decided(outcome) = resolve_fol(&mut dag, &program, &budget) else {
            panic!("a direct negative loop is decidable (as Undefined), not Unsupported");
        };
        assert_eq!(outcome.truth_of(&dag, p), Truth::Undefined);
        assert!(
            outcome.answers.is_empty(),
            "an Undefined goal has no True answer"
        );
    }

    // ── 5/6: budget-cut soundness demotion ──────────────────────────────────

    /// `base :- .` `goal_atom :- base, not(other).` — a tiny `FolBudget`
    /// truncates grounding right after `goal_atom`'s own ground rule is
    /// committed; the run is `Partial`, and `goal_atom` — which depends on a
    /// negation the truncated grounding never confirmed absent — is reported
    /// `Undefined`, never `True`.
    fn budget_cut_program(dag: &mut TermDag) -> (Vec<FolClause>, NodeId, NodeId) {
        let base = leaf(dag, "base");
        let other = leaf(dag, "other");
        let goal_atom = leaf(dag, "goal_atom");
        let base_fact = FolClause {
            head: base,
            body: vec![],
            rule: 0,
        };
        let goal_rule = FolClause {
            head: goal_atom,
            body: vec![FolLit::Pos(base), FolLit::Neg(other)],
            rule: 1,
        };
        (vec![base_fact, goal_rule], base, goal_atom)
    }

    #[test]
    fn budget_cut_demotes_dependent_atom_to_undefined_not_true() {
        let mut dag = TermDag::new();
        let (clauses, base, goal_atom) = budget_cut_program(&mut dag);
        let program = FolProgram {
            clauses,
            goal: goal_atom,
            goal_vars: vec![],
        };
        // 4 work units, not 1. `max_steps` now charges every newly demanded call as well
        // as every committed rule, so 1 unit is spent before the `base` FACT is committed
        // and the run truncates ahead of the state this test is about. 4 reaches the
        // intended shape and is truer to this test's name than the old value ever was:
        // the goal comes back `Undefined` here, where 1 and 2 units yield `False`.
        let budget = FolBudget { max_steps: 4 };
        let FolControl::Decided(outcome) = resolve_fol(&mut dag, &program, &budget) else {
            panic!("a budget cut is Partial, not Unsupported");
        };
        assert_eq!(outcome.status, FolStatus::Partial);
        assert_eq!(outcome.truth_of(&dag, base), Truth::True);
        assert_eq!(
            outcome.truth_of(&dag, goal_atom),
            Truth::Undefined,
            "a truncated negation leaves the goal UNDEFINED, not merely not-True"
        );
        assert_ne!(
            outcome.truth_of(&dag, goal_atom),
            Truth::True,
            "a budget-truncated negation must never found a True answer"
        );
    }

    /// The same budget-capped program run twice yields byte-identical answer
    /// vectors — determinism holds even under truncation.
    #[test]
    fn budget_partial_answer_set_is_deterministic_across_runs() {
        let mut dag = TermDag::new();
        let (clauses, _base, goal_atom) = budget_cut_program(&mut dag);
        let program = FolProgram {
            clauses,
            goal: goal_atom,
            goal_vars: vec![],
        };
        let budget = FolBudget { max_steps: 4 };
        let FolControl::Decided(first) = resolve_fol(&mut dag, &program, &budget) else {
            panic!("expected a decision");
        };
        let FolControl::Decided(second) = resolve_fol(&mut dag, &program, &budget) else {
            panic!("expected a decision");
        };
        assert_eq!(first.answers, second.answers);
        assert_eq!(first.status, second.status);
    }

    // ── 7/8: refusals before or during grounding ────────────────────────────

    /// `p(X) :- not(q(X)).` with `X` unbound and no positive literal to ground
    /// it: the only selectable literal is an unsafe negative one, so the
    /// resolution FLOUNDERS — a typed `Unsupported`, never a fabricated answer.
    #[test]
    fn floundering_naf_goal_is_typed_unsupported_not_a_fabricated_answer() {
        let mut dag = TermDag::new();
        let (_, x) = dag.fresh_meta();
        let head = app(&mut dag, "p", vec![x]);
        let q_x = app(&mut dag, "q", vec![x]);
        let clause = FolClause {
            head,
            body: vec![FolLit::Neg(q_x)],
            rule: 0,
        };
        let (_, goal_x) = dag.fresh_meta();
        let goal = app(&mut dag, "p", vec![goal_x]);
        let program = FolProgram {
            clauses: vec![clause],
            goal,
            goal_vars: vec![(goal_x, "X".to_owned())],
        };
        let budget = FolBudget { max_steps: 10_000 };
        match resolve_fol(&mut dag, &program, &budget) {
            FolControl::Unsupported(FolUnsupported::Floundering) => {}
            other => panic!("expected Floundering, got {other:?}"),
        }
    }

    /// A synthetic clause with 65 body literals is refused by NAME before any
    /// grounding happens.
    #[test]
    fn clause_body_wider_than_64_literals_is_unsupported() {
        let mut dag = TermDag::new();
        let head = leaf(&mut dag, "wide_head");
        let body: Vec<FolLit> = (0..65)
            .map(|i| FolLit::Pos(leaf(&mut dag, &format!("lit{i}"))))
            .collect();
        let literal_count = body.len();
        let clause = FolClause {
            head,
            body,
            rule: 0,
        };
        let program = FolProgram {
            clauses: vec![clause],
            goal: head,
            goal_vars: vec![],
        };
        let budget = FolBudget { max_steps: 10_000 };
        match resolve_fol(&mut dag, &program, &budget) {
            FolControl::Unsupported(FolUnsupported::ClauseBodyTooWide { rule: 0, literals }) => {
                assert_eq!(literals, literal_count);
            }
            other => panic!("expected ClauseBodyTooWide, got {other:?}"),
        }
    }

    // ── 9/10: the proof checker's rejections ────────────────────────────────

    /// A hand-forged `FolProof::ByRule` whose stated `goal` does not match what
    /// the cited clause actually derives from its (empty) premises is rejected.
    #[test]
    fn check_fol_proof_rejects_a_forged_head() {
        let mut dag = TermDag::new();
        let (_, x) = dag.fresh_meta();
        let head = app(&mut dag, "p", vec![x]);
        let clause = FolClause {
            head,
            body: vec![],
            rule: 0,
        };
        let wrong_goal = leaf(&mut dag, "not_p_of_anything");
        let forged = FolProof::ByRule {
            rule: 0,
            goal: wrong_goal,
            premises: vec![],
        };
        let error = check_fol_proof(
            &mut dag,
            &forged,
            std::slice::from_ref(&clause),
            &always_not_false,
        )
        .expect_err("a forged head must be rejected");
        assert!(matches!(error, FolProofError::HeadMismatch { rule: 0, .. }));
    }

    /// A premise-count mismatch and a wrong (nonexistent) rule index are both
    /// rejected by name.
    #[test]
    fn check_fol_proof_rejects_a_premise_count_mismatch_and_an_unknown_rule() {
        let mut dag = TermDag::new();
        let a = leaf(&mut dag, "a");
        let head = app(&mut dag, "p", vec![a]);
        let clause = FolClause {
            head,
            body: vec![],
            rule: 0,
        };
        let clauses = vec![clause];

        let extra_premise = FolProof::ByRule {
            rule: 0,
            goal: head,
            premises: vec![FolProof::ByRule {
                rule: 0,
                goal: head,
                premises: vec![],
            }],
        };
        let error = check_fol_proof(&mut dag, &extra_premise, &clauses, &always_not_false)
            .expect_err("an extra premise on a zero-premise rule must be rejected");
        assert!(matches!(
            error,
            FolProofError::PremiseCountMismatch {
                rule: 0,
                body: 0,
                premises: 1
            }
        ));

        let unknown_rule = FolProof::ByRule {
            rule: 7,
            goal: head,
            premises: vec![],
        };
        let error = check_fol_proof(&mut dag, &unknown_rule, &clauses, &always_not_false)
            .expect_err("citing a rule the program does not have must be rejected");
        assert_eq!(error, FolProofError::UnknownRule { rule: 7 });
    }

    // ── Datalog bridge ───────────────────────────────────────────────────────

    mod datalog_bridge {
        use super::*;
        use crate::clause::{ClauseAtom, ClauseTerm, DlClause, HeadDisjunct};
        use crate::store::RelationStore;

        const P: &str = "https://example.org/p";
        const T: &str = "https://example.org/t";

        fn v(name: &str) -> ClauseTerm {
            ClauseTerm::var(name)
        }

        fn atom(subject: &str, predicate: &str, object: &str) -> ClauseAtom {
            ClauseAtom::positive(v(subject), predicate, v(object))
        }

        /// A ground fact atom over constant IRI subject/object positions — for
        /// building the fact clauses [`solve_datalog_goal`] needs, since it has
        /// no separate EDB channel of its own.
        fn ground_atom(subject: &str, predicate: &str, object: &str) -> ClauseAtom {
            ClauseAtom::positive(ClauseTerm::iri(subject), predicate, ClauseTerm::iri(object))
        }

        fn surface(name: &str) -> String {
            format!("<{name}>")
        }

        /// `t(?s, ?o) :- p(?s, ?m), p(?m, ?o).` — the same two-hop chain shape
        /// [`crate::proof`]'s own fixtures use (copied here rather than
        /// imported, since it is test-only there).
        fn chain_rules() -> Vec<DlClause> {
            vec![DlClause::datalog(
                atom("?s", T, "?o"),
                vec![atom("?s", P, "?m"), atom("?m", P, "?o")],
            )]
        }

        #[test]
        fn solve_datalog_goal_matches_seminaive_over_the_chain_program() {
            let rules = chain_rules();
            let mut edb = RelationStore::new();
            edb.insert(
                &surface("a"),
                &surface(P),
                &surface("b"),
                RelationStore::DEFAULT_GRAPH,
            );
            edb.insert(
                &surface("b"),
                &surface(P),
                &surface("c"),
                RelationStore::DEFAULT_GRAPH,
            );

            let exe = crate::seminaive::compile(rules.clone()).expect("the chain program compiles");
            let evaluation = crate::seminaive::evaluate(&exe, edb)
                .expect("the chain program stays inside every ceiling");
            assert!(evaluation.facts().contains(
                &surface("a"),
                &surface(T),
                &surface("c"),
                RelationStore::DEFAULT_GRAPH
            ));

            // `solve_datalog_goal` has no separate EDB channel — a `DlClause`
            // program IS the whole story for the backward resolver, so the EDB
            // rows must be present as their own fact clauses (empty bodies),
            // exactly as the forward evaluator's seeded `RelationStore` supplies
            // them on its own channel.
            let mut backward_rules = rules;
            backward_rules.push(DlClause::datalog(ground_atom("a", P, "b"), vec![]));
            backward_rules.push(DlClause::datalog(ground_atom("b", P, "c"), vec![]));

            let goal = ClauseAtom::positive(ClauseTerm::iri("a"), T, ClauseTerm::iri("c"));
            let budget = FolBudget { max_steps: 10_000 };
            let (mut dag, control) = solve_datalog_goal(&backward_rules, &goal, &budget)
                .expect("the chain program is Datalog-only");
            let FolControl::Decided(outcome) = control else {
                panic!("the backward resolver must also decide the goal");
            };
            assert_eq!(
                outcome.answers.len(),
                1,
                "a fully ground goal has one answer, or none"
            );

            let triple_op = dag.intern_leaf("triple");
            let fol_clauses = lower_datalog_clauses(&mut dag, triple_op, &backward_rules)
                .expect("the same Datalog-only program lowers cleanly a second time");
            let checked = check_fol_proof(
                &mut dag,
                &outcome.answers[0].proof,
                &fol_clauses,
                &always_not_false,
            )
            .expect("the backward derivation re-derives cleanly");
            assert_eq!(checked, outcome.answers[0].atom);
        }

        #[test]
        fn solve_datalog_goal_refuses_a_non_atomic_clause() {
            let existential = DlClause::new(
                vec![HeadDisjunct::atom(atom("?X", T, "?Y"))],
                vec!["?Y".to_owned()],
                vec![atom("?X", P, "?W")],
            );
            let goal = ClauseAtom::positive(ClauseTerm::iri("a"), T, ClauseTerm::iri("b"));
            let budget = FolBudget { max_steps: 1_000 };
            let error = solve_datalog_goal(&[existential], &goal, &budget)
                .expect_err("an existential head has no Datalog semantics");
            assert_eq!(error.clause(), 0);
            assert_eq!(error.form(), crate::clause::HeadForm::Existential);
        }

        /// The transitive-closure workload from this crate's own synthetic
        /// corpus agrees between the forward semi-naive fixpoint and this
        /// module's backward resolution, over a concrete goal.
        #[test]
        fn solve_datalog_goal_agrees_with_seminaive_over_a_corpus_workload() {
            let workload = crate::synth_corpus::all()
                .into_iter()
                .find(|workload| workload.name.contains("transitive"))
                .expect("the corpus has a transitive-closure workload");

            // The workload's own analytic golden already names a derived fact —
            // no need to run the forward evaluator just to pick a goal, and
            // using the ANALYTIC golden rather than the forward engine's own
            // output keeps this a comparison against a construction, not an
            // engine compared to itself.
            let derived = workload
                .expected
                .iter()
                .next()
                .cloned()
                .expect("the transitive-closure workload derives at least one fact");

            let exe = crate::seminaive::compile(workload.rules.clone())
                .expect("the corpus program compiles");
            let evaluation = crate::seminaive::evaluate(&exe, workload.edb())
                .expect("the corpus stays inside every ceiling");
            assert!(
                evaluation.facts().contains(
                    &derived.subject,
                    &derived.predicate,
                    &derived.object,
                    &derived.graph
                ),
                "the forward evaluator must derive the same golden fact"
            );

            let goal = ClauseAtom::quad(
                ClauseTerm::iri(strip_surface(&derived.subject)),
                ClauseTerm::iri(strip_surface(&derived.predicate)),
                ClauseTerm::iri(strip_surface(&derived.object)),
                if derived.graph.is_empty() {
                    ClauseTerm::default_graph()
                } else {
                    ClauseTerm::iri(strip_surface(&derived.graph))
                },
            );
            // As above: `solve_datalog_goal` has no separate EDB channel, so the
            // workload's EDB triples must ride along as their own fact clauses.
            let mut backward_rules = workload.rules.clone();
            for (subject, predicate, object, graph) in &workload.triples {
                let graph_term = if graph.is_empty() {
                    ClauseTerm::default_graph()
                } else {
                    ClauseTerm::iri(strip_surface(graph))
                };
                backward_rules.push(DlClause::datalog(
                    ClauseAtom::quad(
                        ClauseTerm::iri(strip_surface(subject)),
                        ClauseTerm::iri(strip_surface(predicate)),
                        ClauseTerm::iri(strip_surface(object)),
                        graph_term,
                    ),
                    vec![],
                ));
            }

            let budget = FolBudget { max_steps: 200_000 };
            let (_, control) = solve_datalog_goal(&backward_rules, &goal, &budget)
                .expect("the corpus workload is Datalog-only");
            let FolControl::Decided(outcome) = control else {
                panic!("a fact the forward evaluator derived must be backward-decidable too");
            };
            assert_eq!(
                outcome.answers.len(),
                1,
                "a fully ground goal the forward model derived must be found backward too"
            );
        }

        /// Strip the `<...>` bracketing [`ClauseTerm::surface`] renders an IRI
        /// with, so it can be re-wrapped through [`ClauseTerm::iri`].
        fn strip_surface(surface: &str) -> &str {
            surface
                .strip_prefix('<')
                .and_then(|s| s.strip_suffix('>'))
                .unwrap_or(surface)
        }
    }
}
