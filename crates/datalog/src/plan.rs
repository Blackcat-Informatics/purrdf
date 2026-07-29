// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Index selection and plan construction: the consuming type-state pipeline
//! `Parsed → Stratified → Planned → Executable`.
//!
//! # Why a type-state pipeline
//!
//! A semi-naive executor may run ONLY a program that has been (a) proven stratifiable
//! and (b) join-planned. Encoding that as a doc contract ("call `stratify` first") is
//! fragile: a caller can forget it, or redo it on every call. This module makes an
//! unstratified / unplanned program **unrepresentable at the executor boundary** — the
//! executor's only input is an [`Executable`], whose sole constructor chain is
//! `Parsed::new(..).stratify()?.plan().into_executable()`. There is no other way to
//! obtain one, so the compiler — not a comment — enforces "stratify, then plan, then
//! execute".
//!
//! # Consuming transitions, not marker generics
//!
//! Each stage is a DISTINCT type and each transition takes `self` by value, returning
//! the next stage. A `PhantomData<State>` marker bolted onto one shared struct would let
//! a caller name the wrong state or convert between them; distinct types cannot be
//! confused, and a consumed stage is moved-from and unusable, so a stale earlier-stage
//! value can never be fed to a later step.
//!
//! # What each stage memoizes
//!
//! - [`Stratified`] owns the [`stratify`] result lowered into the per-stratum rule
//!   grouping (`strata[k]` = the program-order rule indices of stratum `k`).
//! - [`Planned`] owns a [`RulePlan`] per rule: the positive/negated body partition, the
//!   flat variable slots, the binding-aware sideways-information-passing order, the
//!   guaranteed index shape, the variable/constant kernel shape, the certified cyclic
//!   subplans, and the swap programs that restore authored order. All are static
//!   functions of the rule, hoisted out of every semi-naive round.
//! - [`Executable`] additionally memoizes the head-predicate set (the IDB-derivable
//!   predicates) a completion frontier reads.
//!
//! Predicate resolution is deliberately NOT hoisted here: a
//! [`PredId`](crate::id::PredId) is a per-store handle minted at load/derivation time in
//! insertion order, so it is meaningless against a store that does not yet exist at plan
//! time. The plan is store-independent; resolving ids here would be unsound, not an
//! optimisation.
//!
//! # Determinism
//!
//! Every associative structure on this path is a `BTreeMap`/`BTreeSet`, and every
//! remaining choice is broken by an explicit total order (authored position, then
//! lexical name). No hash map participates, so a plan is a pure function of the rule
//! program: the same rules always compile to the same plan, on every target.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use crate::binding_pattern::BindingPattern;
use crate::id::TermId;
use crate::store::Bound;

// ── The rule IR the planner consumes ────────────────────────────────────────────

/// One argument position of a rule atom.
///
/// A constant carries the term's **lexical surface**, the same identity the relation
/// store interns on ([`crate::store::TermInterner`]), so a planned constant is compared
/// against stored data without a second rendering convention. An IRI is kept distinct
/// from a literal because only an IRI is bracketed when rendered.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PlanTerm {
    /// A rule variable, named as authored (the name is a plan-time key only).
    Var(String),
    /// A constant IRI, held UNBRACKETED; its surface is `<iri>`.
    Iri(String),
    /// A constant literal, held as its already-rendered lexical surface.
    Literal(String),
}

impl PlanTerm {
    /// A variable term.
    pub fn var(name: impl Into<String>) -> Self {
        Self::Var(name.into())
    }

    /// A constant IRI term, from the unbracketed IRI.
    pub fn iri(iri: impl Into<String>) -> Self {
        Self::Iri(iri.into())
    }

    /// A constant literal term, from its already-rendered lexical surface.
    pub fn literal(surface: impl Into<String>) -> Self {
        Self::Literal(surface.into())
    }

    /// The variable name, if this is a variable.
    pub fn variable(&self) -> Option<&str> {
        match self {
            Self::Var(name) => Some(name),
            Self::Iri(_) | Self::Literal(_) => None,
        }
    }

    /// Whether this term is a variable.
    pub fn is_var(&self) -> bool {
        matches!(self, Self::Var(_))
    }
}

/// The lexical surface of a CONSTANT term — the exact bytes the store interns.
///
/// # Panics
///
/// Panics on a variable: every caller reaches this only after matching a constant, so a
/// variable here is a planner bug, never a data state.
fn constant_surface(term: &PlanTerm) -> String {
    match term {
        PlanTerm::Iri(iri) => format!("<{iri}>"),
        PlanTerm::Literal(surface) => surface.clone(),
        PlanTerm::Var(name) => {
            unreachable!("constant_surface is called only for a constant term, not {name:?}")
        }
    }
}

/// One binary body or head atom: `predicate(subject, object)`, optionally negated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanAtom {
    /// The subject argument.
    subject: PlanTerm,
    /// The predicate IRI, UNBRACKETED — the key a relation is addressed by.
    predicate: String,
    /// The object argument.
    object: PlanTerm,
    /// Whether the atom is negated (a negation-as-failure filter, never a join driver).
    negated: bool,
}

impl PlanAtom {
    /// A positive atom (a join driver).
    pub fn positive(subject: PlanTerm, predicate: impl Into<String>, object: PlanTerm) -> Self {
        Self {
            subject,
            predicate: predicate.into(),
            object,
            negated: false,
        }
    }

    /// A negated atom (a negation-as-failure filter).
    pub fn negated(subject: PlanTerm, predicate: impl Into<String>, object: PlanTerm) -> Self {
        Self {
            negated: true,
            ..Self::positive(subject, predicate, object)
        }
    }

    /// The subject argument.
    pub fn subject(&self) -> &PlanTerm {
        &self.subject
    }

    /// The object argument.
    pub fn object(&self) -> &PlanTerm {
        &self.object
    }

    /// The predicate IRI, unbracketed.
    pub fn predicate(&self) -> &str {
        &self.predicate
    }

    /// Whether the atom is negated.
    pub fn is_negated(&self) -> bool {
        self.negated
    }
}

/// One rule: a head atom implied by the conjunction of its body atoms.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanRule {
    /// The derived atom.
    head: PlanAtom,
    /// The body conjunction, in authored order — the order every plan coordinate and
    /// every provenance restoration is expressed against.
    body: Vec<PlanAtom>,
}

impl PlanRule {
    /// A rule deriving `head` from `body`.
    ///
    /// # Panics
    ///
    /// Panics if `head` is negated: a negated head is not a rule, and admitting one here
    /// would silently produce an unsound program.
    pub fn new(head: PlanAtom, body: Vec<PlanAtom>) -> Self {
        assert!(!head.negated, "a rule head may not be negated");
        Self { head, body }
    }

    /// The derived atom.
    pub fn head(&self) -> &PlanAtom {
        &self.head
    }

    /// The body conjunction, in authored order.
    pub fn body(&self) -> &[PlanAtom] {
        &self.body
    }
}

// ── Stratification ──────────────────────────────────────────────────────────────

/// Assign every predicate of `rules` a stratum, or report the program non-stratifiable.
///
/// A rule's head predicate must sit at or above the stratum of every predicate its body
/// reads, and STRICTLY above every predicate it reads NEGATIVELY. That is what makes
/// negation-as-failure well defined: a negated atom is only ever evaluated against a
/// relation that has already reached its fixpoint.
///
/// The assignment is a Bellman-Ford-style relaxation over the dependency edges: `n`
/// passes suffice for a stratifiable program over `n` predicates, and one further pass
/// that still relaxes proves a negative edge sits inside a cycle. `None` means exactly
/// that — a declared gap, not a failure to try harder.
///
/// The predicate set is gathered through a `BTreeSet` and the edges are a `Vec` in
/// program order, so the relaxation sequence — and therefore the result — is a pure
/// function of the rule program.
pub fn stratify(rules: &[PlanRule]) -> Option<BTreeMap<String, usize>> {
    // Every predicate, heads and body atoms alike.
    let mut preds: BTreeSet<&str> = BTreeSet::new();
    for rule in rules {
        preds.insert(rule.head.predicate.as_str());
        for atom in &rule.body {
            preds.insert(atom.predicate.as_str());
        }
    }

    // Edges: (head predicate, body predicate, negative?).
    let mut edges: Vec<(&str, &str, bool)> = Vec::new();
    for rule in rules {
        for atom in &rule.body {
            edges.push((
                rule.head.predicate.as_str(),
                atom.predicate.as_str(),
                atom.negated,
            ));
        }
    }

    let mut stratum: BTreeMap<String, usize> =
        preds.iter().map(|p| ((*p).to_owned(), 0usize)).collect();

    let n = preds.len();
    for _pass in 0..=n {
        let mut changed = false;
        for (head, body, negative) in &edges {
            let body_s = stratum[*body];
            let need = if *negative { body_s + 1 } else { body_s };
            let head_s = stratum[*head];
            if head_s < need {
                stratum.insert((*head).to_owned(), need);
                changed = true;
            }
        }
        if !changed {
            return Some(stratum);
        }
    }
    // Still relaxing after n + 1 passes ⇒ a negative edge sits in a cycle.
    None
}

// ── Index selection ─────────────────────────────────────────────────────────────

/// The guaranteed index shape at one point in the planned execution order.
///
/// Runtime values are resolved to store-local term ids only when the atom is scanned,
/// but WHICH columns are bound is a plan-time property: by the time this atom runs, the
/// sideways-information-passing order has already bound the named positions.
///
/// This is the arity-2 face of the crate's adornment lattice — see
/// [`pattern`](Self::pattern) — specialised to the store's `(subject, object)` columns,
/// so the index choice is a single `Copy` tag on the hot path rather than a bitset
/// re-derived per tuple.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum IndexChoice {
    /// Neither column bound: a full scan of the relation.
    Any,
    /// Subject bound: gallop the sorted subject column.
    Subject,
    /// Object bound: walk the lazily-built `(object, subject)` permutation.
    Object,
    /// Both columns bound: at most one row, since the key is unique.
    Both,
}

impl IndexChoice {
    /// This index shape as an arity-2 [`BindingPattern`]: position 0 is the subject,
    /// position 1 the object.
    ///
    /// The adornment codes are `"ff"`, `"bf"`, `"fb"` and `"bb"` respectively — the same
    /// lattice a backward magic-sets demand is keyed by, so an index shape and a demand
    /// adornment are comparable values rather than two vocabularies kept in sync by
    /// hand.
    pub fn pattern(self) -> BindingPattern {
        let bound: &[usize] = match self {
            Self::Any => &[],
            Self::Subject => &[0],
            Self::Object => &[1],
            Self::Both => &[0, 1],
        };
        BindingPattern::from_bound_positions(2, bound.iter().copied())
    }

    /// The index shape an arity-2 adornment describes.
    ///
    /// # Panics
    ///
    /// Panics unless `pattern` has arity 2: a binary relation's index shape is not
    /// defined for any other arity, and silently truncating a wider adornment would
    /// select the wrong access path.
    pub fn from_pattern(pattern: BindingPattern) -> Self {
        assert_eq!(
            pattern.arity(),
            2,
            "an index choice is an arity-2 adornment over (subject, object)"
        );
        match (pattern.is_bound(0), pattern.is_bound(1)) {
            (false, false) => Self::Any,
            (true, false) => Self::Subject,
            (false, true) => Self::Object,
            (true, true) => Self::Both,
        }
    }

    /// Lower this plan-time shape to a runtime store [`Bound`], given the interned ids of
    /// whichever positions the shape says are bound.
    ///
    /// `None` means a bound position's term is NOT interned in the store being probed,
    /// so nothing can match it and the caller short-circuits to the empty selection.
    /// That is the same probe-miss semantics
    /// [`RelationStore::term_id`](crate::store::RelationStore::term_id) defines,
    /// expressed once here rather than at every call site.
    pub fn bound(self, subject: Option<TermId>, object: Option<TermId>) -> Option<Bound> {
        match self {
            Self::Any => Some(Bound::Any),
            Self::Subject => subject.map(Bound::Subject),
            Self::Object => object.map(Bound::Object),
            Self::Both => match (subject, object) {
                (Some(s), Some(o)) => Some(Bound::Both(s, o)),
                _ => None,
            },
        }
    }
}

/// Whether a term's value is already known at this point in the execution order: a
/// constant always is, a variable only once something has bound it.
fn term_is_known(term: &PlanTerm, bound: &BTreeSet<String>) -> bool {
    match term {
        PlanTerm::Var(variable) => bound.contains(variable),
        PlanTerm::Iri(_) | PlanTerm::Literal(_) => true,
    }
}

/// Record every variable of `atom` as bound: the atom has been scanned, so its columns
/// carry values from here on.
fn bind_atom_variables(atom: &PlanAtom, bound: &mut BTreeSet<String>) {
    for term in [&atom.subject, &atom.object] {
        if let PlanTerm::Var(variable) = term {
            bound.insert(variable.clone());
        }
    }
}

/// The guaranteed index shape for `atom`, given the variables already bound.
fn index_choice(atom: &PlanAtom, bound: &BTreeSet<String>) -> IndexChoice {
    IndexChoice::from_pattern(BindingPattern::from_bools([
        term_is_known(&atom.subject, bound),
        term_is_known(&atom.object, bound),
    ]))
}

/// Deterministic sideways-information-passing order for an acyclic positive body.
///
/// With no store in hand, cardinalities cannot be consulted soundly — a plan must stay
/// store-independent. The static information available at plan time is still worth a
/// great deal: prefer atoms with more already-bound or constant columns, then more
/// constants, then a repeated-variable equality, and finally the authored position.
/// After an atom is chosen, all of its variables become bound for subsequent choices.
///
/// Every component of the key is a plan-time integer and the last component is the
/// authored position, which is unique — so the maximum is unique and the order is stable.
fn sips_order(rule: &PlanRule, positive: &[usize]) -> Vec<usize> {
    let mut remaining: Vec<usize> = (0..positive.len()).collect();
    let mut bound = BTreeSet::new();
    let mut order = Vec::with_capacity(positive.len());
    while !remaining.is_empty() {
        let (slot, &positive_position) = remaining
            .iter()
            .enumerate()
            .max_by_key(|&(_, &positive_position)| {
                let atom = &rule.body[positive[positive_position]];
                let known = usize::from(term_is_known(&atom.subject, &bound))
                    + usize::from(term_is_known(&atom.object, &bound));
                let constants =
                    usize::from(!atom.subject.is_var()) + usize::from(!atom.object.is_var());
                let repeated = usize::from(matches!(
                    (&atom.subject, &atom.object),
                    (PlanTerm::Var(left), PlanTerm::Var(right)) if left == right
                ));
                (known, constants, repeated, usize::MAX - positive_position)
            })
            .expect("a non-empty remaining set has a best atom");
        remaining.remove(slot);
        order.push(positive_position);
        bind_atom_variables(&rule.body[positive[positive_position]], &mut bound);
    }
    order
}

// ── Lowered operators ───────────────────────────────────────────────────────────

/// The statically selected subject/object term shape of one binary atom.
///
/// Term-kind dispatch happens once, here: the tuple loop never re-interprets a term enum
/// or re-renders a constant surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AtomKernel {
    /// Both positions are variables, bound into these flat slots.
    Vars {
        /// The subject variable's slot in the rule's binding frame.
        subject_slot: usize,
        /// The object variable's slot in the rule's binding frame.
        object_slot: usize,
    },
    /// A variable subject and a constant object.
    VarConst {
        /// The subject variable's slot in the rule's binding frame.
        subject_slot: usize,
        /// The object constant's lexical surface.
        object: String,
    },
    /// A constant subject and a variable object.
    ConstVar {
        /// The subject constant's lexical surface.
        subject: String,
        /// The object variable's slot in the rule's binding frame.
        object_slot: usize,
    },
    /// Both positions are constants — a ground membership probe.
    Consts {
        /// The subject constant's lexical surface.
        subject: String,
        /// The object constant's lexical surface.
        object: String,
    },
}

/// The kernel for `atom`, resolving each variable to its flat binding slot and each
/// constant to its lexical surface.
fn atom_kernel(atom: &PlanAtom, slots: &BTreeMap<String, usize>) -> AtomKernel {
    match (&atom.subject, &atom.object) {
        (PlanTerm::Var(subject), PlanTerm::Var(object)) => AtomKernel::Vars {
            subject_slot: slots[subject],
            object_slot: slots[object],
        },
        (PlanTerm::Var(subject), object) => AtomKernel::VarConst {
            subject_slot: slots[subject],
            object: constant_surface(object),
        },
        (subject, PlanTerm::Var(object)) => AtomKernel::ConstVar {
            subject: constant_surface(subject),
            object_slot: slots[object],
        },
        (subject, object) => AtomKernel::Consts {
            subject: constant_surface(subject),
            object: constant_surface(object),
        },
    }
}

/// A positive atom lowered to a body coordinate plus one monomorphic term-shape kernel.
///
/// Runtime binding presence still selects the concrete store bound (through
/// [`IndexChoice::bound`]), but variable-name and term-enum interpretation is absent from
/// the tuple loop entirely.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AtomOperator {
    /// Index into the rule's authored body.
    body_index: usize,
    /// Index within the rule's positive-atom sequence.
    positive_position: usize,
    /// The guaranteed index shape at this point in the execution order.
    index: IndexChoice,
    /// The statically selected term shape.
    kernel: AtomKernel,
}

impl AtomOperator {
    /// Index into the rule's authored body.
    pub fn body_index(&self) -> usize {
        self.body_index
    }

    /// Index within the rule's positive-atom sequence.
    pub fn positive_position(&self) -> usize {
        self.positive_position
    }

    /// The guaranteed index shape at this point in the execution order.
    pub fn index(&self) -> IndexChoice {
        self.index
    }

    /// The statically selected term shape.
    pub fn kernel(&self) -> &AtomKernel {
        &self.kernel
    }
}

/// Lower every positive atom, in execution order, to its operator — computing each
/// atom's index shape against exactly the variables bound before it runs.
fn lower_operators(
    rule: &PlanRule,
    positive: &[usize],
    execution_order: &[usize],
    slots: &BTreeMap<String, usize>,
) -> Vec<AtomOperator> {
    let mut bound = BTreeSet::new();
    let mut operators = Vec::with_capacity(execution_order.len());
    for &positive_position in execution_order {
        let body_index = positive[positive_position];
        let atom = &rule.body[body_index];
        operators.push(AtomOperator {
            body_index,
            positive_position,
            index: index_choice(atom, &bound),
            kernel: atom_kernel(atom, slots),
        });
        bind_atom_variables(atom, &mut bound);
    }
    operators
}

/// Precompute a minimal deterministic swap program from physical execution order back to
/// authored positive-body order.
///
/// The executor applies these swaps directly to each completed solution, so restoring
/// authored provenance order costs no per-solution permutation allocation.
fn restore_body_order_swaps(execution_order: &[usize]) -> Vec<(usize, usize)> {
    let mut current = execution_order.to_vec();
    let mut swaps = Vec::new();
    for wanted in 0..current.len() {
        let position = current
            .iter()
            .position(|&value| value == wanted)
            .expect("physical groups cover every positive atom exactly once");
        if position != wanted {
            current.swap(wanted, position);
            swaps.push((wanted, position));
        }
    }
    debug_assert!(current.iter().copied().eq(0..current.len()));
    swaps
}

// ── Cyclic certification ────────────────────────────────────────────────────────

/// One positive atom together with both of its stable coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlannedAtom {
    /// Index into the rule's authored body.
    body_index: usize,
    /// Index within the rule's positive-atom sequence.
    positive_position: usize,
}

impl PlannedAtom {
    /// Index into the rule's authored body.
    pub fn body_index(self) -> usize {
        self.body_index
    }

    /// Index within the rule's positive-atom sequence.
    pub fn positive_position(self) -> usize {
        self.positive_position
    }
}

/// A planner-certified cyclic component, lowered to the multiway (leapfrog) kernel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CyclicPlan {
    /// The component's atoms, in authored positive-body order.
    atoms: Vec<PlannedAtom>,
    /// The deterministic variable order the trie join descends: structural degree
    /// descending, then first authored occurrence, then lexical name.
    variables: Vec<String>,
    /// The same order lowered to the rule's flat binding slots.
    variable_slots: Vec<usize>,
}

impl CyclicPlan {
    /// The component's atoms, in authored positive-body order.
    pub fn atoms(&self) -> &[PlannedAtom] {
        &self.atoms
    }

    /// The deterministic variable order the trie join descends.
    pub fn variables(&self) -> &[String] {
        &self.variables
    }

    /// The same order lowered to the rule's flat binding slots.
    pub fn variable_slots(&self) -> &[usize] {
        &self.variable_slots
    }
}

/// One physical group in a rule's positive join.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JoinGroup {
    /// The indexed binary operator for one atom.
    Binary(PlannedAtom),
    /// One certified cyclic component evaluated as a multiway leapfrog join.
    Leapfrog(CyclicPlan),
}

/// A disjoint-set forest over variable nodes, with path halving and union by size.
///
/// Hand-rolled rather than pulled from a graph crate: it is a dozen lines, it must build
/// for every target this crate supports, and the component identity it produces is
/// deliberately not read out of it — see [`certified_cyclic_components`], which keys
/// components by their smallest node so that nothing observable depends on which element
/// the forest happened to make a representative.
#[derive(Debug)]
struct UnionFind {
    /// `parent[i]` is `i`'s parent, or `i` itself for a root.
    parent: Vec<usize>,
    /// Subtree size per root, for union by size.
    size: Vec<usize>,
}

impl UnionFind {
    /// A forest of `n` singleton sets.
    fn new(n: usize) -> Self {
        Self {
            parent: (0..n).collect(),
            size: vec![1; n],
        }
    }

    /// The representative of `x`'s set, halving the path on the way up.
    fn find(&mut self, mut x: usize) -> usize {
        while self.parent[x] != x {
            self.parent[x] = self.parent[self.parent[x]];
            x = self.parent[x];
        }
        x
    }

    /// Merge the sets of `a` and `b`.
    fn union(&mut self, a: usize, b: usize) {
        let (mut ra, mut rb) = (self.find(a), self.find(b));
        if ra == rb {
            return;
        }
        if self.size[ra] < self.size[rb] {
            core::mem::swap(&mut ra, &mut rb);
        }
        self.parent[rb] = ra;
        self.size[ra] += self.size[rb];
    }
}

/// The bridges of an undirected graph: the edges whose removal disconnects their
/// component — equivalently, exactly the edges that lie on NO cycle.
///
/// Tarjan's discovery/low-link search, iterative rather than recursive so a pathological
/// rule cannot overflow a small stack. `edges[e] = (u, v)`; the returned ids index that
/// slice. Adjacency is built in edge order, so the traversal — and therefore the answer —
/// is a pure function of the input.
fn bridge_edges(node_count: usize, edges: &[(usize, usize)]) -> BTreeSet<usize> {
    let mut adjacency: Vec<Vec<(usize, usize)>> = vec![Vec::new(); node_count];
    for (id, &(u, v)) in edges.iter().enumerate() {
        adjacency[u].push((v, id));
        adjacency[v].push((u, id));
    }

    /// Sentinel discovery time for a node the search has not reached.
    const UNVISITED: usize = usize::MAX;
    let mut discovery = vec![UNVISITED; node_count];
    let mut low = vec![0usize; node_count];
    let mut timer = 0usize;
    let mut bridges = BTreeSet::new();

    for start in 0..node_count {
        if discovery[start] != UNVISITED {
            continue;
        }
        discovery[start] = timer;
        low[start] = timer;
        timer += 1;
        // Each frame is (node, the edge id it was entered by, the next adjacency slot).
        let mut stack: Vec<(usize, Option<usize>, usize)> = vec![(start, None, 0)];
        while let Some(&(node, entered_by, slot)) = stack.last() {
            if slot < adjacency[node].len() {
                stack.last_mut().expect("the frame just observed").2 += 1;
                let (next, edge) = adjacency[node][slot];
                // Skip only the exact edge the node was entered by, never every edge to
                // the parent: parallel edges between two nodes are never bridges.
                if Some(edge) == entered_by {
                    continue;
                }
                if discovery[next] == UNVISITED {
                    discovery[next] = timer;
                    low[next] = timer;
                    timer += 1;
                    stack.push((next, Some(edge), 0));
                } else {
                    low[node] = low[node].min(discovery[next]);
                }
            } else {
                stack.pop();
                if let Some(&(parent, _, _)) = stack.last() {
                    low[parent] = low[parent].min(low[node]);
                    if low[node] > discovery[parent] {
                        bridges.insert(entered_by.expect("a child frame has an entering edge"));
                    }
                }
            }
        }
    }
    bridges
}

/// Certify the positive-body cycle components eligible for a multiway join.
///
/// Each atom with two DISTINCT variable positions contributes one undirected variable
/// edge. Duplicate pairs are collapsed before the graph analysis: two relations over the
/// same `(X, Y)` edge are an acyclic intersection, not a two-edge cycle. Removing every
/// bridge leaves exactly the edges that participate in a simple cycle, and their
/// connected components are the subplans safe to promote. Constants, repeated variables,
/// trees and bridge atoms therefore all stay binary.
///
/// Components are keyed by their SMALLEST variable node, so the emitted order depends on
/// the (lexically ordered) variable numbering alone and never on a disjoint-set forest's
/// internal choice of representative.
fn certified_cyclic_components(
    rule: &PlanRule,
    positive: &[usize],
    slots: &BTreeMap<String, usize>,
) -> Vec<CyclicPlan> {
    let mut edge_atoms: BTreeMap<(String, String), Vec<PlannedAtom>> = BTreeMap::new();
    let mut first_occurrence: BTreeMap<String, usize> = BTreeMap::new();
    let mut occurrence = 0usize;

    for (positive_position, &body_index) in positive.iter().enumerate() {
        let planned = PlannedAtom {
            body_index,
            positive_position,
        };
        let atom = &rule.body[body_index];
        for term in [&atom.subject, &atom.object] {
            if let PlanTerm::Var(var) = term {
                first_occurrence.entry(var.clone()).or_insert(occurrence);
                occurrence += 1;
            }
        }
        let (PlanTerm::Var(left), PlanTerm::Var(right)) = (&atom.subject, &atom.object) else {
            continue;
        };
        if left == right {
            continue;
        }
        let edge = if left < right {
            (left.clone(), right.clone())
        } else {
            (right.clone(), left.clone())
        };
        edge_atoms.entry(edge).or_default().push(planned);
    }

    // A simple undirected cycle needs at least three distinct edges.
    if edge_atoms.len() < 3 {
        return Vec::new();
    }

    let variable_names: Vec<String> = edge_atoms
        .keys()
        .flat_map(|(left, right)| [left.clone(), right.clone()])
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let variable_id: BTreeMap<&str, usize> = variable_names
        .iter()
        .enumerate()
        .map(|(id, name)| (name.as_str(), id))
        .collect();

    let edge_keys: Vec<(String, String)> = edge_atoms.keys().cloned().collect();
    let edges: Vec<(usize, usize)> = edge_keys
        .iter()
        .map(|(left, right)| (variable_id[left.as_str()], variable_id[right.as_str()]))
        .collect();

    let bridge_ids = bridge_edges(variable_names.len(), &edges);
    let mut union = UnionFind::new(variable_names.len());
    for (id, &(left, right)) in edges.iter().enumerate() {
        if bridge_ids.contains(&id) {
            continue;
        }
        union.union(left, right);
    }

    // Group the surviving (cycle) edges by component, keyed by the component's smallest
    // node, so the emitted order is a function of the lexical variable order alone.
    let mut component_key: BTreeMap<usize, usize> = BTreeMap::new();
    for (id, &(left, _)) in edges.iter().enumerate() {
        if bridge_ids.contains(&id) {
            continue;
        }
        let root = union.find(left);
        let key = component_key.entry(root).or_insert(left);
        *key = (*key).min(left);
    }
    let mut component_edges: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for (id, &(left, _)) in edges.iter().enumerate() {
        if bridge_ids.contains(&id) {
            continue;
        }
        let root = union.find(left);
        component_edges
            .entry(component_key[&root])
            .or_default()
            .push(id);
    }

    let mut plans = Vec::new();
    for ids in component_edges.into_values() {
        // A simple undirected cycle has at least three unique edges. This also makes the
        // duplicate-pair exclusion explicit at the promotion boundary.
        if ids.len() < 3 {
            continue;
        }
        let mut atoms = Vec::new();
        let mut degree: BTreeMap<String, usize> = BTreeMap::new();
        for id in ids {
            let key = &edge_keys[id];
            atoms.extend(edge_atoms[key].iter().copied());
            *degree.entry(key.0.clone()).or_default() += 1;
            *degree.entry(key.1.clone()).or_default() += 1;
        }
        atoms.sort_by_key(|atom| atom.positive_position);
        let mut variables: Vec<String> = degree.keys().cloned().collect();
        variables.sort_by(|left, right| {
            degree[right]
                .cmp(&degree[left])
                .then_with(|| first_occurrence[left].cmp(&first_occurrence[right]))
                .then_with(|| left.cmp(right))
        });
        let variable_slots = variables.iter().map(|variable| slots[variable]).collect();
        plans.push(CyclicPlan {
            atoms,
            variables,
            variable_slots,
        });
    }
    plans
}

// ── The per-rule plan ───────────────────────────────────────────────────────────

/// The cyclic-only physical sidecar.
///
/// Boxed so the overwhelmingly common acyclic [`RulePlan`] carries one null pointer
/// instead of two more slices: selective multiway joining must not tax the binary
/// majority's resident plan footprint.
#[derive(Debug, Clone, PartialEq, Eq)]
struct HybridPlan {
    /// The physical positive-join groups, in execution order.
    join_groups: Box<[JoinGroup]>,
    /// Swaps restoring the cyclic execution order to authored order.
    source_order_swaps: Box<[(usize, usize)]>,
}

/// A per-rule precomputed join plan.
///
/// This is the store-independent relational-algebra plan: body partition, flat binding
/// frame, sideways-information-passing order, index and term-shape kernels, certified
/// cyclic groups, and the swap programs that restore authored provenance order.
/// Store-local term ids remain runtime values, but which columns are bound and which
/// concrete kernel consumes them are decided here, once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RulePlan {
    /// Body indices of the POSITIVE atoms, in body order (the join drivers).
    positive: Box<[usize]>,
    /// Body indices of the NEGATED atoms, in body order (the negation filters).
    negated: Box<[usize]>,
    /// Variable names in authored first-occurrence order; the executor stores bindings
    /// in a flat frame indexed by these slots.
    variables: Box<[String]>,
    /// One statically-shaped operator per positive body atom, in physical execution
    /// order. Each retains its authored positive position for provenance restoration.
    operators: Box<[AtomOperator]>,
    /// In-place swaps restoring positive-source provenance from physical execution order
    /// to authored body order.
    operator_source_order_swaps: Box<[(usize, usize)]>,
    /// Present only for a structurally-certified cyclic rule.
    hybrid: Option<Box<HybridPlan>>,
}

impl RulePlan {
    /// Plan one rule.
    ///
    /// The body is partitioned into positive (join) and negated (filter) atoms,
    /// preserving body order; variables are assigned flat slots in authored
    /// first-occurrence order across the body and then the head. Body-first preserves the
    /// join's natural binding order, and including the head gives diagnostics a complete
    /// rule layout without changing the positive operators.
    pub(crate) fn for_rule(rule: &PlanRule) -> Self {
        let mut positive = Vec::new();
        let mut negated = Vec::new();
        for (i, atom) in rule.body.iter().enumerate() {
            if atom.negated {
                negated.push(i);
            } else {
                positive.push(i);
            }
        }

        let mut variables = Vec::new();
        let mut slots = BTreeMap::new();
        for atom in rule.body.iter().chain(core::iter::once(&rule.head)) {
            for term in [&atom.subject, &atom.object] {
                if let PlanTerm::Var(name) = term
                    && !slots.contains_key(name)
                {
                    let slot = variables.len();
                    variables.push(name.clone());
                    slots.insert(name.clone(), slot);
                }
            }
        }

        // A simple undirected cycle requires at least three positive edges. Avoid all
        // graph and planned-atom scratch for the very common 0/1/2-atom rules.
        let cyclic = if positive.len() < 3 {
            Vec::new()
        } else {
            certified_cyclic_components(rule, &positive, &slots)
        };

        if cyclic.is_empty() {
            let execution_order = sips_order(rule, &positive);
            let operators = lower_operators(rule, &positive, &execution_order, &slots);
            let operator_source_order_swaps = restore_body_order_swaps(&execution_order);
            return Self {
                positive: positive.into_boxed_slice(),
                negated: negated.into_boxed_slice(),
                variables: variables.into_boxed_slice(),
                operators: operators.into_boxed_slice(),
                operator_source_order_swaps: operator_source_order_swaps.into_boxed_slice(),
                hybrid: None,
            };
        }

        // Map every promoted atom to its owning component. Components are edge-disjoint
        // after bridge removal, so one atom belongs to at most one of them.
        let mut component_of: Vec<Option<usize>> = vec![None; rule.body.len()];
        for (component, plan) in cyclic.iter().enumerate() {
            for atom in &plan.atoms {
                component_of[atom.body_index] = Some(component);
            }
        }

        // Emit a component at its first authored atom and skip its later atoms; any
        // non-cycle atom stays a binary group at its own authored position.
        let mut cyclic: Vec<Option<CyclicPlan>> = cyclic.into_iter().map(Some).collect();
        let mut join_groups = Vec::new();
        let mut execution_source_order = Vec::with_capacity(positive.len());
        for (positive_position, &body_index) in positive.iter().enumerate() {
            let atom = PlannedAtom {
                body_index,
                positive_position,
            };
            match component_of[atom.body_index] {
                Some(component) => {
                    if let Some(plan) = cyclic[component].take() {
                        execution_source_order
                            .extend(plan.atoms.iter().map(|a| a.positive_position));
                        join_groups.push(JoinGroup::Leapfrog(plan));
                    }
                }
                None => {
                    execution_source_order.push(atom.positive_position);
                    join_groups.push(JoinGroup::Binary(atom));
                }
            }
        }

        let source_order_swaps = restore_body_order_swaps(&execution_source_order);
        let operators = lower_operators(rule, &positive, &execution_source_order, &slots);
        let operator_source_order_swaps = restore_body_order_swaps(&execution_source_order);
        Self {
            positive: positive.into_boxed_slice(),
            negated: negated.into_boxed_slice(),
            variables: variables.into_boxed_slice(),
            operators: operators.into_boxed_slice(),
            operator_source_order_swaps: operator_source_order_swaps.into_boxed_slice(),
            hybrid: Some(Box::new(HybridPlan {
                join_groups: join_groups.into_boxed_slice(),
                source_order_swaps: source_order_swaps.into_boxed_slice(),
            })),
        }
    }

    /// The positive body-atom indices, in body order.
    pub fn positive(&self) -> &[usize] {
        &self.positive
    }

    /// The negated body-atom indices, in body order.
    pub fn negated(&self) -> &[usize] {
        &self.negated
    }

    /// The stable slot-to-variable table for the rule's physical binding frame.
    pub fn variables(&self) -> &[String] {
        &self.variables
    }

    /// The prelowered positive atom operators, in physical execution order.
    pub fn operators(&self) -> &[AtomOperator] {
        &self.operators
    }

    /// The operator for one authored positive-body coordinate.
    ///
    /// # Panics
    ///
    /// Panics if `positive_position` is not a positive coordinate of this rule.
    pub fn operator_at(&self, positive_position: usize) -> &AtomOperator {
        self.operators
            .iter()
            .find(|operator| operator.positive_position == positive_position)
            .expect("every positive atom has exactly one physical operator")
    }

    /// Whether this rule has a planner-certified cyclic positive subplan.
    pub fn has_cyclic_subplan(&self) -> bool {
        self.hybrid.is_some()
    }

    /// The physical positive-join groups, in deterministic execution order.
    ///
    /// # Panics
    ///
    /// Panics unless [`has_cyclic_subplan`](Self::has_cyclic_subplan): an acyclic rule
    /// allocates no group sidecar at all, and fabricating one here would erase the
    /// distinction the type exists to make.
    pub fn join_groups(&self) -> &[JoinGroup] {
        &self
            .hybrid
            .as_ref()
            .expect("join groups exist only for a certified cyclic plan")
            .join_groups
    }

    /// In-place swaps restoring physical operator order to authored body order.
    pub fn operator_source_order_swaps(&self) -> &[(usize, usize)] {
        &self.operator_source_order_swaps
    }

    /// Source restoration for the cyclic groups' physical execution order.
    ///
    /// # Panics
    ///
    /// Panics unless [`has_cyclic_subplan`](Self::has_cyclic_subplan).
    pub fn hybrid_source_order_swaps(&self) -> &[(usize, usize)] {
        &self
            .hybrid
            .as_ref()
            .expect("hybrid source swaps exist only for a cyclic plan")
            .source_order_swaps
    }
}

// ── The type-state pipeline ─────────────────────────────────────────────────────

/// Stage 1: a parsed rule program, not yet stratified.
///
/// Owns the rules behind an [`Arc`], so the terminal executable can be shared across
/// calls without borrowing a caller's scratch buffer.
#[derive(Debug, Clone)]
pub struct Parsed {
    /// The program, in authored order.
    rules: Arc<[PlanRule]>,
}

impl Parsed {
    /// Enter the pipeline with a parsed rule program.
    pub fn new(rules: Vec<PlanRule>) -> Self {
        Self {
            rules: Arc::from(rules),
        }
    }

    /// Compute the stratification ONCE and lower it into the per-stratum rule grouping.
    ///
    /// `None` means the program is non-stratifiable — a negative dependency edge inside a
    /// cycle. That is a declared gap the caller reports; there is no fallback engine and
    /// no partial answer.
    ///
    /// A rule belongs to the stratum of its HEAD predicate, and within a stratum the
    /// authored program order is preserved, so rules fire in a stable order.
    pub fn stratify(self) -> Option<Stratified> {
        let stratum_of = stratify(&self.rules)?;

        let max_stratum = self
            .rules
            .iter()
            .map(|r| stratum_of[r.head.predicate.as_str()])
            .max()
            .unwrap_or(0);
        let mut strata: Vec<Vec<usize>> = vec![Vec::new(); max_stratum + 1];
        for (i, rule) in self.rules.iter().enumerate() {
            strata[stratum_of[rule.head.predicate.as_str()]].push(i);
        }

        Some(Stratified {
            rules: self.rules,
            strata,
        })
    }
}

/// Stage 2: a stratifiable program with its per-stratum rule grouping memoized.
#[derive(Debug, Clone)]
pub struct Stratified {
    /// The program, in authored order.
    rules: Arc<[PlanRule]>,
    /// `strata[k]` = the program-order indices of stratum `k`'s rules.
    strata: Vec<Vec<usize>>,
}

impl Stratified {
    /// Precompute one complete store-independent [`RulePlan`] per rule.
    pub fn plan(self) -> Planned {
        let plans: Vec<RulePlan> = self.rules.iter().map(RulePlan::for_rule).collect();
        Planned {
            rules: self.rules,
            strata: self.strata,
            plans,
        }
    }
}

/// Stage 3: a stratified program with its per-rule join plans memoized.
#[derive(Debug, Clone)]
pub struct Planned {
    /// The program, in authored order.
    rules: Arc<[PlanRule]>,
    /// `strata[k]` = the program-order indices of stratum `k`'s rules.
    strata: Vec<Vec<usize>>,
    /// One entry per rule, parallel to `rules` by index.
    plans: Vec<RulePlan>,
}

impl Planned {
    /// Seal the plan into the terminal [`Executable`], memoizing the head-predicate set a
    /// completion frontier reads.
    pub fn into_executable(self) -> Executable {
        let head_predicates: BTreeSet<String> = self
            .rules
            .iter()
            .map(|r| r.head.predicate.clone())
            .collect();
        Executable {
            rules: self.rules,
            strata: self.strata,
            plans: self.plans,
            head_predicates,
        }
    }
}

/// Stage 4 (terminal): a fully stratified, join-planned program.
///
/// The SOLE input type of a semi-naive executor. Its fields are private and its only
/// constructor is [`Planned::into_executable`], so a value of this type is a proof that
/// the program was stratified (stage 1 → 2) and planned (stage 2 → 3).
#[derive(Debug, Clone)]
pub struct Executable {
    /// The program, in authored order.
    rules: Arc<[PlanRule]>,
    /// `strata[k]` = the program-order indices of stratum `k`'s rules.
    strata: Vec<Vec<usize>>,
    /// One entry per rule, parallel to `rules` by index.
    plans: Vec<RulePlan>,
    /// The IDB-derivable (rule-head) predicates, in lexical order.
    head_predicates: BTreeSet<String>,
}

impl Executable {
    /// The number of strata (a completion frontier's total).
    pub fn stratum_count(&self) -> usize {
        self.strata.len()
    }

    /// Whether stratum `k` has no rules (a trivially-saturated empty stratum).
    ///
    /// # Panics
    ///
    /// Panics if `k` is not a stratum of this program.
    pub fn stratum_is_empty(&self, k: usize) -> bool {
        self.strata[k].is_empty()
    }

    /// The IDB-derivable (rule-head) predicates — the ones settled only when their
    /// stratum completes, and therefore excluded from a pure-EDB seed frontier.
    pub fn head_predicates(&self) -> &BTreeSet<String> {
        &self.head_predicates
    }

    /// The program-order rule indices assigned to stratum `k`.
    ///
    /// Exposing the immutable index slice lets an executor drive an INDEXED parallel
    /// iterator while preserving program order at the deterministic merge boundary.
    ///
    /// # Panics
    ///
    /// Panics if `k` is not a stratum of this program.
    pub fn stratum_rule_indices(&self, k: usize) -> &[usize] {
        &self.strata[k]
    }

    /// Resolve one rule index to its immutable rule and precomputed plan.
    ///
    /// # Panics
    ///
    /// Panics if `index` is not a rule of this program.
    pub fn rule_entry(&self, index: usize) -> (&PlanRule, &RulePlan) {
        (&self.rules[index], &self.plans[index])
    }

    /// The head predicates of stratum `k`'s rules — recorded into the settled frontier
    /// when the stratum reaches its natural fixpoint.
    ///
    /// # Panics
    ///
    /// Panics if `k` is not a stratum of this program.
    pub fn stratum_head_predicates(&self, k: usize) -> impl Iterator<Item = &str> {
        self.strata[k]
            .iter()
            .map(move |&i| self.rules[i].head.predicate.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::permute;

    const P: &str = "https://example.org/p";
    const Q: &str = "https://example.org/q";
    const R: &str = "https://example.org/r";
    const S: &str = "https://example.org/s";

    fn v(name: &str) -> PlanTerm {
        PlanTerm::var(name)
    }

    fn atom(subject: &str, predicate: &str, object: &str) -> PlanAtom {
        PlanAtom::positive(v(subject), predicate, v(object))
    }

    /// The three-atom triangle `p(X,Y), q(Y,Z), r(Z,X)` — the canonical cyclic body.
    fn triangle_body() -> Vec<PlanAtom> {
        vec![
            atom("?X", P, "?Y"),
            atom("?Y", Q, "?Z"),
            atom("?Z", R, "?X"),
        ]
    }

    fn triangle_rule(body: Vec<PlanAtom>) -> PlanRule {
        PlanRule::new(atom("?X", S, "?Z"), body)
    }

    #[test]
    fn plan_partitions_the_body_preserving_authored_order() {
        let rule = PlanRule::new(
            atom("?X", S, "?Z"),
            vec![
                atom("?X", P, "?Y"),
                PlanAtom::negated(v("?Y"), Q, v("?Z")),
                atom("?Y", R, "?Z"),
            ],
        );
        let plan = RulePlan::for_rule(&rule);
        assert_eq!(plan.positive(), [0, 2]);
        assert_eq!(plan.negated(), [1]);
        assert!(!plan.has_cyclic_subplan());
        assert_eq!(rule.body()[1].subject().variable(), Some("?Y"));
        assert!(!rule.head().is_negated());
    }

    /// Variable slots are authored first-occurrence order across the body, then the head
    /// — a head-only variable still gets a slot, at the end.
    #[test]
    fn plan_assigns_slots_in_authored_first_occurrence_order() {
        let rule = PlanRule::new(
            PlanAtom::positive(v("?W"), S, v("?X")),
            vec![atom("?X", P, "?Y"), atom("?Y", Q, "?X")],
        );
        let plan = RulePlan::for_rule(&rule);
        assert_eq!(plan.variables(), ["?X", "?Y", "?W"]);
    }

    /// A constant is bound from the start, so the planner runs the constant-bearing atom
    /// first and every later atom inherits its bindings — the whole point of the
    /// sideways-information-passing order.
    #[test]
    fn plan_sips_runs_the_most_bound_atom_first() {
        let rule = PlanRule::new(
            atom("?X", S, "?Z"),
            vec![
                atom("?Y", Q, "?Z"),
                PlanAtom::positive(PlanTerm::iri("https://example.org/a"), P, v("?Y")),
            ],
        );
        let plan = RulePlan::for_rule(&rule);
        let order: Vec<usize> = plan
            .operators()
            .iter()
            .map(AtomOperator::positive_position)
            .collect();
        assert_eq!(order, [1, 0], "the constant-subject atom drives the join");
        // The driver has a bound subject; the second atom's subject is bound by it.
        assert_eq!(plan.operator_at(1).index(), IndexChoice::Subject);
        assert_eq!(plan.operator_at(0).index(), IndexChoice::Subject);
        assert_eq!(plan.operator_at(1).body_index(), 1);
        assert_eq!(
            plan.operator_at(1).kernel(),
            &AtomKernel::ConstVar {
                subject: "<https://example.org/a>".to_owned(),
                // Slots follow AUTHORED body order, not execution order: `?Y` first
                // occurs in body atom 0, so it holds slot 0 even though the atom that
                // binds it runs second.
                object_slot: 0,
            }
        );
        // Restoring authored order is a swap program, applied in place.
        assert_eq!(plan.operator_source_order_swaps(), [(0, 1)]);
    }

    /// Every kernel shape is selected statically, and a fully ground atom becomes a
    /// membership probe with both surfaces rendered once.
    #[test]
    fn plan_lowers_every_kernel_shape() {
        let a = PlanTerm::iri("https://example.org/a");
        let lit = PlanTerm::literal("\"7\"^^<http://www.w3.org/2001/XMLSchema#integer>");
        let rule = PlanRule::new(
            atom("?X", S, "?Y"),
            vec![
                atom("?X", P, "?Y"),
                PlanAtom::positive(v("?X"), Q, lit.clone()),
                PlanAtom::positive(a.clone(), R, v("?Y")),
                PlanAtom::positive(a, R, lit),
            ],
        );
        let plan = RulePlan::for_rule(&rule);
        let kernels: Vec<&AtomKernel> = (0..4).map(|i| plan.operator_at(i).kernel()).collect();
        assert!(matches!(kernels[0], AtomKernel::Vars { .. }));
        assert!(matches!(kernels[1], AtomKernel::VarConst { .. }));
        assert!(matches!(kernels[2], AtomKernel::ConstVar { .. }));
        assert_eq!(
            kernels[3],
            &AtomKernel::Consts {
                subject: "<https://example.org/a>".to_owned(),
                object: "\"7\"^^<http://www.w3.org/2001/XMLSchema#integer>".to_owned(),
            }
        );
    }

    /// An unbound atom is a full scan; once a variable is bound the same atom becomes a
    /// one-column probe; two bound columns is the unique-key probe.
    #[test]
    fn index_choice_tracks_the_binding_frontier() {
        let rule = PlanRule::new(
            atom("?X", S, "?Z"),
            vec![
                atom("?X", P, "?Y"),
                atom("?X", Q, "?Y"),
                atom("?Y", R, "?Z"),
            ],
        );
        let plan = RulePlan::for_rule(&rule);
        // Two atoms over the same (X, Y) pair are an intersection, never a cycle.
        assert!(!plan.has_cyclic_subplan());
        assert_eq!(plan.operator_at(0).index(), IndexChoice::Any);
        assert_eq!(plan.operator_at(1).index(), IndexChoice::Both);
        assert_eq!(plan.operator_at(2).index(), IndexChoice::Subject);
    }

    /// The index shape and the adornment lattice are one vocabulary: every shape round
    /// trips through its arity-2 pattern, and the codes are the canonical adornments.
    #[test]
    fn index_choice_round_trips_through_the_adornment_lattice() {
        let cases = [
            (IndexChoice::Any, "ff"),
            (IndexChoice::Subject, "bf"),
            (IndexChoice::Object, "fb"),
            (IndexChoice::Both, "bb"),
        ];
        for (choice, code) in cases {
            let pattern = choice.pattern();
            assert_eq!(pattern.code(), code, "{choice:?}");
            assert_eq!(IndexChoice::from_pattern(pattern), choice);
            assert_eq!(BindingPattern::from_code(code), pattern);
        }
        // The lattice order matches the index order: a full scan is the most general
        // shape and the unique-key probe the most specific.
        assert!(
            IndexChoice::Any
                .pattern()
                .subsumes(IndexChoice::Subject.pattern())
        );
        assert!(
            IndexChoice::Subject
                .pattern()
                .subsumes(IndexChoice::Both.pattern())
        );
        assert_eq!(
            IndexChoice::Subject
                .pattern()
                .join(IndexChoice::Object.pattern()),
            IndexChoice::Both.pattern()
        );
        assert_eq!(
            IndexChoice::Subject
                .pattern()
                .meet(IndexChoice::Object.pattern()),
            IndexChoice::Any.pattern()
        );
    }

    #[test]
    #[should_panic(expected = "an index choice is an arity-2 adornment")]
    fn index_choice_rejects_a_non_binary_adornment() {
        let _ = IndexChoice::from_pattern(BindingPattern::from_code("bfb"));
    }

    /// The plan-time shape lowers to a runtime store bound, and a bound position whose
    /// term the store has never interned short-circuits to "no rows".
    #[test]
    fn index_choice_lowers_to_a_store_bound() {
        let s = TermId::from_index(0);
        let o = TermId::from_index(1);
        assert_eq!(IndexChoice::Any.bound(None, None), Some(Bound::Any));
        assert_eq!(
            IndexChoice::Subject.bound(Some(s), None),
            Some(Bound::Subject(s))
        );
        assert_eq!(
            IndexChoice::Object.bound(None, Some(o)),
            Some(Bound::Object(o))
        );
        assert_eq!(
            IndexChoice::Both.bound(Some(s), Some(o)),
            Some(Bound::Both(s, o))
        );
        // A missing term on a bound position means the selection is empty.
        assert_eq!(IndexChoice::Subject.bound(None, Some(o)), None);
        assert_eq!(IndexChoice::Both.bound(Some(s), None), None);
        assert_eq!(IndexChoice::Both.bound(None, None), None);
    }

    /// The swap program restores authored order from ANY execution order, in place, and
    /// emits no swap when the order is already authored.
    #[test]
    fn restore_body_order_swaps_is_a_correct_in_place_program() {
        let identity: Vec<usize> = (0..7).collect();
        assert!(restore_body_order_swaps(&identity).is_empty());
        for seed in 0..32u64 {
            let order = permute(&identity, seed);
            let swaps = restore_body_order_swaps(&order);
            let mut applied = order.clone();
            for &(a, b) in &swaps {
                applied.swap(a, b);
            }
            assert_eq!(applied, identity, "seed {seed}: order restored");
            assert!(
                swaps.len() < identity.len(),
                "seed {seed}: at most n-1 swaps"
            );
        }
    }

    /// A triangle is certified: one leapfrog group covering all three atoms, with a
    /// deterministic descent order over all three variables.
    #[test]
    fn cyclic_certification_promotes_a_triangle() {
        let rule = triangle_rule(triangle_body());
        let plan = RulePlan::for_rule(&rule);
        assert!(plan.has_cyclic_subplan());
        let groups = plan.join_groups();
        assert_eq!(groups.len(), 1);
        let JoinGroup::Leapfrog(cyclic) = &groups[0] else {
            panic!("the triangle must be promoted to a multiway group");
        };
        assert_eq!(
            cyclic
                .atoms()
                .iter()
                .map(|a| a.positive_position())
                .collect::<Vec<_>>(),
            [0, 1, 2]
        );
        assert_eq!(cyclic.atoms()[0].body_index(), 0);
        assert_eq!(cyclic.variables(), ["?X", "?Y", "?Z"]);
        assert_eq!(cyclic.variable_slots(), [0, 1, 2]);
        assert!(plan.hybrid_source_order_swaps().is_empty());
    }

    /// A tree (every edge a bridge) is never promoted, and neither is a body whose only
    /// repeated edge is a duplicate variable pair.
    #[test]
    fn cyclic_certification_leaves_acyclic_bodies_binary() {
        let path = PlanRule::new(
            atom("?X", S, "?W"),
            vec![
                atom("?X", P, "?Y"),
                atom("?Y", Q, "?Z"),
                atom("?Z", R, "?W"),
            ],
        );
        assert!(!RulePlan::for_rule(&path).has_cyclic_subplan());

        // Three atoms but only two distinct variable edges: an intersection, not a cycle.
        let duplicated = PlanRule::new(
            atom("?X", S, "?Z"),
            vec![
                atom("?X", P, "?Y"),
                atom("?X", Q, "?Y"),
                atom("?Y", R, "?Z"),
            ],
        );
        assert!(!RulePlan::for_rule(&duplicated).has_cyclic_subplan());

        // A self-edge (repeated variable) contributes no graph edge at all.
        let self_edge = PlanRule::new(
            atom("?X", S, "?Z"),
            vec![
                atom("?X", P, "?X"),
                atom("?X", Q, "?Y"),
                atom("?Y", R, "?Z"),
            ],
        );
        assert!(!RulePlan::for_rule(&self_edge).has_cyclic_subplan());
    }

    /// A bridge atom hanging off a triangle stays a binary group: only the edges on a
    /// cycle are promoted, and the plan still covers every positive atom exactly once.
    #[test]
    fn cyclic_certification_keeps_bridge_atoms_binary() {
        let mut body = triangle_body();
        body.push(atom("?Z", S, "?W"));
        let rule = PlanRule::new(atom("?X", S, "?W"), body);
        let plan = RulePlan::for_rule(&rule);
        assert!(plan.has_cyclic_subplan());
        let groups = plan.join_groups();
        assert_eq!(groups.len(), 2, "one cycle group plus one bridge atom");
        assert!(matches!(groups[0], JoinGroup::Leapfrog(_)));
        let JoinGroup::Binary(bridge) = &groups[1] else {
            panic!("the bridge atom must stay binary");
        };
        assert_eq!(bridge.positive_position(), 3);
        assert_eq!(bridge.body_index(), 3);
        assert_eq!(plan.operators().len(), 4, "every positive atom is lowered");
    }

    /// An acyclic plan allocates no group sidecar, and asking for one is a hard error
    /// rather than a fabricated empty answer.
    #[test]
    #[should_panic(expected = "join groups exist only for a certified cyclic plan")]
    fn acyclic_plan_has_no_join_groups() {
        let rule = PlanRule::new(atom("?X", S, "?Y"), vec![atom("?X", P, "?Y")]);
        let _ = RulePlan::for_rule(&rule).join_groups();
    }

    /// Determinism: certification does not depend on the order the body's edges are
    /// discovered in. Over every permutation of the triangle's atoms the SAME component
    /// is promoted, covering the same atoms over the same variables; only the authored
    /// coordinates move with the atoms.
    #[test]
    fn cyclic_certification_is_independent_of_edge_discovery_order() {
        let body = triangle_body();
        for seed in 0..24u64 {
            let permuted = permute(&body, seed);
            let plan = RulePlan::for_rule(&triangle_rule(permuted));
            assert!(plan.has_cyclic_subplan(), "seed {seed}");
            let groups = plan.join_groups();
            assert_eq!(groups.len(), 1, "seed {seed}");
            let JoinGroup::Leapfrog(cyclic) = &groups[0] else {
                panic!("seed {seed}: the triangle must still be promoted");
            };
            assert_eq!(
                cyclic
                    .atoms()
                    .iter()
                    .map(|a| a.positive_position())
                    .collect::<Vec<_>>(),
                [0, 1, 2],
                "seed {seed}: the component covers every atom, in authored order"
            );
            let mut variables = cyclic.variables().to_vec();
            variables.sort();
            assert_eq!(variables, ["?X", "?Y", "?Z"], "seed {seed}");
            assert_eq!(
                cyclic.variable_slots().len(),
                3,
                "seed {seed}: every descent variable has a frame slot"
            );
        }
    }

    /// Determinism: planning is a pure function of the program. Repeated compilations of
    /// the same rules produce structurally identical plans.
    #[test]
    fn planning_is_reproducible() {
        let rules = [
            triangle_rule(triangle_body()),
            PlanRule::new(
                atom("?X", Q, "?Y"),
                vec![atom("?X", P, "?Y"), PlanAtom::negated(v("?Y"), R, v("?X"))],
            ),
        ];
        let reference: Vec<RulePlan> = rules.iter().map(RulePlan::for_rule).collect();
        for _ in 0..8 {
            let again: Vec<RulePlan> = rules.iter().map(RulePlan::for_rule).collect();
            assert_eq!(again, reference, "the same rules always plan identically");
        }
    }

    // ── Stratification and the type-state pipeline ──────────────────────────────

    #[test]
    fn stratify_lifts_a_negated_dependency_one_stratum() {
        // q :- p.   r :- q, not p.
        let rules = vec![
            PlanRule::new(atom("?X", Q, "?Y"), vec![atom("?X", P, "?Y")]),
            PlanRule::new(
                atom("?X", R, "?Y"),
                vec![atom("?X", Q, "?Y"), PlanAtom::negated(v("?X"), P, v("?Y"))],
            ),
        ];
        let strata = stratify(&rules).expect("the program is stratifiable");
        assert_eq!(strata[P], 0);
        assert_eq!(strata[Q], 0);
        assert_eq!(
            strata[R], 1,
            "a negated read sits strictly below its reader"
        );
    }

    #[test]
    fn stratify_rejects_negation_inside_a_cycle() {
        // p :- not q.   q :- p.  — the negative edge is inside a cycle.
        let rules = vec![
            PlanRule::new(
                atom("?X", P, "?Y"),
                vec![PlanAtom::negated(v("?X"), Q, v("?Y"))],
            ),
            PlanRule::new(atom("?X", Q, "?Y"), vec![atom("?X", P, "?Y")]),
        ];
        assert_eq!(stratify(&rules), None);
    }

    #[test]
    fn stratify_admits_positive_recursion() {
        // Transitive closure: p :- p, p. A positive cycle is one stratum.
        let rules = vec![PlanRule::new(
            atom("?X", P, "?Z"),
            vec![atom("?X", P, "?Y"), atom("?Y", P, "?Z")],
        )];
        let strata = stratify(&rules).expect("positive recursion is stratifiable");
        assert_eq!(strata[P], 0);
    }

    #[test]
    fn stratify_of_an_empty_program_is_empty() {
        assert_eq!(stratify(&[]), Some(BTreeMap::new()));
    }

    /// The pipeline is the only way to reach an [`Executable`], and it memoizes the
    /// per-stratum grouping, the per-rule plans and the head-predicate set.
    #[test]
    fn pipeline_seals_a_stratified_planned_program() {
        let rules = vec![
            PlanRule::new(atom("?X", Q, "?Y"), vec![atom("?X", P, "?Y")]),
            PlanRule::new(
                atom("?X", R, "?Y"),
                vec![atom("?X", Q, "?Y"), PlanAtom::negated(v("?X"), P, v("?Y"))],
            ),
        ];
        let exe = Parsed::new(rules)
            .stratify()
            .expect("stratifiable")
            .plan()
            .into_executable();

        assert_eq!(exe.stratum_count(), 2);
        assert!(!exe.stratum_is_empty(0));
        assert_eq!(exe.stratum_rule_indices(0), [0]);
        assert_eq!(exe.stratum_rule_indices(1), [1]);
        assert_eq!(
            exe.head_predicates(),
            &[Q.to_owned(), R.to_owned()].into_iter().collect()
        );
        assert_eq!(
            exe.stratum_head_predicates(1).collect::<Vec<_>>(),
            vec![R],
            "stratum 1 derives exactly r"
        );
        let (rule, plan) = exe.rule_entry(1);
        assert_eq!(rule.head().predicate(), R);
        assert_eq!(rule.body().len(), 2);
        assert!(rule.body()[1].is_negated());
        assert_eq!(rule.body()[1].object(), &v("?Y"));
        assert_eq!(plan.positive(), [0]);
        assert_eq!(plan.negated(), [1]);
    }

    /// A non-stratifiable program never reaches the executor: the pipeline stops at
    /// stage 1 → 2, and there is no other constructor.
    #[test]
    fn pipeline_refuses_a_non_stratifiable_program() {
        let rules = vec![
            PlanRule::new(
                atom("?X", P, "?Y"),
                vec![PlanAtom::negated(v("?X"), Q, v("?Y"))],
            ),
            PlanRule::new(atom("?X", Q, "?Y"), vec![atom("?X", P, "?Y")]),
        ];
        assert!(Parsed::new(rules).stratify().is_none());
    }

    /// Rules that all land in one stratum keep authored program order within it — the
    /// executor's stable firing order.
    #[test]
    fn pipeline_preserves_program_order_within_a_stratum() {
        let rules: Vec<PlanRule> = (0..5)
            .map(|i| {
                PlanRule::new(
                    atom("?X", P, "?Y"),
                    vec![atom("?X", &format!("https://example.org/b{i}"), "?Y")],
                )
            })
            .collect();
        let exe = Parsed::new(rules)
            .stratify()
            .expect("stratifiable")
            .plan()
            .into_executable();
        assert_eq!(exe.stratum_count(), 1);
        assert_eq!(exe.stratum_rule_indices(0), [0, 1, 2, 3, 4]);
    }

    #[test]
    #[should_panic(expected = "a rule head may not be negated")]
    fn a_negated_head_is_not_a_rule() {
        let _ = PlanRule::new(PlanAtom::negated(v("?X"), P, v("?Y")), Vec::new());
    }

    /// The bridge finder is exact on the shapes certification depends on: a path is all
    /// bridges, a cycle has none, and a lollipop has exactly its stick.
    #[test]
    fn bridge_edges_identifies_exactly_the_non_cycle_edges() {
        // Path 0-1-2: both edges are bridges.
        assert_eq!(
            bridge_edges(3, &[(0, 1), (1, 2)]),
            [0, 1].into_iter().collect()
        );
        // Triangle: no bridges.
        assert!(bridge_edges(3, &[(0, 1), (1, 2), (2, 0)]).is_empty());
        // Lollipop: triangle 0-1-2 plus the stick 2-3.
        assert_eq!(
            bridge_edges(4, &[(0, 1), (1, 2), (2, 0), (2, 3)]),
            BTreeSet::from([3])
        );
        // Two parallel edges between the same pair form a cycle: neither is a bridge.
        assert!(bridge_edges(2, &[(0, 1), (0, 1)]).is_empty());
        // A disconnected graph is handled component by component.
        assert_eq!(
            bridge_edges(4, &[(0, 1), (2, 3)]),
            [0, 1].into_iter().collect()
        );
        assert!(bridge_edges(0, &[]).is_empty());
    }

    /// The disjoint-set forest merges exactly the connected components.
    #[test]
    fn union_find_merges_connected_components() {
        let mut uf = UnionFind::new(6);
        uf.union(0, 1);
        uf.union(1, 2);
        uf.union(4, 5);
        assert_eq!(uf.find(0), uf.find(2));
        assert_eq!(uf.find(4), uf.find(5));
        assert_ne!(uf.find(0), uf.find(3));
        assert_ne!(uf.find(0), uf.find(4));
        // Merging an already-merged pair is a no-op.
        uf.union(2, 0);
        assert_eq!(uf.find(0), uf.find(1));
    }
}
