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

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

use crate::owl_dl::Kb;
use crate::owl_dl::clause::BodyAtom;
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
    /// Units charged so far, clamped at [`Self::cap`].
    spent: std::cell::Cell<u64>,
    /// The ceiling this meter stops at.
    cap: u64,
}

impl Work {
    /// A meter that stops at `cap`.
    const fn new(cap: u64) -> Self {
        Self {
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

    /// Whether the budget is gone.
    ///
    /// Every enumerator that could run long consults this and stops, so the latency between
    /// the cap being reached and the search reporting it is bounded by one charge rather than
    /// by whatever the enumeration would have cost.
    pub(crate) fn exhausted(&self) -> bool {
        self.spent.get() >= self.cap
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
///   untyped-restrictions ontology's 17-triple `owl:equivalentClass` shape spends 2,724
///   units; its `rdfs:subClassOf` control — the same seventeen triples with BOTH
///   restrictions moved off the equivalence — 206.
/// * **the differential corpora** of [`crate::owl_dl::oracle`] — 9,800 generated,
///   deliberately adversarial knowledge bases (pinned by
///   `the_enumerated_search_spaces_are_pinned`). Their most expensive DECIDING case spends
///   6.1 million units, over a THREE-axiom knowledge base whose completion graph reaches 101
///   nodes. That case is what fixes the constant term: work is a function of the SEARCH
///   rather than of the input's size, so a size-derived cap has to carry a floor generous
///   enough for a small ontology whose search is not, and 64 million is that measurement
///   times ten.
/// * **the two block families** of this crate's consistency bench (`benches/consistency.rs`),
///   at 1/2/4/8/16 blocks. The INDEPENDENT family (one individual per block) spends 2,724 /
///   17,750 / 177,461 / 2,398,087 / 40,349,307 units and decides at every size, the largest
///   with fifteen times its budget left. The STACKED family — the same blocks co-typed on
///   ONE individual, which is the shape this cap exists for — spends 2,724 / 185,099 /
///   75,826,178 at 1/2/4 blocks and decides them (the two-block knowledge base is the same
///   one the step ledger pins as `co-typed-equivalence-blocks`, at the same 185,099), and
///   from five blocks on it reaches the cap: `unknown` under `completeness
///   budget-exhausted`, with `work` equal to `work-budget` in the certificate — the same
///   signature `crates/validate/tests/dl_work_budget.rs` pins at ten co-typed copies. Run
///   UNCAPPED the same family spends 5,194,168 units at three blocks, 75,826,178 at four,
///   687,884,004 at five and 4.4 BILLION at six — roughly a factor of nine per added block —
///   so ten blocks is some 10¹³ units of grinding, which is what this class did before the
///   cap existed while its round count sat at a few percent of the round budget.
///
/// The base is [`cap_base`] — the same size the round cap is derived from — and the formula is
/// `64,000,000 + base³ × 256`. CUBIC rather than the round cap's quadratic, because the two
/// bound quantities one degree apart: a round's own cost is about quadratic in the size (nodes
/// times clauses) and the number of rounds is about linear in it, so a work bound that grew
/// only as fast as the round cap would tighten as ontologies grow.
///
/// What the formula does NOT promise is that the stacked family becomes decidable by making
/// the number bigger. Its work grows by roughly a factor of ten per added block against a
/// cubic budget, so every cap has an `n` it stops at; the honest curve is stated above, and
/// what a cap buys is that the ontology past that `n` ANSWERS — `unknown`, with
/// `completeness budget-exhausted` and `work` equal to `work-budget` — in a few seconds
/// instead of grinding.
///
/// It is a pure function of the knowledge base — same input, same cap — and it is a COUNT
/// rather than a clock reading, which is what keeps a [`Decision`] reproducible run to run
/// and on `wasm32`.
pub(crate) fn work_cap(kb: &Kb) -> u64 {
    let base = cap_base(kb);
    64_000_000
        + base
            .saturating_mul(base)
            .saturating_mul(base)
            .saturating_mul(256)
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
pub(crate) fn max_clique(
    items: &[usize],
    compat: &dyn Fn(usize, usize) -> bool,
    meter: &Work,
) -> Option<Vec<usize>> {
    let mut best: Vec<usize> = Vec::new();
    let mut current: Vec<usize> = Vec::new();
    let mut work: u64 = 0;
    let finished = rec_clique(items, compat, 0, &mut current, &mut best, &mut work, meter);
    if finished { Some(best) } else { None }
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

/// Backtracking helper for [`max_clique`]; `false` means a work budget ran out — either this
/// search's own local ceiling, or the run's shared [`Work`] meter.
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
fn rec_clique(
    items: &[usize],
    compat: &dyn Fn(usize, usize) -> bool,
    start: usize,
    current: &mut Vec<usize>,
    best: &mut Vec<usize>,
    work: &mut u64,
    meter: &Work,
) -> bool {
    *work += 1;
    meter.charge(1);
    if *work > MAX_CLIQUE_WORK || meter.exhausted() {
        return false;
    }
    if current.len() > best.len() {
        *best = current.clone();
    }
    // Bound: even taking every remaining item, this recursive case cannot beat `best`.
    if current.len() + (items.len() - start) <= best.len() {
        return true;
    }
    for i in start..items.len() {
        // One unit per CANDIDATE CONSIDERED, not only per recursive call: a level whose
        // candidates mostly fail `compat` never recurses, so without this charge the loop
        // below could walk the whole remaining slice — items.len() - start candidates, which
        // a mixed `≠`-graph can make large — with neither budget seeing it.
        *work += 1;
        meter.charge(1);
        if *work > MAX_CLIQUE_WORK || meter.exhausted() {
            return false;
        }
        let cand = items[i];
        if current.iter().all(|&m| compat(m, cand)) {
            current.push(cand);
            if !rec_clique(items, compat, i + 1, current, best, work, meter) {
                return false;
            }
            current.pop();
        }
        // Re-check the bound as the window shrinks.
        if current.len() + (items.len() - i - 1) <= best.len() {
            return true;
        }
    }
    true
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
    achiever_cache: RefCell<BTreeMap<Role, BTreeSet<(u32, bool)>>>,
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
            kb,
            work: Work::new(work_cap),
            meta,
            unconditional,
            achiever_cache: RefCell::new(BTreeMap::new()),
            range_by_role,
        }
    }

    /// The knowledge base every rule reads.
    pub(crate) const fn kb(&self) -> &'a Kb {
        self.kb
    }

    /// The run's work meter — what both drivers charge their own scans and clones to, and
    /// read the run's work figure off.
    pub(crate) const fn work(&self) -> &Work {
        &self.work
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
        // The keeper is now known to inhabit the DATA domain, so the TBox's unconditional
        // assertions never constrained it — whichever encoding they arrived in. Withdrawing
        // them can only remove a clash, never add one, which is the direction an
        // identification is allowed to move the answer in.
        if st.nodes[keep].concrete {
            for concept in &self.unconditional {
                st.nodes[keep].label.remove(concept);
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
        // A neighbourhood read is the single most-called scan in either calculus — every
        // clause body atom over a role, every counting rule and every satisfaction test goes
        // through it — so it is where an unbounded search spends most of what a round cap
        // cannot see. Charged whole: the achiever closure below, then one unit per edge each
        // step examines.
        if self.work.exhausted() {
            return Vec::new();
        }
        let ach = self.achievers(role);
        let x = find(st, x);
        let mut out: Vec<usize> = Vec::new();
        let mut seen: BTreeSet<usize> = BTreeSet::new();
        self.step(st, x, &ach, &mut seen, &mut out);
        for &(prop, dir) in &ach {
            if !self.kb.transitive.contains(&prop) {
                continue;
            }
            if self.work.exhausted() {
                return out;
            }
            let single: BTreeSet<(u32, bool)> = std::iter::once((prop, dir)).collect();
            // Breadth-first over this one transitive role, seeded from `x`'s own step.
            let mut frontier: Vec<usize> = Vec::new();
            self.step(st, x, &single, &mut BTreeSet::new(), &mut frontier);
            let mut visited: BTreeSet<usize> = frontier.iter().copied().collect();
            while let Some(y) = frontier.pop() {
                // The transitive closure is the one loop here whose length is a function of
                // the graph rather than of the role hierarchy, so it is polled as well as
                // charged: a run whose budget went while this was running stops here.
                if self.work.exhausted() {
                    return out;
                }
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
        // One unit per edge examined, charged before the scan rather than inside it: the loop
        // below visits every edge unconditionally, so the cost is known in advance and one
        // charge is cheaper than one per iteration.
        self.work.charge(st.edges.len() as u64);
        // The charge above is what a NARROW cap needs to see, and seeing it is only useful if
        // the scan then honours it: a graph whose edge count alone exhausts the meter must not
        // still walk every edge before this method returns, or the latency between the cap
        // being reached and the search reporting it would be the size of the edge vector
        // rather than one charge, exactly the gap this bulk charge exists to close.
        if self.work.exhausted() {
            return;
        }
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
    ///
    /// Memoized in [`Self::achiever_cache`]: the closure is a pure function of the KB's role
    /// axioms, which do not change while this `Graph` runs a search, so it is built at most
    /// once per role however many times [`Self::neighbors`] asks for it. A HIT is charged one
    /// unit — a lookup and a clone of a small set — and only a MISS pays for the stack walk
    /// below, so the meter still moves on every call but the search now pays the closure's
    /// real cost once rather than once per call.
    fn achievers(&self, role: Role) -> BTreeSet<(u32, bool)> {
        if let Some(cached) = self.achiever_cache.borrow().get(&role) {
            self.work.charge(1);
            return cached.clone();
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
                return set;
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
        self.achiever_cache.borrow_mut().insert(role, set.clone());
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
        let with_filler: Vec<usize> = self
            .neighbors(st, x, role)
            .into_iter()
            .filter(|&y| self.has_concept(st, y, filler))
            .collect();
        let Some(mut clique) = max_clique(&with_filler, &|a, b| are_distinct(st, a, b), &self.work)
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
        let with_filler: Vec<usize> = self
            .neighbors(st, x, role)
            .into_iter()
            .filter(|&y| self.has_concept(st, y, filler))
            .collect();
        match max_clique(&with_filler, &|a, b| are_distinct(st, a, b), &self.work) {
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

    /// A bare tree node: no label, no incoming edge, not a root — the minimal shape these
    /// tests need to populate a [`State`] by hand rather than through a knowledge base.
    fn bare_node(root: bool) -> Node {
        Node {
            label: BTreeSet::new(),
            parent: None,
            incoming: None,
            root,
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
        State {
            nodes: vec![bare_node(true), bare_node(true)],
            edges: std::iter::repeat_n((0usize, 1usize, prop), n).collect(),
            root_of: BTreeMap::new(),
            clash: false,
            clique_exhausted: std::cell::Cell::new(false),
        }
    }

    // --- FB-1: `max_clique`/`rec_clique` poll the shared meter DURING the search -----------

    /// [`rec_clique`]'s CANDIDATE loop — not only its recursive calls — must stop the moment
    /// the shared meter is exhausted, because a candidate set most of which is pairwise
    /// INCOMPATIBLE never recurses past depth one: the whole cost is one call's `for` loop
    /// over the remaining candidates, calling `compat` once each. Before this fix that loop
    /// carried no charge and no poll at all, so neither the shared meter nor `rec_clique`'s
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
    /// [`rec_clique`] makes is a pure function of `items` and `compat`, never of a clock or a
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

    /// [`Graph::step`] charges the WHOLE edge scan's cost up front, in one bulk charge, so a
    /// NARROW cap sees the true cost of the scan it is about to refuse before that scan runs
    /// even one comparison. Without the check right after that charge, the loop below it
    /// would walk every one of a graph's edges regardless — which is exactly the gap this
    /// test pins shut: a cap far smaller than the edge count must come back with NO neighbours
    /// rather than the true one, because it never got to look.
    #[test]
    fn neighbors_stops_the_edge_scan_when_the_bulk_charge_exhausts_the_meter() {
        const PROP: u32 = 7;
        let kb = Kb::empty();
        let st = two_node_state_with_edges(2_000_000, PROP);

        let narrow = Graph::new(&kb, 5);
        let truncated = narrow.neighbors(&st, 0, Role::Named(PROP));
        assert!(
            truncated.is_empty(),
            "a cap of 5 against two million edges must see none of them: {truncated:?}"
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
            full,
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
        assert!(ample.achiever_cache.borrow().contains_key(&role));
    }
}
