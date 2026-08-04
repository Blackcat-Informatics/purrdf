// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The `SHOIQ(D)` HYPERTABLEAU: the OWL-Direct decision core.
//!
//! A from-scratch implementation of the hypertableau calculus (Motik, Shearer & Horrocks,
//! "Hypertableau Reasoning for Description Logics", JAIR 33, 2009) over the DL-clauses
//! [`crate::owl_dl::clause`] derives, for the `SHOIQ(D)` fragment: the boolean connectives,
//! existential/universal restrictions, transitive roles (`S`), role hierarchies (`H`),
//! qualified number restrictions (`Q`), inverse roles (`I`), nominals (`O`), and a CONCRETE
//! domain (`D`) of datatype values. Beyond the letters it decides self-restrictions
//! (`owl:hasSelf`, and through them the reflexive/irreflexive role axioms), role disjointness
//! and asymmetry; the one `SROIQ` role feature it does NOT decide is
//! `owl:propertyChainAxiom`, which is a named [`Construct::PropertyChain`](crate::Construct)
//! boundary rather than a silent drop. Algorithms are not copyrightable; this code is
//! original.
//!
//! # What makes it a hypertableau rather than a tableau
//!
//! A concept-tree tableau reads STRUCTURE at search time: it looks at each concept in each
//! node's label, and for a `⊔` it branches — so a terminology of `n` disjunctions branches on
//! every node whether or not anything made the disjunction relevant. This calculus compiles
//! the structure out in front ([`crate::owl_dl::clause`]) and then applies exactly three
//! rules:
//!
//! | rule | when | what it does |
//! |---|---|---|
//! | **Hyperresolution** | a clause BODY matches the graph | derives the clause's head |
//! | **`≥`-rule** | an at-least head atom is unsatisfied at an UNBLOCKED node | mints anonymous witnesses, pairwise distinct |
//! | **`⊔`-rule** | a derived head has more than one disjunct, none satisfied | branches, depth-first, in clause-and-disjunct order |
//!
//! Everything the incumbent spread over ten rules and eight clash triggers is one of those
//! three, because the clause set carries the difference:
//!
//! * `⊓`, absorption and the `∀`-propagation are hyperresolution with an
//!   [`Atomic`](HeadForm::Atomic)/[`Conjunctive`](HeadForm::Conjunctive) head — and `∀r.C`'s
//!   body `c(x) ∧ r(x,y)` is matched against an EDGE, so one derivation step consumes the
//!   whole rule instance instead of a label scan per node per round;
//! * `⊥`, a complementary pair `C, ¬C`, a negated nominal naming the node's own individual, a
//!   negated self restriction on a node with that loop, an asymmetric role's symmetric pair
//!   and a disjoint role pair sharing one pair are all clauses with an EMPTY head
//!   ([`Inconsistency`](HeadForm::Inconsistency)) — a clash is a derivation of `false`, not a
//!   separate detector;
//! * a `≤n r.C` violation is the `⊔`-rule with nothing left to choose: its clause head is the
//!   disjunction `⋁_{i<j} yᵢ ≈ yⱼ` over `n + 1` counted successors, so when every pair is
//!   already recorded `≠` the branch list is empty and the state closes;
//! * the `o`-rule is the clause `{a₁…aₙ}(x) → x ≈ a₁ ∨ … ∨ x ≈ aₙ`: an identification per
//!   member, deterministic for a singleton, a branch for a set — never a name comparison, so
//!   the absence of a unique name assumption is a property of the clause and not of a rule
//!   that remembers to honour it.
//!
//! The one thing not compiled into clauses is the CONCRETE domain, and deliberately: whether
//! `r₁ ∩ … ∩ rₘ ∩ ¬s₁ ∩ … ∩ ¬sₖ` holds a value is a question about the whole set of ranges on
//! a node rather than a clause of bounded arity, and it is answered by
//! [`crate::owl_dl::data`] against `purrdf-xsd`'s value spaces. That decision procedure —
//! and the value-class identity that decides when two literal nodes are one element of `Δ_D` —
//! is shared verbatim with the incumbent through [`Graph`], so the two calculi cannot
//! disagree about the data domain.
//!
//! # Termination: ANYWHERE pairwise blocking
//!
//! A tree node `x` is **directly blocked** by a node `y` when
//!
//! 1. `y` has a strictly smaller node index than `x` — the well-founded order that makes
//!    mutual blocking impossible;
//! 2. neither is a root (nominal) node, and both have a predecessor;
//! 3. `y` is not itself blocked;
//! 4. `L(x) = L(y)`, `L(pred(x)) = L(pred(y))`, and the two incoming edges carry the same
//!    `(property, direction)` pair.
//!
//! and **blocked** when it is directly blocked or its predecessor is blocked. A blocked node
//! is not expanded by the `≥`-rule; every other rule still applies to it.
//!
//! Conditions (2) and (4) are *pairwise* (double) blocking, which is the discipline inverse
//! roles force: with `I` in the language a successor's label can constrain its predecessor, so
//! a blocked node may only stand in for one whose PREDECESSOR looks the same and is reached
//! the same way — otherwise the model construction that unravels a blocked node by copying its
//! blocker's successors would import a successor whose `∀r⁻`-obligation the predecessor does
//! not satisfy. Condition (1) is what makes this ANYWHERE blocking rather than the incumbent's
//! ancestor blocking: the blocker need not be on `x`'s branch, only earlier. That is a strictly
//! stronger blocking condition — every ancestor blocker is also an anywhere blocker — so it
//! cannot make a search explore more, and on a terminology whose witnesses repeat across
//! sibling subtrees (the common case: every `∃r.C` in the same terminology generates the same
//! label) it collapses the graph to one representative per label signature instead of one per
//! branch.
//!
//! ## Why it is sufficient for this fragment
//!
//! Soundness of blocking is the direction that matters and it is unconditional: blocking only
//! ever WITHHOLDS a `≥`-rule application, so a clash-free blocked graph is a graph the
//! unblocked calculus could not have closed either — and a clash the calculus does derive is
//! derived from rules that are each model-preserving, so it refutes the knowledge base whether
//! anything was blocked or not.
//!
//! Completeness is the direction that needs the conditions above. A clash-free saturated graph
//! is unravelled into a model by giving every blocked node the successors of its blocker,
//! repeatedly. Condition (4) makes that substitution invisible to every clause: a clause body
//! is a conjunction of concept atoms on nodes and role atoms on edges, and `x` and `y` agree
//! on their own label, on their predecessor's label and on the connecting role, so any body
//! instance satisfied at `x` is satisfied at `y`, whose head was therefore already derived.
//! Condition (2) is what keeps nominals out of it: a root node is never blocked, so the
//! finitely many named individuals are all expanded, and a tree node that acquires a nominal
//! is identified with a root by the `o`-clause before it can stand in for anything. Condition
//! (3) prevents a cycle of nodes justifying one another with no expanded representative, and
//! (1) makes the "not blocked" test well-founded.
//!
//! One empirical honesty about condition (4)'s predecessor-label half: no knowledge base is
//! KNOWN that separates it from label-only blocking in this rule set. A deliberate hunt —
//! the generated corpora run under a label-only mutation, plus a hand-targeted family of
//! inverse-role/∀⁻ chains (kept as a permanent differential test in the oracle) — changed
//! tallies but never a verdict. The structural reason narrows the classic separation:
//! blocking here withholds ONLY `≥`-rule applications, while every clause body — including
//! the `∀r⁻` back-propagation whose obligations the pairwise condition guards in the
//! published calculus — keeps matching blocked nodes, and blocking is recomputed every
//! round as labels grow. The condition is kept because it is the published calculus's and
//! costs one comparison; what must not be claimed is that the test corpus DEMONSTRATES its
//! necessity, and this paragraph is that claim's replacement.
//!
//! Termination follows from (1) and (4) alone: a node's blocking signature is
//! `(L(x), L(pred(x)), incoming(x))`, drawn from the FIXED, finalized concept table, so there
//! are finitely many signatures; the first node with a given signature is unblocked and every
//! later one is directly blocked; and only unblocked nodes are expanded, each by boundedly
//! many successors (one per at-least atom, times its number). The round cap
//! ([`step_cap`](crate::owl_dl::graph::step_cap)) remains a hard backstop, so a termination bug surfaces as an
//! [`EntailError::Build`] rather than a hang.
//!
//! ## The one boundary this blocking does not cross
//!
//! `SHOIQ`'s interaction of nominals, inverse roles and number restrictions can require a
//! nominal-introduction rule (Horrocks & Sattler's `NN`-rule) to be complete for knowledge
//! bases where a `≤n r⁻` restriction on a NAMED individual bounds how many anonymous
//! predecessors it may have. This calculus has no such rule, and neither does the incumbent
//! concept-tree tableau — a SHARED absence, which means both may report `consistent` for
//! such a knowledge base where the full calculus refutes it, and the differential between
//! them is structurally blind to exactly this corner: two calculi missing the same rule do
//! not disagree about its consequences. What the differential establishes is zero divergence
//! over its corpora, not agreement on every input. It is recorded
//! here as the honest limit of the decision core rather than presented as decided; nothing in
//! this crate reports a subsumption on the strength of a `consistent` verdict alone that the
//! incumbent would not report too.
//!
//! # Two ways to ask
//!
//! [`consistent`] answers `bool` and turns an exhausted cap into [`EntailError::Build`] — the
//! shape the query-directed materialization layer wants, where a truncated search has no
//! honest answer to return. [`decide`] answers a [`Decision`], which carries the round count
//! and an `exhausted` flag instead of throwing one; that is what the reasoner services need,
//! because a service that ran a thousand sub-questions must be able to report "these are
//! decided, that one ran out" rather than lose the whole run to one hard instance.
//!
//! # Determinism
//!
//! The clause set is derived in ascending concept-id order; a round visits nodes in ascending
//! index order and, at each node, its label's concepts in ascending order and their clauses in
//! derivation order; a role atom is matched over
//! [`Graph::neighbors`](crate::owl_dl::graph::Graph::neighbors), which is first-seen edge
//! order; and the `⊔`-rule branches in authored disjunct order. Nothing is read out of a hash
//! map and nothing consults a clock, so a [`Decision`] — verdict, round count and exhausted
//! flag alike — is byte-identical run to run and on wasm32.

use purrdf_datalog::clause::HeadForm;

use crate::EntailError;
use crate::owl_dl::Kb;
use crate::owl_dl::clause::{BodyAtom, ClauseSet, DlClause, HeadAtom, derive};
use crate::owl_dl::concept::Role;
use crate::owl_dl::graph::{Assumptions, Decision, Exhausted, Graph, State, find, step_cap};

/// One head atom with its variables replaced by the nodes a match bound them to.
///
/// The `⊔`-rule needs to hold a branch's alternatives across a state clone, so a derived head
/// is grounded once and then applied — rather than re-matching the clause in each branch,
/// which would re-derive the same instance from a state the branch has already changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ground {
    /// Add a concept to a node's label.
    Concept(usize, u32),
    /// Give a node an `r`-edge to itself.
    SelfLoop(usize, Role),
    /// Ensure `n` pairwise-distinct `role`-neighbours satisfying the filler.
    AtLeast(usize, u32, Role, u32),
    /// Identify two nodes.
    Equal(usize, usize),
    /// Identify a node with an individual's root.
    EqualIndividual(usize, u32),
}

/// One level of the search: the state a `⊔`-rule branched from, and its untried disjuncts.
///
/// This is the state a call frame of the recursive form held implicitly, written down so
/// that the stack it lives on is the heap — see [`Hyper::solve`].
struct Branches {
    /// The saturated state the alternatives below are applied to.
    ///
    /// Held rather than recomputed because an alternative starts from a CLONE of it: a
    /// sibling must not see what the branch before it derived.
    state: State,
    /// The disjuncts not yet tried, in authored order.
    alternatives: std::vec::IntoIter<Vec<Ground>>,
}

/// The hypertableau driver: the graph operations, the clause set, and a budget.
struct Hyper<'a> {
    /// The completion graph operations over the knowledge base.
    g: Graph<'a>,
    /// The DL-clauses derived from it.
    clauses: ClauseSet,
    /// Derivation rounds consumed so far.
    steps: u64,
    /// Hard round cap; exceeding it is a hard error (a termination-bug backstop).
    cap: u64,
    /// Whether the caller's stop signal — not the cap — ended the search.
    ///
    /// Recorded on the driver rather than carried in [`Exhausted`] so that the private
    /// `Result<_, Exhausted>` plumbing every rule and branch is written against stays
    /// exactly what it was: the two stops travel out of the search identically and are
    /// separated once, where the [`Decision`] is assembled.
    stopped: bool,
}

/// Decide whether the knowledge base plus `assumptions` has a consistent completion,
/// spending at most `cap` derivation rounds.
pub(crate) fn decide(kb: &Kb, assumptions: &Assumptions<'_>, cap: u64) -> Decision {
    let mut h = Hyper::new(kb, cap);
    let st = h.g.init_state(assumptions);
    match h.solve(st) {
        Ok(consistent) => Decision {
            consistent,
            steps: h.steps,
            exhausted: false,
            stopped: false,
        },
        // One private refusal, two public facts: `stopped` is what the driver recorded when
        // it turned the poll into an `Exhausted`, and `exhausted` is therefore reserved for
        // the cap it is named after.
        Err(Exhausted) => Decision {
            consistent: false,
            steps: h.steps,
            exhausted: !h.stopped,
            stopped: h.stopped,
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
    if decision.stopped {
        return Err(EntailError::Stopped);
    }
    if decision.exhausted {
        return Err(EntailError::Build(
            "OWL-Direct hypertableau exceeded its step cap (possible non-termination)".to_owned(),
        ));
    }
    Ok(decision.consistent)
}

impl<'a> Hyper<'a> {
    /// Build a driver over `kb` bounded by `cap` rounds, deriving its clause set.
    fn new(kb: &'a Kb, cap: u64) -> Self {
        let g = Graph::new(kb);
        Self {
            clauses: derive(g.kb()),
            g,
            steps: 0,
            cap,
            stopped: false,
        }
    }

    /// The depth-first, deterministic search: saturate, then branch on a derived disjunction.
    ///
    /// One level of the search is one `⊔`-rule application, so the search is as DEEP as the
    /// knowledge base has open disjunctions — twenty thousand individuals under one union
    /// class is twenty thousand levels. As call frames that is a stack overflow, which is
    /// not a refusal a caller can catch: the process aborts, nothing unwinds, and a host
    /// embedding this library dies with it. The round cap is no defence, because it bounds
    /// derivation ROUNDS and a level costs one of them. So the search carries its own
    /// [`Branches`] stack on the heap and its reachable depth is a function of memory
    /// rather than of a thread's stack rlimit — which differs by an order of magnitude
    /// between a native binary and `wasm32` and is not something a library can read or
    /// raise. [`Exhausted`] stays exactly what it was.
    ///
    /// The order is the recursion's, so the [`Decision`] is unchanged: a branch is explored
    /// to exhaustion before its next sibling is tried, siblings go in authored disjunct
    /// order, and the first clash-free completion ends the search.
    fn solve(&mut self, st: State) -> Result<bool, Exhausted> {
        let mut stack: Vec<Branches> = Vec::new();
        // The state to saturate and expand next — what the recursive form passed down.
        let mut pending = Some(st);
        loop {
            let Some(mut st) = pending.take() else {
                // Nothing to descend into, so back up: take the next alternative of the
                // deepest level that still has one, and drop a level that has none.
                let Some(level) = stack.last_mut() else {
                    // Every alternative of every level clashed.
                    return Ok(false);
                };
                match level.alternatives.next() {
                    Some(disjunct) => {
                        let mut next = level.state.clone();
                        if self.apply(&mut next, &disjunct) {
                            pending = Some(next);
                        }
                    }
                    None => {
                        stack.pop();
                    }
                }
                continue;
            };
            if !self.saturate(&mut st)? {
                // A clash: this alternative is dead, and the loop backs up.
                continue;
            }
            match self.find_branch(&st) {
                Some(alternatives) => stack.push(Branches {
                    state: st,
                    alternatives: alternatives.into_iter(),
                }),
                // No disjunction left to branch on: a clash-free completion, which is the
                // answer for the whole search rather than for this level alone.
                None => return Ok(true),
            }
        }
    }

    /// Apply hyperresolution and the `≥`-rule to a fixpoint.
    ///
    /// Returns `Ok(false)` on a clash, `Ok(true)` at a clash-free fixpoint over the
    /// non-disjunctive clauses.
    fn saturate(&mut self, st: &mut State) -> Result<bool, Exhausted> {
        loop {
            self.tick()?;
            if self.concrete_domain_clashes(st) {
                st.clash = true;
                return Ok(false);
            }
            let changed = self.round(st);
            Self::check_clique(st)?;
            if st.clash {
                return Ok(false);
            }
            if !changed {
                return Ok(true);
            }
        }
    }

    /// Consume one round against the cap.
    /// Convert a mid-rule clique-budget exhaustion into the search's own exhaustion.
    fn check_clique(st: &State) -> Result<(), Exhausted> {
        if st.clique_exhausted.get() {
            return Err(Exhausted);
        }
        Ok(())
    }

    fn tick(&mut self) -> Result<(), Exhausted> {
        // The caller's stop signal, polled once per derivation round — the same boundary the
        // cap is charged at, so a search that can be capped can be stopped.
        if self.g.kb().stopped() {
            self.stopped = true;
            return Err(Exhausted);
        }
        if self.steps >= self.cap {
            return Err(Exhausted);
        }
        self.steps += 1;
        Ok(())
    }

    /// Whether any node's CONCRETE-domain constraints have no solution.
    ///
    /// The one decision this calculus does not take through a clause — see the module docs —
    /// and it is [`crate::owl_dl::data`]'s answer, shared verbatim with the incumbent.
    fn concrete_domain_clashes(&self, st: &State) -> bool {
        (0..st.nodes.len()).any(|x| find(st, x) == x && self.g.data_clashes(st, x))
    }

    /// One derivation round: every non-disjunctive clause instance, applied once.
    ///
    /// Matches are collected before they are applied, because applying one — a merge, or a
    /// minted witness — changes the graph the others were found in. A match invalidated that
    /// way is re-checked against the current state before it is applied (every node index is
    /// resolved through [`find`]), so the worst a stale match can be is redundant.
    fn round(&self, st: &mut State) -> bool {
        let blocked = self.blocking(st);
        let mut changed = false;
        for x in 0..st.nodes.len() {
            if find(st, x) != x {
                continue;
            }
            let triggers: Vec<u32> = st.nodes[x].label.iter().copied().collect();
            for concept in triggers {
                for &index in self.clauses.triggered_by(concept) {
                    changed |= self.fire(st, index, x, &blocked);
                    if st.clash {
                        return changed;
                    }
                }
            }
            for &index in self.clauses.untriggered() {
                changed |= self.fire(st, index, x, &blocked);
                if st.clash {
                    return changed;
                }
            }
        }
        changed
    }

    /// Apply every match of clause `index` rooted at node `x`, if its head is not a
    /// disjunction. Returns whether the graph changed.
    fn fire(&self, st: &mut State, index: usize, x: usize, blocked: &[bool]) -> bool {
        let clause = self.clauses.clause(index);
        let form = clause.head_form();
        if form == HeadForm::Disjunctive {
            return false;
        }
        let mut instances: Vec<Vec<usize>> = Vec::new();
        Self::for_each_match(&self.g, st, clause, x, &mut |frame| {
            instances.push(frame.to_vec());
            // An empty head is `false`: the first match refutes the state, so there is
            // nothing to learn from the rest.
            form == HeadForm::Inconsistency
        });
        if instances.is_empty() {
            return false;
        }
        if form == HeadForm::Inconsistency {
            st.clash = true;
            return false;
        }
        let mut changed = false;
        for frame in instances {
            // A non-disjunctive head has exactly one disjunct, and `ground_head` cannot expand
            // it into more: the schematic pair atom is what makes a clause disjunctive.
            let disjunct = ground(&clause.head[0], &frame);
            if self.satisfied(st, &disjunct) {
                continue;
            }
            // The `≥`-rule is the one rule blocking withholds. A blocked node's at-least
            // obligation is not satisfied and not discharged: it is deferred to the blocker,
            // which is what the model construction in the module docs makes good.
            if disjunct.iter().any(
                |atom| matches!(atom, Ground::AtLeast(node, ..) if is_blocked(st, blocked, *node)),
            ) {
                continue;
            }
            // The head was NOT satisfied, so asserting it moves the graph: every atom's
            // assertion is a change exactly when its satisfaction test was false (a concept
            // enters a label, a loop appears, a witness is minted, two nodes become one), which
            // is what makes this `true` rather than a second "did anything happen" flag —
            // and what makes the round loop terminate instead of re-firing a satisfied head.
            changed = true;
            if !self.apply(st, &disjunct) {
                return changed;
            }
        }
        changed
    }

    /// The first derived head disjunction none of whose disjuncts holds, grounded — the
    /// `⊔`-rule's alternatives, in clause-and-disjunct order.
    fn find_branch(&self, st: &State) -> Option<Vec<Vec<Ground>>> {
        for x in 0..st.nodes.len() {
            if find(st, x) != x {
                continue;
            }
            let triggers: Vec<u32> = st.nodes[x].label.iter().copied().collect();
            for concept in triggers {
                for &index in self.clauses.triggered_by(concept) {
                    if let Some(branch) = self.branch_of(st, index, x) {
                        return Some(branch);
                    }
                }
            }
            for &index in self.clauses.untriggered() {
                if let Some(branch) = self.branch_of(st, index, x) {
                    return Some(branch);
                }
            }
        }
        None
    }

    /// The grounded alternatives of clause `index` at node `x`, if it is a disjunction with no
    /// satisfied disjunct.
    fn branch_of(&self, st: &State, index: usize, x: usize) -> Option<Vec<Vec<Ground>>> {
        let clause = self.clauses.clause(index);
        if clause.head_form() != HeadForm::Disjunctive {
            return None;
        }
        let mut found: Option<Vec<Vec<Ground>>> = None;
        Self::for_each_match(&self.g, st, clause, x, &mut |frame| {
            let disjuncts = ground_head(&clause.head, frame);
            if disjuncts
                .iter()
                .any(|disjunct| self.satisfied(st, disjunct))
            {
                return false;
            }
            found = Some(disjuncts);
            true
        });
        found
    }

    /// Whether every atom of a grounded disjunct already holds.
    fn satisfied(&self, st: &State, disjunct: &[Ground]) -> bool {
        disjunct.iter().all(|atom| match *atom {
            Ground::Concept(node, concept) => self.g.has_concept(st, node, concept),
            Ground::SelfLoop(node, role) => self.g.has_self_loop(st, node, role),
            Ground::AtLeast(node, n, role, filler) => {
                self.g.has_at_least(st, node, n, role, filler)
            }
            Ground::Equal(left, right) => find(st, left) == find(st, right),
            Ground::EqualIndividual(node, individual) => {
                st.nodes[find(st, node)].nominals.contains(&individual)
            }
        })
    }

    /// Assert every atom of a grounded disjunct. Returns `false` if the state clashed.
    fn apply(&self, st: &mut State, disjunct: &[Ground]) -> bool {
        for atom in disjunct {
            match *atom {
                Ground::Concept(node, concept) => {
                    self.g.add_concept(st, node, concept);
                }
                Ground::SelfLoop(node, role) => {
                    self.g.add_self_loop(st, node, role);
                }
                Ground::AtLeast(node, n, role, filler) => {
                    self.g.ensure_at_least(st, node, n, role, filler);
                }
                Ground::Equal(left, right) => self.g.merge_nodes(st, left, right),
                Ground::EqualIndividual(node, individual) => {
                    let root = self.g.root(st, individual);
                    self.g.merge_nodes(st, root, node);
                }
            }
            if st.clash {
                return false;
            }
        }
        !st.clash
    }

    /// Call `visit` on every binding frame that satisfies `clause`'s body with variable `0`
    /// bound to `x`; stop early when it answers `true`.
    ///
    /// The join plan is the body's own order, which [`crate::owl_dl::clause`] guarantees binds
    /// every variable before it is used and in ascending variable order, so this is one
    /// left-deep pass with no search over orders — and the frame is a STACK that grows as
    /// variables bind rather than a vector pre-sized to the clause's arity. That is what keeps
    /// a `≤300 r.C` clause from allocating three hundred slots at every node in every round to
    /// discover, one successor in, that the node has two.
    fn for_each_match(
        g: &Graph<'a>,
        st: &State,
        clause: &DlClause,
        x: usize,
        visit: &mut dyn FnMut(&[usize]) -> bool,
    ) -> bool {
        let mut frame = vec![find(st, x)];
        Self::walk(g, st, clause, 0, &mut frame, visit)
    }

    /// Match `clause.body[at..]`, extending `frame`. Returns whether `visit` stopped the walk.
    ///
    /// A variable is BOUND exactly when it is inside `frame`, so binding one is a push and
    /// undoing it is a pop — the invariant `clause::DlClause::is_matchable` asserts.
    fn walk(
        g: &Graph<'a>,
        st: &State,
        clause: &DlClause,
        at: usize,
        frame: &mut Vec<usize>,
        visit: &mut dyn FnMut(&[usize]) -> bool,
    ) -> bool {
        let Some(&atom) = clause.body.get(at) else {
            return visit(frame);
        };
        match atom {
            BodyAtom::Concept { var, concept } => {
                if g.has_concept(st, frame[var as usize], concept) {
                    return Self::walk(g, st, clause, at + 1, frame, visit);
                }
            }
            BodyAtom::Denotes { var, individual } => {
                let node = find(st, frame[var as usize]);
                if st.nodes[node].nominals.contains(&individual) {
                    return Self::walk(g, st, clause, at + 1, frame, visit);
                }
            }
            BodyAtom::Role { from, to, role } => {
                let source = frame[from as usize];
                if (to as usize) < frame.len() {
                    let target = find(st, frame[to as usize]);
                    if g.neighbors(st, source, role).contains(&target) {
                        return Self::walk(g, st, clause, at + 1, frame, visit);
                    }
                } else {
                    for y in g.neighbors(st, source, role) {
                        frame.push(y);
                        let stopped = Self::walk(g, st, clause, at + 1, frame, visit);
                        frame.pop();
                        if stopped {
                            return true;
                        }
                    }
                }
            }
            BodyAtom::Successors {
                role,
                filler,
                first,
                count,
            } => {
                // The frame is a stack, so a variable's index IS its position in it: this atom
                // binds `first ..= first + count - 1`, which can only be the next `count`
                // pushes. A clause that numbered them otherwise would silently ground its head
                // against the wrong nodes, so the alignment is asserted rather than assumed.
                debug_assert_eq!(
                    first as usize,
                    frame.len(),
                    "a schematic successor atom must bind the frame's next variables"
                );
                // The counted successors of a `≤n` restriction: those `role`-neighbours of
                // variable 0 that satisfy the filler, enumerated `count` at a time in strictly
                // increasing node order — so what is enumerated is the count-element SETS, and
                // a set of size `count` is by construction pairwise different as TERMS (whether
                // they are pairwise `≠` as ELEMENTS is what the head disjunction settles).
                let counted: Vec<usize> = g
                    .neighbors(st, frame[0], role)
                    .into_iter()
                    .filter(|&y| g.has_concept(st, y, filler))
                    .collect();
                let mut sorted = counted;
                sorted.sort_unstable();
                sorted.dedup();
                return Self::walk_subsets(g, st, clause, at, frame, &sorted, 0, count, visit);
            }
        }
        false
    }

    /// Extend `frame` with every strictly-increasing `remaining`-element selection from
    /// `pool[from..]`, continuing the body walk once the selection is complete.
    #[allow(clippy::too_many_arguments)]
    fn walk_subsets(
        g: &Graph<'a>,
        st: &State,
        clause: &DlClause,
        at: usize,
        frame: &mut Vec<usize>,
        pool: &[usize],
        from: usize,
        remaining: u32,
        visit: &mut dyn FnMut(&[usize]) -> bool,
    ) -> bool {
        if remaining == 0 {
            return Self::walk(g, st, clause, at + 1, frame, visit);
        }
        // Not enough successors left to complete the selection: the restriction cannot be
        // violated here, which is the common case and the one that must stay cheap.
        if pool.len() - from < remaining as usize {
            return false;
        }
        for index in from..pool.len() {
            frame.push(pool[index]);
            let stopped = Self::walk_subsets(
                g,
                st,
                clause,
                at,
                frame,
                pool,
                index + 1,
                remaining - 1,
                visit,
            );
            frame.pop();
            if stopped {
                return true;
            }
        }
        false
    }

    /// Which nodes are blocked, by node index — see the module docs for the discipline.
    ///
    /// One pass in ascending index order, carrying the unblocked nodes seen so far as the
    /// candidate blockers. A node is directly blocked when an earlier candidate has its
    /// signature, indirectly blocked when its predecessor is blocked, and a blocked node is
    /// never itself a candidate.
    ///
    /// A predecessor whose representative has a HIGHER index than the node — which only a
    /// merge can produce, since a successor is always created after its predecessor — is read
    /// as unblocked, because its status is not yet known in this pass. That under-blocks in
    /// that one case, which costs expansion and never a verdict: blocking withholds work, so
    /// doing the work anyway is what the calculus would have done without the optimization,
    /// and termination rests on DIRECT blocking alone (the signature argument in the module
    /// docs bounds the unblocked nodes whether or not indirect blocking fires).
    fn blocking(&self, st: &State) -> Vec<bool> {
        let n = st.nodes.len();
        let mut blocked = vec![false; n];
        let mut candidates: Vec<usize> = Vec::new();
        for x in 0..n {
            if find(st, x) != x || st.nodes[x].root {
                continue;
            }
            let Some(parent) = st.nodes[x].parent.map(|p| find(st, p)) else {
                continue;
            };
            let directly = candidates
                .iter()
                .any(|&y| Self::same_signature(st, x, y, parent));
            let indirectly = parent < x && blocked[parent];
            if directly || indirectly {
                blocked[x] = true;
            } else {
                candidates.push(x);
            }
        }
        blocked
    }

    /// Whether `x` (whose predecessor is `parent`) has `y`'s blocking signature: same label,
    /// same predecessor label, same incoming edge.
    fn same_signature(st: &State, x: usize, y: usize, parent: usize) -> bool {
        let Some(other_parent) = st.nodes[y].parent.map(|p| find(st, p)) else {
            return false;
        };
        st.nodes[x].incoming == st.nodes[y].incoming
            && st.nodes[x].label == st.nodes[y].label
            && st.nodes[parent].label == st.nodes[other_parent].label
    }
}

/// Whether the node a grounded atom names is blocked.
///
/// A node minted during the current round is absent from the round's blocking vector and reads
/// as unblocked, which is the same conservative direction [`Hyper::blocking`] documents.
fn is_blocked(st: &State, blocked: &[bool], node: usize) -> bool {
    let node = find(st, node);
    blocked.get(node).copied().unwrap_or(false)
}

/// Ground a clause's whole head against a matched `frame`, expanding the one schematic atom.
///
/// The result is the `⊔`-rule's alternatives in authored order: each `head` entry contributes
/// one disjunct, except [`HeadAtom::EqualSomePair`], which contributes one disjunct per PAIR of
/// the successors it counted — in ascending `(i, j)` order, so the branch order is a function
/// of the clause and the graph and nothing else.
fn ground_head(head: &[Vec<HeadAtom>], frame: &[usize]) -> Vec<Vec<Ground>> {
    let mut out: Vec<Vec<Ground>> = Vec::with_capacity(head.len());
    for disjunct in head {
        match disjunct.as_slice() {
            [HeadAtom::EqualSomePair { first, count }] => {
                let first = *first as usize;
                let count = *count as usize;
                for left in 0..count {
                    for right in (left + 1)..count {
                        out.push(vec![Ground::Equal(
                            frame[first + left],
                            frame[first + right],
                        )]);
                    }
                }
            }
            atoms => out.push(ground(atoms, frame)),
        }
    }
    out
}

/// Replace a head disjunct's variables with the nodes `frame` bound them to.
fn ground(disjunct: &[HeadAtom], frame: &[usize]) -> Vec<Ground> {
    disjunct
        .iter()
        .map(|atom| match *atom {
            HeadAtom::Concept { var, concept } => Ground::Concept(frame[var as usize], concept),
            HeadAtom::SelfLoop { var, role } => Ground::SelfLoop(frame[var as usize], role),
            HeadAtom::AtLeast {
                var,
                n,
                role,
                filler,
            } => Ground::AtLeast(frame[var as usize], n, role, filler),
            HeadAtom::EqualIndividual { var, individual } => {
                Ground::EqualIndividual(frame[var as usize], individual)
            }
            // `ground_head` above expands this atom into one disjunct per pair, so it never
            // reaches an atom-at-a-time grounding: a single `Ground` cannot stand for a
            // disjunction, and inventing one that did is how a case split becomes a
            // conjunction.
            HeadAtom::EqualSomePair { .. } => unreachable!(
                "a schematic pair disjunction is expanded by ground_head, never as one atom"
            ),
        })
        .collect()
}
