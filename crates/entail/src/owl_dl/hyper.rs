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
//! | **`⊔`-rule** | a derived head has more than one disjunct, none satisfied | branches, depth-first, over the FIRST open disjunction, in authored disjunct order |
//!
//! Everything the incumbent spread over ten rules and eight clash triggers is one of those
//! three, because the clause set carries the difference:
//!
//! * `⊓`, the whole ABSORBED terminology (`A ⊑ D`, `∃r.C ⊑ D`, `A ⊓ B ⊑ D`, `rdfs:domain`,
//!   `rdfs:range`) and the `∀`-propagation are hyperresolution with an
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
//! ## The clauses whose head lands on the matched node's PREDECESSOR
//!
//! Absorption ([`crate::owl_dl::absorb`]) authors `∃r.C ⊑ D` re-rooted at the filler, as
//! `C(y) ∧ r⁻(y, x) → D(x)`. The head is therefore asserted on a node the match REACHED
//! rather than on the node the round is visiting, and for a tree node `y` that node is its
//! predecessor. Two things have to be said about that, and neither weakens the argument above.
//!
//! First, DERIVATION. Blocking withholds exactly one rule, the `≥`-rule, and it withholds it
//! at the node whose label carries the at-least concept — an at-least head atom is always on
//! variable `0`, because it comes from a concept clause `c(x) → ≥n r.C(x)` and an absorbed
//! clause's head is a single concept atom, never a counting one. Hyperresolution keeps
//! matching blocked nodes, so `D(pred(y))` is derived at `y` whether or not `y` is blocked.
//! No obligation is deferred onto a predecessor and left unmade.
//!
//! Second, the MODEL CONSTRUCTION. Unravelling replaces a blocked `x` with copies of its
//! blocker `y`'s successors. Take a copy `z′` of a successor `z` of `y`, now attached under
//! `x`, and suppose the clause matches at `z′` — `C ∈ L(z)`, and the connecting edge is an
//! `r`-edge. The same instance matched at `z` in the graph, so `D ∈ L(y)`; and condition (4)'s
//! `L(x) = L(y)` gives `D ∈ L(x)`, which is exactly what the copy needs. The
//! head-on-predecessor direction is discharged by the SAME label equality that discharges the
//! ordinary direction — no new condition, and none of (1)–(4) becomes dispensable.
//!
//! Where the predecessor-label half of (4) begins to carry weight is a CHAIN of such clauses:
//! the `D` derived on `pred(x)` may itself guard another head-on-predecessor clause, whose
//! head lands on `pred(pred(x))`. The one-step argument covers each link only because
//! `L(pred(x)) = L(pred(y))` makes the next link's premise agree too. That is a reason to keep
//! the pairwise condition, not evidence that label-only blocking breaks — and the deliberate
//! hunt below was re-run after absorption landed, over corpora that now generate exactly these
//! clauses.
//!
//! One empirical honesty about condition (4)'s predecessor-label half: no knowledge base is
//! KNOWN that separates it from label-only blocking in this rule set. That is not a hunt
//! somebody once conducted and wrote down — it is a claim this crate re-checks on every test
//! run, because the mutation it names EXISTS. `Kb::label_only_blocking` (a `cfg(test)` field
//! on [`Kb`], mirroring the `internalize_only` switch the encoding differential uses) makes
//! [`Hyper::same_signature`] compare labels alone, dropping the predecessor-label and
//! incoming-edge halves, and three tests in `crate::owl_dl::oracle` read it:
//!
//! * `blocking_differential` decides EVERY generated knowledge base twice, once under each
//!   condition, and fails the run on a verdict difference. Measured population: 9,799 of the
//!   suite's 9,800 cases — the one exclusion is the single `wide` knowledge base that exhausts
//!   the narrowed round cap whatever it is given — and every verdict agrees. Each property
//!   floors that share at 95%, so the claim cannot quietly come to rest on a handful of cases;
//! * `label_only_blocking_decides_the_inverse_universal_chains_identically` applies the same
//!   mutation to the hand-targeted family of inverse-role/∀⁻ chains that was written as a
//!   deliberate hunt for a separating knowledge base — the corner the generators reach thinly;
//! * `label_only_blocking_builds_a_smaller_graph_than_the_pairwise_condition` pins the OTHER
//!   direction: a knowledge base whose completion graph is strictly smaller under label-only
//!   blocking. Without it, a switch nobody read would produce the same agreement, and the two
//!   tests above would be the calculus agreeing with itself.
//!
//! The sweep covers the corpora as they are TODAY, which is what re-running it after
//! absorption began authoring the head-on-predecessor clauses above bought: those corpora now
//! generate exactly the shape the condition was suspected to be needed for. What moves under
//! the mutation is cost — a smaller graph, and which cases reach the narrowed step cap — never
//! an answer. The structural reason narrows the classic separation:
//! blocking here withholds ONLY `≥`-rule applications, while every clause body — including
//! the `∀r⁻` back-propagation whose obligations the pairwise condition guards in the
//! published calculus — keeps matching blocked nodes, and blocking is recomputed every
//! round as labels grow. The condition is kept because it is the published calculus's and
//! costs one comparison; what must not be claimed is that the test corpus DEMONSTRATES its
//! necessity, and the tests named above are what that claim was replaced with.
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
//! [`consistent`] answers `bool` and turns an exhausted budget into [`EntailError::Build`] — the
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
//! order; the `⊔`-rule takes the FIRST open disjunction that same scan meets; and it branches
//! in the alternatives' authored order, which [`crate::owl_dl::clause`] fixed once from the
//! concept table and the absorbed clauses. The WORK figure is counted off the same search —
//! edges scanned, body atoms joined, subsets enumerated, nodes cloned — so it moves only when
//! the search does.
//! Nothing is read out of a hash map and nothing consults a clock, so a [`Decision`] — verdict,
//! round count, work figure, exhausted flag and the three shape counters
//! ([`Decision::peak_nodes`](crate::owl_dl::graph::Decision), `disjunctions`, `peak_depth`)
//! alike — is byte-identical run to run and on wasm32, and that
//! is asserted rather than merely stated: `a_decision_is_byte_identical_run_to_run` below
//! decides one knowledge base twice and compares the whole struct.

use purrdf_datalog::clause::HeadForm;

use crate::EntailError;
use crate::owl_dl::Kb;
use crate::owl_dl::clause::{BodyAtom, ClauseSet, DlClause, HeadAtom, derive};
use crate::owl_dl::concept::Role;
use crate::owl_dl::graph::{Assumptions, Budget, Decision, Exhausted, Graph, State, find};

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
    ///
    /// The run's second cap is not here: it lives on [`Graph`]'s work meter, because the
    /// work it bounds is done inside the graph operations and this driver reads it through
    /// [`Hyper::check_work`].
    cap: u64,
    /// Whether the caller's stop signal — not the cap — ended the search.
    ///
    /// Recorded on the driver rather than carried in [`Exhausted`] so that the private
    /// `Result<_, Exhausted>` plumbing every rule and branch is written against stays
    /// exactly what it was: the two stops travel out of the search identically and are
    /// separated once, where the [`Decision`] is assembled.
    stopped: bool,
    /// The largest node vector any state reached — see [`Decision::peak_nodes`].
    peak_nodes: u64,
    /// How many times the `⊔`-rule branched — see [`Decision::disjunctions`].
    disjunctions: u64,
    /// The deepest the branch stack got — see [`Decision::peak_depth`].
    peak_depth: u64,
}

/// Decide whether the knowledge base plus `assumptions` has a consistent completion,
/// spending at most `budget`'s derivation rounds and work units.
pub(crate) fn decide(kb: &Kb, assumptions: &Assumptions<'_>, budget: Budget) -> Decision {
    let mut h = Hyper::new(kb, budget);
    let st = h.g.init_state(assumptions);
    match h.solve(st) {
        Ok(consistent) => Decision {
            consistent,
            steps: h.steps,
            work: h.g.work().spent(),
            exhausted: false,
            stopped: false,
            peak_nodes: h.peak_nodes,
            disjunctions: h.disjunctions,
            peak_depth: h.peak_depth,
        },
        // One private refusal, two public facts: `stopped` is what the driver recorded when
        // it turned the poll into an `Exhausted`, and `exhausted` is therefore reserved for
        // the cap it is named after.
        //
        // The three shape counters are reported for a truncated search too, and they are the
        // measurements a reader of a `budget-exhausted` certificate most needs: they say
        // whether the rounds went into a graph, into a branch factor or into a depth.
        Err(Exhausted) => Decision {
            consistent: false,
            steps: h.steps,
            work: h.g.work().spent(),
            exhausted: !h.stopped,
            stopped: h.stopped,
            peak_nodes: h.peak_nodes,
            disjunctions: h.disjunctions,
            peak_depth: h.peak_depth,
        },
    }
}

/// Decide whether the knowledge base plus `assumptions` has a consistent completion.
///
/// # Errors
///
/// [`EntailError::Build`] if either cap is exceeded (a termination-bug backstop for the
/// round cap, and the honest ceiling on per-round work for the other).
pub(crate) fn consistent(kb: &Kb, assumptions: &Assumptions<'_>) -> Result<bool, EntailError> {
    let decision = decide(kb, assumptions, Budget::for_kb(kb));
    if decision.stopped {
        return Err(EntailError::Stopped);
    }
    if decision.exhausted {
        return Err(EntailError::Build(
            "OWL-Direct hypertableau exceeded its search budget (possible non-termination, or an \
             ontology whose per-round work is beyond the work cap)"
                .to_owned(),
        ));
    }
    Ok(decision.consistent)
}

impl<'a> Hyper<'a> {
    /// Build a driver over `kb` bounded by `budget`, deriving its clause set.
    fn new(kb: &'a Kb, budget: Budget) -> Self {
        let g = Graph::new(kb, budget.work);
        Self {
            clauses: derive(g.kb()),
            g,
            steps: 0,
            cap: budget.steps,
            stopped: false,
            peak_nodes: 0,
            disjunctions: 0,
            peak_depth: 0,
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
                    // Every alternative of every level clashed — but only a search that still
                    // had budget when it said so is reporting a refutation rather than a
                    // truncation, because an out-of-budget enumeration stops short and a
                    // branch can close for want of a match it never looked for.
                    self.check_work()?;
                    return Ok(false);
                };
                match level.alternatives.next() {
                    Some(disjunct) => {
                        // A sibling starts from a COPY of the level's state, and copying a
                        // completion graph costs its size. That is work the round cap cannot
                        // see at all — a clone happens between rounds — and on a knowledge
                        // base whose disjunctions interleave it is where a large share of an
                        // unbounded search goes.
                        self.g
                            .work()
                            .charge((level.state.nodes.len() + level.state.edges.len()) as u64 + 1);
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
                Some(alternatives) => {
                    // One `⊔`-rule application, and one more level of search tree. Counted
                    // here rather than where an alternative is taken, because the rule is
                    // applied once and then its alternatives are walked: counting the walk
                    // would report the tree's EDGES under a name that says rule.
                    self.disjunctions = self.disjunctions.saturating_add(1);
                    stack.push(Branches {
                        state: st,
                        alternatives: alternatives.into_iter(),
                    });
                    self.peak_depth = self.peak_depth.max(stack.len() as u64);
                }
                // No disjunction left to branch on: a clash-free completion, which is the
                // answer for the whole search rather than for this level alone — provided the
                // scan that found no open disjunction ran to the end, which an out-of-budget
                // one does not.
                None => {
                    self.check_work()?;
                    return Ok(true);
                }
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
            // Twice per round, and both are needed. The first measures the graph this round
            // INHERITED, which is the only observation a round that clashes before deriving
            // anything ever makes; the second measures what the round MINTED, and is taken
            // before the clash test so that a branch which closed only after growing large is
            // measured rather than discarded — that branch is exactly the one a reader of this
            // counter is looking for.
            self.observe(st);
            if self.concrete_domain_clashes(st) {
                st.clash = true;
                return Ok(false);
            }
            let changed = self.round(st);
            self.observe(st);
            Self::check_clique(st)?;
            // A round whose enumerations stopped for want of budget derived less than the
            // rule set says it should, so its `changed = false` is not a fixpoint and its
            // clash is not a refutation. Checked before both readings.
            self.check_work()?;
            if st.clash {
                return Ok(false);
            }
            if !changed {
                return Ok(true);
            }
        }
    }

    /// Convert a mid-rule clique-budget exhaustion into the search's own exhaustion.
    fn check_clique(st: &State) -> Result<(), Exhausted> {
        if st.clique_exhausted.get() {
            return Err(Exhausted);
        }
        Ok(())
    }

    /// Convert work-budget exhaustion into the search's own exhaustion.
    ///
    /// The meter is consulted rather than decremented here: the charges happen where the work
    /// does — inside [`Graph`]'s scans and this driver's matcher and clones — and this is the
    /// one place they become a decision. Every enumerator polls the same meter and stops, so
    /// the search reaches this within a bounded amount of work of the cap rather than after
    /// whatever the enumeration in flight would have cost.
    fn check_work(&self) -> Result<(), Exhausted> {
        if self.g.work().exhausted() {
            return Err(Exhausted);
        }
        Ok(())
    }

    /// Record how large `st` is against [`Decision::peak_nodes`](crate::owl_dl::graph::Decision).
    fn observe(&mut self, st: &State) {
        self.peak_nodes = self.peak_nodes.max(st.nodes.len() as u64);
    }

    fn tick(&mut self) -> Result<(), Exhausted> {
        // The caller's stop signal, polled once per derivation round — the same boundary the
        // cap is charged at, so a search that can be capped can be stopped.
        if self.g.kb().stopped() {
            self.stopped = true;
            return Err(Exhausted);
        }
        self.check_work()?;
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
        // Labelled so the trigger and clause scans below can bail out of the WHOLE round the
        // moment the meter reports exhausted, rather than finishing the node they were on and
        // every node after it. `saturate` still gates the verdict on `check_work` before it
        // trusts this round's `changed` — see [`Hyper::check_work`] — so stopping here only
        // shortens the latency between the cap being reached and that gate firing; it can never
        // by itself turn a truncated scan into a wrong answer.
        'nodes: for x in 0..st.nodes.len() {
            if self.g.work().exhausted() {
                break 'nodes;
            }
            if find(st, x) != x {
                continue;
            }
            // A general concept inclusion quantifies over `owl:Thing`, so a TBox clause is
            // matched from ABSTRACT nodes only. This is the same restriction the internalized
            // encoding gets for free by not being seeded into a concrete node's label
            // ([`Graph::root`], [`Graph::new_successor`], [`Graph::merge_nodes`]), and it has
            // to be stated here because the absorbed encoding is a CLAUSE rather than a label:
            // a `∀p.A` propagates the named class `A` onto a literal's node, and firing
            // `A ⊑ D` there would derive `D` of a VALUE the axiom never quantified over.
            //
            // The scope is the node variable 0 binds. A clause's HEAD may still land on a
            // concrete node — `rdfs:range` over a data property is exactly `r(x,y) → DR(y)`
            // with `y` a literal — which is the range axiom doing its job rather than a TBox
            // axiom escaping its domain.
            let object_domain = !st.nodes[x].concrete;
            let triggers: Vec<u32> = st.nodes[x].label.iter().copied().collect();
            // One unit per label concept enumerated at this node. A label that grows is what
            // makes a round more expensive without making the search take more rounds.
            self.g.work().charge(triggers.len() as u64 + 1);
            for concept in triggers {
                // A node whose label grew large is what makes the trigger scan itself the
                // remaining cost — the outer node-level check above only runs once per node,
                // and a node with thousands of triggered concepts must not run them all before
                // the meter is consulted again.
                if self.g.work().exhausted() {
                    break 'nodes;
                }
                for &index in self.clauses.triggered_by(concept) {
                    // One unit per clause CONSIDERED, whether or not it is fired: a
                    // disjunctive clause is skipped here without being matched, and a skip
                    // this round repeats is still a scan.
                    self.g.work().charge(1);
                    if self.g.work().exhausted() {
                        break 'nodes;
                    }
                    if !object_domain && self.clauses.is_tbox(index) {
                        continue;
                    }
                    changed |= self.fire(st, index, x, &blocked);
                    if st.clash {
                        return changed;
                    }
                }
            }
            for &index in self.clauses.untriggered() {
                self.g.work().charge(1);
                if self.g.work().exhausted() {
                    break 'nodes;
                }
                if !object_domain && self.clauses.is_tbox(index) {
                    continue;
                }
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

    /// The FIRST open head disjunction the derivation order meets, or `None` when none is open
    /// — the `⊔`-rule's branch point.
    ///
    /// The order is the round's own: nodes ascending, and within a node the label's concepts
    /// ascending with their clauses in derivation order, then the untriggered clauses. So the
    /// disjunction branched on is the one the search had already reached, the rule is a pure
    /// function of the state, and the scan stops at the first match instead of running to the
    /// end.
    ///
    /// # Why not the NARROWEST open disjunction, which is the textbook rule
    ///
    /// Every open disjunction has to be resolved before a completion is clash-free, so which
    /// one is taken first changes no verdict — it changes the shape of the tree the search
    /// walks to reach it. The published argument for taking the narrowest first is that a level
    /// of `k` alternatives multiplies the subtree below it by `k`, so putting the widest levels
    /// deepest lets the clashes above prune them. This calculus was MEASURED under that rule,
    /// over the generated corpora of [`crate::owl_dl::oracle`] — 8,900 knowledge bases at the
    /// time, 9,200 now — and
    /// the argument did not pay here:
    ///
    /// * by itself it was close to a wash, saving rounds on the nominal and counting families
    ///   and spending them on the boolean and two-role ones, for a fraction of a percent
    ///   against a run of some twenty thousand rounds;
    /// * it made one knowledge base in the boolean corpus (`complement ⊗ disjunction`) cost
    ///   439 rounds where this rule decides it in 178 — the corpus's most expensive DECIDING
    ///   case under it, and the number the suite's own cap had to be widened to clear;
    /// * and the minimum is only known once the scan has matched every clause of every label
    ///   concept of every node, including the `≤n` clauses whose body enumerates the
    ///   count-element SUBSETS of a node's successors. That work is charged to no derivation
    ///   round, because a branch point is chosen between rounds rather than inside one, so it
    ///   is cost the ROUND cap cannot see — it is charged to the work meter
    ///   ([`work_cap`](crate::owl_dl::graph::work_cap)) instead, which is the budget that can.
    ///
    /// What DOES pay is which alternative of the chosen disjunction is tried first, and that is
    /// a property of the clause set rather than of this scan:
    /// [`Kb::order_disjuncts`](crate::owl_dl::Kb::order_disjuncts) authors the alternatives that
    /// mint no witnesses ahead of the ones that do, and the corpus-wide win the two levers were
    /// first measured together for is entirely that one's. The measurements above are kept
    /// here, rather than deleted with the rule they retired, so the next reader who reaches for
    /// narrowest-first finds out what it was worth without re-running the corpus.
    fn find_branch(&self, st: &State) -> Option<Vec<Vec<Ground>>> {
        for x in 0..st.nodes.len() {
            if find(st, x) != x {
                continue;
            }
            let triggers: Vec<u32> = st.nodes[x].label.iter().copied().collect();
            // The branch-point scan is charged exactly as the round's is, and it is charged
            // for the reason [`Hyper::find_branch`]'s own measurements give: a branch point is
            // chosen BETWEEN rounds, so every clause this scan matches — including the `≤n`
            // clauses whose bodies enumerate successor subsets — used to be work no budget
            // could see. This is the counter that sees it.
            self.g.work().charge(triggers.len() as u64 + 1);
            for concept in triggers {
                for &index in self.clauses.triggered_by(concept) {
                    self.g.work().charge(1);
                    if let Some(branch) = self.branch_of(st, index, x) {
                        return Some(branch);
                    }
                }
            }
            for &index in self.clauses.untriggered() {
                self.g.work().charge(1);
                if let Some(branch) = self.branch_of(st, index, x) {
                    return Some(branch);
                }
            }
            if self.g.work().exhausted() {
                // Out of budget mid-scan: stop rather than walk the rest of the graph for an
                // answer the driver is about to discard. The `None` is not read as "no open
                // disjunction" — `solve` checks the meter before it believes one.
                return None;
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
            // Grounding a `≤n` head expands one schematic atom into one alternative per PAIR
            // of counted successors, so the alternatives a single match produces are
            // quadratic in the count. Charged by what came out.
            self.g.work().charge(disjuncts.len() as u64);
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
        // One unit for the attempt itself, so a clause that matches nothing at a node still
        // costs what it took to find that out. A round tries every clause of every label
        // concept of every node, so this charge alone is what makes the ROUND's own size
        // visible to the work budget.
        g.work().charge(1);
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
        // One unit per JOIN STEP — one body atom, at one partial binding. This is the
        // matcher's own cost, and the quantity that grows when a knowledge base makes many
        // clauses match at one node: the join tree's size, not the number of rounds it is
        // walked in.
        g.work().charge(1);
        // Out of budget: stop the walk rather than finish an enumeration whose result is
        // about to be discarded. `true` is the stop signal the callers already understand,
        // and the driver reports the exhaustion before any verdict is read off this state.
        if g.work().exhausted() {
            return true;
        }
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
        // One unit per selection step. A `≤n` clause enumerates the `count`-element SUBSETS
        // of a node's counted successors, which is `C(k, count)` selections from `k`
        // successors — the most violently super-linear enumeration in the calculus, done
        // inside a single round, and the reason a rounds-denominated cap could watch a search
        // grind at three percent of its budget.
        g.work().charge(1);
        if g.work().exhausted() {
            return true;
        }
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
            // One unit per candidate blocker considered. Anywhere blocking compares a node
            // against every earlier unblocked node, so this scan is quadratic in the graph
            // and runs once per ROUND — work the round count reports as one.
            self.g.work().charge(candidates.len() as u64 + 1);
            let directly = candidates
                .iter()
                .any(|&y| self.same_signature(st, x, y, parent));
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
    ///
    /// # The mutation this method carries
    ///
    /// Under [`Kb::labels_alone_block`](crate::owl_dl::Kb) — a `cfg(test)` switch nothing
    /// outside the differential corpus sets — the two PAIRWISE halves are dropped and the
    /// comparison is the labels alone. That is the weaker blocking condition the module docs
    /// discuss: it blocks strictly more nodes, so it withholds strictly more `≥`-rule
    /// applications, and if the predecessor-label half were load-bearing for this rule set a
    /// knowledge base would come out consistent under it that the shipped condition refutes.
    /// The claim that none does is checked over every generated corpus by the blocking
    /// differential in [`crate::owl_dl::oracle`] rather than asserted in prose.
    fn same_signature(&self, st: &State, x: usize, y: usize, parent: usize) -> bool {
        if st.nodes[x].label != st.nodes[y].label {
            return false;
        }
        if self.g.kb().labels_alone_block() {
            return true;
        }
        let Some(other_parent) = st.nodes[y].parent.map(|p| find(st, p)) else {
            return false;
        };
        st.nodes[x].incoming == st.nodes[y].incoming
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

#[cfg(test)]
mod tests {
    use super::{Hyper, decide};
    use crate::owl_dl::Kb;
    use crate::owl_dl::clause::{HeadAtom, derive};
    use crate::owl_dl::concept::{Concept, Role};
    use crate::owl_dl::graph::{Assumptions, Budget};

    /// A class term id.
    const A: u32 = 10;
    /// A second class term id.
    const B: u32 = 11;
    /// A third class term id.
    const C: u32 = 12;
    /// A fourth class term id.
    const D: u32 = 13;
    /// A role term id.
    const R: u32 = 20;
    /// An individual term id.
    const IND: u32 = 30;

    /// A knowledge base that branches, mints and counts: a three-way disjunction, a two-way
    /// one, an existential the first reaches, and a cardinality bound over the witnesses.
    fn branching_kb() -> Kb {
        let mut kb = Kb::empty();
        kb.push_gci(
            Concept::Named(A),
            Concept::Some(Role::Named(R), Box::new(Concept::Named(C))),
        );
        kb.push_gci(
            Concept::Named(B),
            Concept::Max(1, Role::Named(R), Box::new(Concept::Named(C))),
        );
        let wide = kb.table.intern(Concept::Or(vec![
            Concept::Named(A),
            Concept::Named(B),
            Concept::Named(C),
        ]));
        let narrow = kb
            .table
            .intern(Concept::Or(vec![Concept::Named(C), Concept::Named(D)]));
        kb.abox_types.push((IND, wide));
        kb.abox_types.push((IND, narrow));
        kb.individuals.insert(IND);
        kb.finalize();
        kb
    }

    /// A [`Decision`](crate::owl_dl::graph::Decision) is a pure function of the knowledge base:
    /// the same one decided twice gives back the same WHOLE struct — verdict, round count,
    /// WORK figure and both stop flags.
    ///
    /// The determinism doctrine is stated in the module docs; this is what makes it an
    /// observation. A search that read a `HashMap`, a clock or a float would still answer the
    /// same verdict most runs, and it is the two cost figures that would move first — the work
    /// figure soonest of all, because it counts every scan rather than every round.
    #[test]
    fn a_decision_is_byte_identical_run_to_run() {
        let kb = branching_kb();
        let cap = Budget::for_kb(&kb);
        let first = decide(&kb, &Assumptions::of_kb(), cap);
        let again = decide(&kb, &Assumptions::of_kb(), cap);
        assert_eq!(first, again, "two runs, one decision");
        assert!(!first.exhausted && !first.stopped, "{first:?}");
    }

    /// The `⊔`-rule takes the FIRST open disjunction the derivation order meets, and NOT the
    /// narrowest — the rule the measurements at [`Hyper::find_branch`] retired.
    ///
    /// The three-way disjunction is interned first, so it has the smaller concept id, is what
    /// the individual's label enumerates first, and is therefore the branch point — although a
    /// two-way disjunction is open on the same node. Asserting the WIDE one is what makes this
    /// a test of the selection rule rather than of the fixture: a scan that ranked its
    /// candidates by width would hand back the other one.
    #[test]
    fn the_branch_point_is_the_first_open_disjunction_in_derivation_order() {
        let kb = branching_kb();
        let mut h = Hyper::new(&kb, Budget::for_kb(&kb));
        let mut st = h.g.init_state(&Assumptions::of_kb());
        assert!(
            h.saturate(&mut st).expect("a fixture this small saturates"),
            "the state must be clash-free before a branch point is chosen"
        );
        let branch = h.find_branch(&st).expect("two disjunctions are open");
        assert_eq!(
            branch.len(),
            3,
            "the three-way disjunction is met first, so it is the branch point: {branch:?}"
        );
    }

    /// A disjunction's alternatives are AUTHORED non-generating first, whatever order the
    /// interner's canonical form put its members in.
    ///
    /// `A ⊑ ∃r.C` makes `A` generating through the absorbed table — the closure, since `A`
    /// itself is an atomic leaf — while `B` forces nothing. The canonical form sorts `A` before
    /// `B` (concept ids ascend), so the emitted order is the reverse of it, and that is the
    /// whole observable difference between the identity order and the search order.
    #[test]
    fn a_disjunct_that_forces_witnesses_is_authored_last() {
        let mut kb = Kb::empty();
        kb.push_gci(
            Concept::Named(A),
            Concept::Some(Role::Named(R), Box::new(Concept::Named(C))),
        );
        let disjunction = kb
            .table
            .intern(Concept::Or(vec![Concept::Named(A), Concept::Named(B)]));
        kb.abox_types.push((IND, disjunction));
        kb.individuals.insert(IND);
        kb.finalize();
        let generating = kb.table.intern(Concept::Named(A));
        let inert = kb.table.intern(Concept::Named(B));
        assert!(
            kb.generates(generating),
            "A ⊑ ∃r.C makes A generating through the absorbed clause it triggers"
        );
        assert!(!kb.generates(inert), "B forces nothing");
        assert!(
            generating < inert,
            "the canonical order puts A first, so the cost order is observable"
        );

        let clauses = derive(&kb);
        let clause = clauses
            .triggered_by(disjunction)
            .iter()
            .map(|&index| clauses.clause(index))
            .find(|clause| clause.head.len() == 2)
            .expect("the disjunction derives its ⊔-clause");
        let authored: Vec<u32> = clause
            .head
            .iter()
            .map(|disjunct| match disjunct.as_slice() {
                [HeadAtom::Concept { concept, .. }] => *concept,
                other => panic!("a ⊔-clause disjunct is one concept atom: {other:?}"),
            })
            .collect();
        assert_eq!(
            authored,
            vec![inert, generating],
            "the alternative that mints witnesses is tried last"
        );
    }

    /// The generating closure is TRANSITIVE over the absorbed table: `A ⊑ B` and `B ⊑ ∃r.C`
    /// make `A` generating, though nothing about `A`'s own decomposition says so.
    ///
    /// This is the row of the cost table that cannot be read off a concept: a named class is an
    /// atomic leaf, and what it forces is a fact about the clauses it triggers.
    #[test]
    fn the_generating_closure_follows_a_chain_of_absorbed_clauses() {
        let mut kb = Kb::empty();
        kb.push_gci(Concept::Named(A), Concept::Named(B));
        kb.push_gci(
            Concept::Named(B),
            Concept::Some(Role::Named(R), Box::new(Concept::Named(C))),
        );
        kb.finalize();
        let a = kb.table.intern(Concept::Named(A));
        let b = kb.table.intern(Concept::Named(B));
        let c = kb.table.intern(Concept::Named(C));
        assert!(kb.generates(b), "B ⊑ ∃r.C is one link");
        assert!(kb.generates(a), "A ⊑ B ⊑ ∃r.C is two");
        assert!(!kb.generates(c), "the filler forces nothing of its own");
    }

    /// A disjunction is generating only when EVERY alternative is: one alternative that mints
    /// nothing is a way to satisfy it that mints nothing.
    #[test]
    fn a_disjunction_is_generating_only_when_every_alternative_is() {
        let mut kb = Kb::empty();
        kb.push_gci(
            Concept::Named(A),
            Concept::Some(Role::Named(R), Box::new(Concept::Named(C))),
        );
        kb.push_gci(
            Concept::Named(B),
            Concept::Some(Role::Named(R), Box::new(Concept::Named(D))),
        );
        let mixed = kb
            .table
            .intern(Concept::Or(vec![Concept::Named(A), Concept::Named(C)]));
        let both = kb
            .table
            .intern(Concept::Or(vec![Concept::Named(A), Concept::Named(B)]));
        let conjunction = kb
            .table
            .intern(Concept::And(vec![Concept::Named(A), Concept::Named(C)]));
        kb.finalize();
        assert!(!kb.generates(mixed), "C is a way out of the disjunction");
        assert!(kb.generates(both), "both alternatives mint");
        assert!(
            kb.generates(conjunction),
            "a conjunction holds every conjunct, so ONE generating conjunct generates"
        );
    }

    /// FB-1 (bulk enumerations honour an exhausted work meter immediately): [`Hyper::round`]'s
    /// node loop must stop visiting nodes the moment the shared work meter is exhausted,
    /// rather than finishing every remaining node's trigger and clause scan first.
    ///
    /// Five thousand individuals share one trigger (`A ⊑ B`, a faithful guard that fires at
    /// every `A`-typed node), so a completed round would derive `B` at every one of them. Under
    /// a work cap far smaller than that scan, this pins that only a SMALL PREFIX of the five
    /// thousand nodes gets visited before the meter reports itself exhausted and the loop
    /// breaks — not that the derivation comes out wrong: `Hyper::saturate` gates the verdict on
    /// `check_work` before it ever trusts a round's `changed` flag (see [`Hyper::check_work`]),
    /// so a truncated round here can only ever surface as `Exhausted`, never as an answer.
    #[test]
    fn round_stops_visiting_nodes_once_the_work_meter_is_exhausted() {
        const MANY: u32 = 5_000;
        let mut kb = Kb::empty();
        kb.push_gci(Concept::Named(A), Concept::Named(B));
        for i in 0..MANY {
            let individual = 1_000 + i;
            kb.abox_types.push((individual, A));
            kb.individuals.insert(individual);
        }
        kb.finalize();
        let b = kb.table.intern(Concept::Named(B));

        let budget = Budget {
            steps: Budget::for_kb(&kb).steps,
            work: 40,
        };
        let h = Hyper::new(&kb, budget);
        let mut st = h.g.init_state(&Assumptions::of_kb());

        h.round(&mut st);

        assert!(
            h.g.work().exhausted(),
            "a cap of 40 against five thousand nodes' worth of trigger scanning must exhaust"
        );
        let derived = st.nodes.iter().filter(|n| n.label.contains(&b)).count();
        assert!(
            derived < MANY as usize,
            "a narrow work cap must stop the node scan before every one of the {MANY} nodes \
             is visited: {derived} nodes already carry B"
        );
    }
}
