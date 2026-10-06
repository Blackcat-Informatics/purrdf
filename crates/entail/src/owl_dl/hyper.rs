// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

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

use std::cell::RefCell;

use purrdf_datalog::clause::HeadForm;

use crate::EntailError;
use crate::owl_dl::Kb;
use crate::owl_dl::clause::{BodyAtom, ClauseSet, DlClause, HeadAtom, TransitivePatterns, derive};
use crate::owl_dl::concept::Role;
use crate::owl_dl::graph::{
    Assumptions, Budget, Decision, Exhausted, GeneratedRoot, Graph, State, find, for_each_step,
};
use crate::owl_dl::proof::{
    BranchOutcome, BranchStep, ClashStep, MergeCause, MergeLicence, MergeStep, NodeRef, Recorder,
    RecorderMark, frame_refs, node_ref, observe_alternative, observe_body, observe_completion,
};

/// One head atom with its variables replaced by the nodes a match bound them to.
///
/// The `⊔`-rule needs to hold a branch's alternatives across a state clone, so a derived head
/// is grounded once and then applied — rather than re-matching the clause in each branch,
/// which would re-derive the same instance from a state the branch has already changed.
///
/// # Why the node carrier is a parameter
///
/// The SEARCH grounds a head against completion-graph node INDICES (`Ground<usize>`), which is
/// the only thing it can assert against. A [`DlProof`](crate::owl_dl::proof::DlProof)'s checker
/// grounds the same head against the merge-invariant
/// [`NodeRef`](crate::owl_dl::proof::NodeRef) identities a proof term carries
/// (`Ground<NodeRef>`), because it holds no graph. Both go through [`ground_head`], so
/// "the checker regenerates the alternatives" is the SAME function on a different carrier
/// rather than a second implementation that could drift from it — which is precisely why
/// [`Grounding`](crate::owl_dl::proof::TrustBaseEntry::Grounding) is a named trust-base entry
/// and the regeneration is reported `trusted` rather than `attested`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Ground<N> {
    /// Add a concept to a node's label.
    Concept(N, u32),
    /// Give a node an `r`-edge to itself.
    SelfLoop(N, Role),
    /// Ensure `n` pairwise-distinct `role`-neighbours satisfying the filler.
    AtLeast(N, u32, Role, u32),
    /// Identify two nodes.
    Equal(N, N),
    /// Identify a node with an individual's root.
    EqualIndividual(N, u32),
    /// Motik–Shearer–Horrocks Table 5 `NI`-rule: identify the node with the RESERVED ROOT
    /// `u.⟨R,B,i⟩` named by the [`GeneratedRoot`], minting it if it does not yet exist. This is
    /// the annotated equality of the at-most clausification — an equality whose right side is a
    /// generated root rather than a matched successor — and it is what bounds the blockable
    /// predecessors of a nominal into a finite reserved set.
    EqualReserved(N, GeneratedRoot),
}

impl<N> Ground<N> {
    /// The same atom over another node carrier.
    ///
    /// Used once, by the recorder, to read a search-time `Ground<usize>` off the state as the
    /// `Ground<NodeRef>` a proof term carries. It is a structure map and nothing else: no atom
    /// is dropped, added or reordered, so a recorded alternative is the alternative the search
    /// actually branched over.
    pub(crate) fn map<M>(&self, f: &mut impl FnMut(&N) -> M) -> Ground<M> {
        match self {
            Self::Concept(node, concept) => Ground::Concept(f(node), *concept),
            Self::SelfLoop(node, role) => Ground::SelfLoop(f(node), *role),
            Self::AtLeast(node, n, role, filler) => Ground::AtLeast(f(node), *n, *role, *filler),
            Self::Equal(left, right) => {
                let left = f(left);
                Ground::Equal(left, f(right))
            }
            Self::EqualIndividual(node, individual) => {
                Ground::EqualIndividual(f(node), *individual)
            }
            Self::EqualReserved(node, key) => Ground::EqualReserved(f(node), key.clone()),
        }
    }
}

/// The site a `⊔`-rule branch point was found at, captured only by a RECORDING run.
///
/// The alternatives alone do not say what generated them, and a proof term that could not name
/// the clause instance would leave a checker nothing to regenerate them from.
pub(crate) struct BranchSite {
    /// The clause whose disjunctive head produced the alternatives.
    index: usize,
    /// The node variable `0` was bound to.
    x: usize,
    /// The matcher's binding frame.
    frame: Vec<usize>,
    /// How many leading alternatives came from [`ground_head`] — the rest are the `NI`-rule's,
    /// which are appended by [`Hyper::append_ni_alternatives`] and are NOT regenerable from the
    /// clause alone.
    head_alternatives: usize,
}

/// A `⊔`-rule branch point: its alternatives, and (when recording) where they came from.
pub(crate) struct Branching {
    /// The grounded alternatives, in the order the search will try them.
    alternatives: Vec<Vec<Ground<usize>>>,
    /// The clause instance that generated them — `None` for a non-recording run, which needs
    /// nothing but the alternatives themselves.
    site: Option<BranchSite>,
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
    alternatives: std::vec::IntoIter<Vec<Ground<usize>>>,
    /// How many have been dispensed — the ordinal the NEXT one takes in the recorded
    /// alternative list, so an outcome can be filed against the alternative it belongs to.
    dispensed: usize,
    /// The recorded branch step this level is, when the run is recording.
    record: Option<usize>,
}

/// The alternative a recording run is currently exploring, and where its outcome is filed.
struct OpenSlot {
    /// The recorded branch step.
    branch: usize,
    /// Which alternative of it.
    ordinal: usize,
    /// What the recorder held before the alternative was applied, so the closure it reached
    /// can be identified as "whatever was written down since".
    mark: RecorderMark,
}

/// ONE identification, as the search observed it — the three identities and the rule.
///
/// Held together rather than passed as four positional arguments for the reason
/// [`Assumptions`] is one struct: three same-typed [`NodeRef`]s in a signature are three
/// arguments a caller can silently permute, and permuting them here would record a merge that
/// joined the wrong pair while every test still passed.
struct Identified {
    /// Which rule forced the identification.
    cause: MergeCause,
    /// The identity the licensing atom names for the absorbing side.
    left: NodeRef,
    /// The node being identified, as it stood before the merge.
    right: NodeRef,
    /// The identity both denote after it.
    joined: NodeRef,
}

/// WHERE the disjunct [`Hyper::apply`] is asserting came from.
///
/// A merge is licensed by the grounded head atom that asserted it, and there are exactly two
/// places this search asserts one: a non-disjunctive clause instance the derivation round
/// fired, and one alternative of a `⊔`-rule branch point. Naming the site here is what lets an
/// instrumented run write a [`MergeLicence`] a checker can ground for itself.
///
/// It borrows the frame rather than owning it, so a NON-recording run — which is every run
/// every existing caller makes — allocates nothing to construct one.
enum ApplySite<'f> {
    /// The single grounded head disjunct of non-disjunctive clause `index`, matched at `frame`.
    Clause {
        /// The clause index.
        index: usize,
        /// The matcher's binding frame.
        frame: &'f [usize],
    },
    /// Alternative `ordinal` of the recorded branch point at `branch`.
    Branch {
        /// The recorded branch step.
        branch: usize,
        /// Which alternative of it.
        ordinal: usize,
    },
    /// Nothing replayable: a non-recording run, or a branch point past the recording ceiling.
    Unrecorded,
}

/// The hypertableau driver: the graph operations, the clause set, and a budget.
struct Hyper<'a> {
    /// The completion graph operations over the knowledge base.
    g: Graph<'a>,
    /// The DL-clauses derived from it.
    clauses: ClauseSet,
    /// [`ClauseSet::match_radius`]: how many reads a change reaches the matches a round must
    /// redo across.
    radius: usize,
    /// Per clause, its shape when its body makes exactly one neighbourhood read — see
    /// [`SingleRead`] — so a round can match it against what that read gained alone.
    single_reads: Vec<Option<SingleRead>>,
    /// The region search's stamped scratch, reused round after round.
    region: RegionScratch,
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
    /// Where an INSTRUMENTED run writes its clash witnesses and merge provenance.
    ///
    /// `None` for every run every existing caller makes, and that is the whole safety
    /// argument: a non-recording driver allocates nothing here, and each of the three record
    /// sites is one `Option` test that reads no state, charges no work and takes no branch.
    /// A recorded run therefore explores the same search tree in the same order and reports
    /// the byte-identical [`Decision`] — asserted by
    /// `a_recorded_decision_is_identical_to_an_unrecorded_one` below, beside the pre-existing
    /// `a_decision_is_byte_identical_run_to_run`.
    ///
    /// A [`RefCell`] because the two rule sites that observe a clash and a merge — [`Hyper::fire`]
    /// and [`Hyper::apply`] — hold the driver through `&self`, exactly as
    /// [`Graph`](crate::owl_dl::graph::Graph)'s work meter is a `Cell` for the same reason.
    trace: Option<RefCell<Recorder>>,
}

/// Decide whether the knowledge base plus `assumptions` has a consistent completion,
/// spending at most `budget`'s derivation rounds and work units.
pub(crate) fn decide(kb: &Kb, assumptions: &Assumptions<'_>, budget: Budget) -> Decision {
    let mut h = Hyper::new(kb, budget);
    let st = h.g.init_state(assumptions);
    h.run(st)
}

/// Decide as [`decide`] does, and ALSO write down the clash witnesses, the merge provenance
/// and the concrete-domain clashes the search met.
///
/// The recording is an observation and never a lever: it consults no state the search reads,
/// charges nothing to the work meter, and takes no branch of its own. So the [`Decision`] this
/// returns is the one [`decide`] returns for the same arguments, which is asserted rather than
/// stated.
pub(crate) fn decide_recording(
    kb: &Kb,
    assumptions: &Assumptions<'_>,
    budget: Budget,
) -> (Decision, Recorder) {
    let mut h = Hyper::new_recording(kb, budget);
    let st = h.g.init_state(assumptions);
    let decision = h.run(st);
    let trace = h
        .trace
        .take()
        .unwrap_or_else(|| unreachable!("a recording driver holds a recorder"));
    (decision, trace.into_inner())
}

impl Hyper<'_> {
    /// Run the search from `st` and assemble the [`Decision`] it reached.
    fn run(&mut self, st: State) -> Decision {
        match self.solve(st) {
            Ok(consistent) => Decision {
                consistent,
                steps: self.steps,
                work: self.g.work().spent(),
                exhausted: false,
                stopped: false,
                peak_nodes: self.peak_nodes,
                disjunctions: self.disjunctions,
                peak_depth: self.peak_depth,
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
                steps: self.steps,
                work: self.g.work().spent(),
                exhausted: !self.stopped,
                stopped: self.stopped,
                peak_nodes: self.peak_nodes,
                disjunctions: self.disjunctions,
                peak_depth: self.peak_depth,
            },
        }
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
        Self::build(kb, budget, None)
    }

    /// The same driver, INSTRUMENTED: it writes down every clash witness, merge and
    /// concrete-domain clash it meets.
    fn new_recording(kb: &'a Kb, budget: Budget) -> Self {
        Self::build(kb, budget, Some(RefCell::new(Recorder::default())))
    }

    /// The one constructor both entry points share.
    fn build(kb: &'a Kb, budget: Budget, trace: Option<RefCell<Recorder>>) -> Self {
        let g = Graph::new(kb, budget.work);
        let clauses = derive(g.kb());
        Self {
            radius: clauses.match_radius(),
            single_reads: single_reads(g.kb(), g.patterns(), &clauses),
            region: RegionScratch::default(),
            clauses,
            g,
            steps: 0,
            cap: budget.steps,
            stopped: false,
            peak_nodes: 0,
            disjunctions: 0,
            peak_depth: 0,
            trace,
        }
    }

    /// Write down the clause instance that derived `false`, if this run is recording.
    ///
    /// The witness is OBSERVED in the completion graph by
    /// [`observe_body`](crate::owl_dl::proof::observe_body) rather than grounded from the
    /// clause, so it is a second reading of the instance and the checker's own grounding has
    /// something to disagree with. Nothing here consults or charges the work meter.
    fn record_clash(&self, st: &State, index: usize, x: usize, frame: &[usize]) {
        let Some(trace) = self.trace.as_ref() else {
            return;
        };
        let witness = observe_body(self.g.kb(), st, self.clauses.clause(index), frame);
        trace.borrow_mut().clash(ClashStep::new(
            index,
            node_ref(st, x),
            frame_refs(st, frame),
            witness,
        ));
    }

    /// Write down one identification and the clause instance that LICENSED it, if this run is
    /// recording.
    ///
    /// The licence is resolved from `site` — the place the disjunct being asserted came from —
    /// so a checker can ground that clause's head itself and find the identification there. The
    /// frame is turned into merge-invariant identities HERE rather than at the call site, so a
    /// non-recording run allocates nothing: `site` carries a borrowed slice of node indices and
    /// this is the only place it is read. Nothing here consults or charges the work meter.
    fn record_merge(&self, st: &State, identified: Identified, site: &ApplySite<'_>, atom: usize) {
        let Identified {
            cause,
            left,
            right,
            joined,
        } = identified;
        let Some(trace) = self.trace.as_ref() else {
            return;
        };
        let licence = match *site {
            ApplySite::Clause { index, frame } => MergeLicence::Clause {
                clause: index,
                frame: frame_refs(st, frame),
                atom,
            },
            ApplySite::Branch { branch, ordinal } => MergeLicence::Branch {
                branch,
                ordinal,
                atom,
            },
            ApplySite::Unrecorded => MergeLicence::Unrecorded,
        };
        trace.borrow_mut().merge(MergeStep::new(
            cause, left, right, joined, st.clash, licence,
        ));
    }

    /// Write down a concrete-domain clash, if this run is recording.
    fn record_data_clash(&self, st: &State, node: usize) {
        let Some(trace) = self.trace.as_ref() else {
            return;
        };
        trace.borrow_mut().data_clash(node_ref(st, node));
    }

    /// What the recorder held before an alternative was explored, if this run is recording.
    fn record_mark(&self) -> Option<RecorderMark> {
        Some(self.trace.as_ref()?.borrow().mark())
    }

    /// Write down a `⊔`-rule BRANCH POINT: the clause instance, the frame, and ALL of the
    /// alternatives it generated in the producer's own order.
    ///
    /// Returns the recorded step's index, so the alternative that led here can be filed against
    /// it. `None` when the run is not recording, or when the recording ceiling has been reached
    /// — in which case the proof is [`DlProof::truncated`](crate::DlProof::truncated) and its
    /// branch tree is refused wholesale rather than checked with a hole in it.
    ///
    /// The alternatives are read off the vector the search is ABOUT TO BRANCH OVER, never
    /// re-derived by calling [`ground_head`] a second time. That is the same discipline
    /// [`observe_body`](crate::owl_dl::proof::observe_body) keeps for a clash witness and it is
    /// what stops the checker's regeneration from being a comparison of one function with
    /// itself: a search that branched over a SHORTENED alternative list records the shortened
    /// list, and the checker's regeneration disagrees with it.
    ///
    /// Nothing here consults or charges the work meter.
    fn record_branch(&self, st: &State, branching: &Branching) -> Option<usize> {
        let trace = self.trace.as_ref()?;
        let site = branching.site.as_ref()?;
        let head: Vec<_> = branching.alternatives[..site.head_alternatives]
            .iter()
            .map(|disjunct| observe_alternative(st, disjunct))
            .collect();
        let introduced: Vec<_> = branching.alternatives[site.head_alternatives..]
            .iter()
            .map(|disjunct| observe_alternative(st, disjunct))
            .collect();
        trace.borrow_mut().branch(BranchStep::new(
            site.index,
            node_ref(st, site.x),
            frame_refs(st, &site.frame),
            head,
            introduced,
        ))
    }

    /// File the outcome of the alternative `slot` names.
    fn record_outcome(&self, slot: &OpenSlot, outcome: BranchOutcome) {
        let Some(trace) = self.trace.as_ref() else {
            return;
        };
        trace
            .borrow_mut()
            .outcome(slot.branch, slot.ordinal, outcome);
    }

    /// File the outcome of the alternative `slot` names as whatever closed it — the clash step,
    /// data clash or clashing merge the recorder wrote down since the slot was opened.
    fn record_closure(&self, slot: &OpenSlot) {
        let Some(trace) = self.trace.as_ref() else {
            return;
        };
        let mut recorder = trace.borrow_mut();
        let outcome = recorder.closure_since(&slot.mark);
        recorder.outcome(slot.branch, slot.ordinal, outcome);
    }

    /// File the ROOT state's outcome — the one closure or branch point that is not an
    /// alternative of anything.
    fn record_root(&self, outcome: BranchOutcome) {
        let Some(trace) = self.trace.as_ref() else {
            return;
        };
        trace.borrow_mut().root(outcome);
    }

    /// File the root's outcome as whatever closed it.
    fn record_root_closure(&self, mark: Option<&RecorderMark>) {
        let Some(trace) = self.trace.as_ref() else {
            return;
        };
        let mut recorder = trace.borrow_mut();
        let outcome = mark.map_or(BranchOutcome::Unrecorded, |mark| {
            recorder.closure_since(mark)
        });
        recorder.root(outcome);
    }

    /// Write down the CLASH-FREE COMPLETION the search stopped at, if this run is recording.
    ///
    /// Read straight off the state's node and edge vectors, which is unmetered: recomputing the
    /// neighbour closure here would call [`Graph::neighbors`], charge the work meter, and so
    /// make a recorded run reach a different [`Decision`] than an unrecorded one. The checker
    /// computes the closure itself from the caller's own role axioms instead.
    ///
    /// The blocking witnesses are the ones the LAST derivation round computed. That round
    /// derived nothing — it is the round whose `changed` was false, which is what ended
    /// [`Hyper::saturate`] — so the graph it read is the graph recorded here.
    fn record_completion(&self, st: &State) {
        let Some(trace) = self.trace.as_ref() else {
            return;
        };
        let blocks = trace.borrow().blocking().to_vec();
        let completion = observe_completion(st, &blocks);
        trace.borrow_mut().completion(completion);
    }

    /// Write down which node blocked which, if this run is recording.
    ///
    /// DIRECT blocking pairs only: an indirectly blocked node has no blocker of its own, and a
    /// checker recomputes indirect blocking from the recorded predecessors rather than being
    /// told it.
    fn record_blocking(&self, st: &State, pairs: &[(usize, usize)]) {
        let Some(trace) = self.trace.as_ref() else {
            return;
        };
        let pairs = pairs
            .iter()
            .map(|&(blocked, blocker)| (node_ref(st, blocked), node_ref(st, blocker)))
            .collect();
        trace.borrow_mut().set_blocking(pairs);
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
        // The alternative whose subtree is being explored right now, and where its outcome is
        // filed. `None` at the root, and `None` for every non-recording run — the whole of the
        // branch bookkeeping below is inside `if let Some(..)`/`Option::map`, reads no state
        // the search reads, takes no branch of its own, and charges nothing.
        let mut open: Option<OpenSlot> = None;
        // What the recorder held before the ROOT state was saturated, for the same reason.
        let root_mark = self.record_mark();
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
                let ordinal = level.dispensed;
                let record = level.record;
                match level.alternatives.next() {
                    Some(disjunct) => {
                        level.dispensed += 1;
                        // A sibling starts from a COPY of the level's state, and copying a
                        // completion graph costs its size. That is work the round cap cannot
                        // see at all — a clone happens between rounds — and on a knowledge
                        // base whose disjunctions interleave it is where a large share of an
                        // unbounded search goes.
                        self.g
                            .work()
                            .charge((level.state.nodes.len() + level.state.edges.len()) as u64 + 1);
                        let mut next = level.state.clone();
                        let slot = record.and_then(|branch| {
                            Some(OpenSlot {
                                branch,
                                ordinal,
                                mark: self.record_mark()?,
                            })
                        });
                        // An alternative's identifications are licensed by the branch point that
                        // dispensed it: the checker regenerates that point's alternatives from
                        // the clause it cites and finds the identification in one of them.
                        let site = record.map_or(ApplySite::Unrecorded, |branch| {
                            ApplySite::Branch { branch, ordinal }
                        });
                        if self.apply(&mut next, &disjunct, &site) {
                            open = slot;
                            pending = Some(next);
                        } else if let Some(slot) = slot.as_ref() {
                            // The alternative's own assertion closed the state: it never
                            // reached a saturation, so its closure is whatever `apply` wrote.
                            self.record_closure(slot);
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
                match open.take() {
                    Some(slot) => self.record_closure(&slot),
                    None => self.record_root_closure(root_mark.as_ref()),
                }
                continue;
            }
            match self.find_branch(&mut st) {
                Some(branching) => {
                    // One `⊔`-rule application, and one more level of search tree. Counted
                    // here rather than where an alternative is taken, because the rule is
                    // applied once and then its alternatives are walked: counting the walk
                    // would report the tree's EDGES under a name that says rule.
                    self.disjunctions = self.disjunctions.saturating_add(1);
                    let record = self.record_branch(&st, &branching);
                    let outcome = record.map_or(BranchOutcome::Unrecorded, BranchOutcome::Branch);
                    match open.take() {
                        Some(slot) => self.record_outcome(&slot, outcome),
                        None => self.record_root(outcome),
                    }
                    stack.push(Branches {
                        state: st,
                        alternatives: branching.alternatives.into_iter(),
                        dispensed: 0,
                        record,
                    });
                    self.peak_depth = self.peak_depth.max(stack.len() as u64);
                }
                // No disjunction left to branch on: a clash-free completion, which is the
                // answer for the whole search rather than for this level alone — provided the
                // scan that found no open disjunction ran to the end, which an out-of-budget
                // one does not.
                None => {
                    self.check_work()?;
                    match open.take() {
                        Some(slot) => self.record_outcome(&slot, BranchOutcome::Open),
                        None => self.record_root(BranchOutcome::Open),
                    }
                    self.record_completion(&st);
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
            // The closures this state has cached take in the edges appended since the last
            // round, and what that adds to them is what this round sees for the first time.
            self.g.begin_round(st, self.steps);
            // Twice per round, and both are needed. The first measures the graph this round
            // INHERITED, which is the only observation a round that clashes before deriving
            // anything ever makes; the second measures what the round MINTED, and is taken
            // before the clash test so that a branch which closed only after growing large is
            // measured rather than discarded — that branch is exactly the one a reader of this
            // counter is looking for.
            self.observe(st);
            // Delta saturation. The nodes written since the last round began, and those whose
            // blocking flipped, are the changes; only a root whose own reading changed can newly
            // clash in the data domain, and only a root a match can READ a change from can match
            // anything new. Everything else already matched exactly this, and derived it.
            let touched = st.nodes.take_touched();
            let new_edges = st.edges_seen..st.edges.len();
            st.edges_seen = st.edges.len();
            let flips = self.update_blocking(st, &touched);
            let changed = self.changed_roots(st, &touched, &flips);
            if let Some(node) = self.concrete_domain_clashes(st, &changed) {
                self.record_data_clash(st, node);
                st.clash = true;
                return Ok(false);
            }
            let affected = if self.g.kb().rematches_everything() {
                (0..st.nodes.len())
                    .filter(|&x| find(st, x) == x)
                    .map(|node| Affected {
                        node,
                        full: true,
                        via: 0,
                    })
                    .collect()
            } else {
                region(
                    &mut self.region,
                    &self.g,
                    st,
                    &Changes {
                        nodes: &changed,
                        edges: new_edges,
                    },
                    self.radius,
                    self.g.patterns(),
                )
            };
            // A disjunction opens only where a body gains a match, which is where this round
            // re-matches; the `⊔`-rule's scan looks there and at what it left open.
            for affected in &affected {
                st.open.insert(affected.node);
            }
            let changed = self.round(st, &affected);
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

    /// The FIRST node whose CONCRETE-domain constraints have no solution, if any.
    ///
    /// The one decision this calculus does not take through a clause — see the module docs —
    /// and it is [`crate::owl_dl::data`]'s answer, shared verbatim with the incumbent.
    ///
    /// It answers with the node rather than a bare `bool` so an instrumented run can NAME the
    /// node it closed on. The scan is the same scan: `find` short-circuits at the first `true`
    /// exactly as the `any` it replaced did, so the same nodes are examined, the same work is
    /// charged, and the same state closes.
    fn concrete_domain_clashes(&self, st: &State, changed: &[usize]) -> Option<usize> {
        // A node's data-domain answer is a function of its own reading, so only a root whose
        // reading moved since it was last checked can have changed it — and `changed` is those
        // roots, ascending, so the first that clashes is the one a full scan found first.
        changed
            .iter()
            .copied()
            .find(|&x| find(st, x) == x && self.g.data_clashes(st, x))
    }

    /// The roots that changed since the last round began, ascending: every node written
    /// since, resolved to its root, and every node whose blocking flipped.
    ///
    /// One more change is not local. A blocked node's `≥n R.{o}` obligation is exempt from
    /// blocking exactly when the nominal `o` counts over an inverse
    /// ([`Graph::nominal_counts_over_inverse`]), which reads `o`'s label from wherever the
    /// blocked node is. So a nominal whose label changed while it bounds an inverse count makes
    /// every blocked node a change of its own.
    fn changed_roots(&self, st: &State, touched: &[usize], flips: &[usize]) -> Vec<usize> {
        let mut changed: Vec<usize> = touched.iter().chain(flips).map(|&x| find(st, x)).collect();
        changed.sort_unstable();
        changed.dedup();
        if changed
            .iter()
            .any(|&x| self.g.bounds_an_inverse_count(st, x))
        {
            // Rare by construction — a nominal bounding an inverse count whose label moved —
            // so the one pass over the blocked set it takes is not a per-round cost.
            changed.extend(
                (0..st.nodes.len()).filter(|&x| st.blocking.is_blocked(x) && find(st, x) == x),
            );
            changed.sort_unstable();
            changed.dedup();
        }
        // One unit per change taken in: the log is what a round now costs to find out.
        self.g.work().charge(touched.len() as u64 + 1);
        changed
    }

    /// The clauses no concept triggers that can match at the root `x`, in clause order: those
    /// an incident edge of `x` can satisfy ([`ClauseSet::edge_triggered`]) and the few nothing
    /// triggers ([`ClauseSet::untriggered`]).
    fn untriggered_at(&self, st: &State, x: usize) -> Vec<usize> {
        let mut open: Vec<usize> = self.clauses.untriggered().to_vec();
        for &edge in st.class_edges(x) {
            let (from, to, property) = st.edges[edge];
            if find(st, from) == x {
                open.extend_from_slice(self.clauses.edge_triggered((property, true)));
            }
            if find(st, to) == x {
                open.extend_from_slice(self.clauses.edge_triggered((property, false)));
            }
        }
        open.sort_unstable();
        open.dedup();
        open
    }

    /// What a [`SingleRead`] clause's read at the root `x` sees that the clause has not been
    /// matched against: the members its closures gained for this round, and the members whose
    /// own reading changed — or `None` when a closure it follows through `via` is not cached,
    /// so the gain is not known and the clause is matched in full.
    ///
    /// Sound for exactly the roots a round reaches through transitive closures alone. Such a
    /// root is more than [`ClauseSet::match_radius`] plain reads from every change, so its
    /// one-edge neighbours and their readings are as they were, and the patterns outside `via`
    /// reach no change at all; what can be new to the read is in the closures through `via`,
    /// and there it is what an appended edge added ([`Reach::fresh_in`]) or a member whose own
    /// reading changed. Every member was a member when the clause last matched here or was
    /// appended for this round, because an edge that extends a closure is a change the region
    /// of the next round reaches its owner from.
    fn delta_of(&self, st: &State, single: &SingleRead, x: usize, via: u64) -> Option<Vec<usize>> {
        let mut delta: Vec<usize> = Vec::new();
        for &index in &single.patterns {
            if TransitivePatterns::bit(index) & via == 0 {
                continue;
            }
            let reach = self.g.reach_of(st, x, index)?;
            delta.extend_from_slice(reach.fresh_in(self.steps));
            let changed = &self.region.changed;
            if changed.len() <= reach.order().len() {
                delta.extend(changed.iter().copied().filter(|&c| reach.contains(c)));
            } else {
                delta.extend(
                    reach
                        .order()
                        .iter()
                        .copied()
                        .filter(|y| changed.binary_search(y).is_ok()),
                );
            }
        }
        let mut seen = std::collections::BTreeSet::new();
        delta.retain(|&y| seen.insert(y));
        Some(delta)
    }

    /// One derivation round: every non-disjunctive clause instance, applied once.
    ///
    /// Matches are collected before they are applied, because applying one — a merge, or a
    /// minted witness — changes the graph the others were found in. A match invalidated that
    /// way is re-checked against the current state before it is applied (every node index is
    /// resolved through [`find`]), so the worst a stale match can be is redundant.
    fn round(&self, st: &mut State, affected: &[Affected]) -> bool {
        let mut changed = false;
        // Labelled so the trigger and clause scans below can bail out of the WHOLE round the
        // moment the meter reports exhausted, rather than finishing the node they were on and
        // every node after it. `saturate` still gates the verdict on `check_work` before it
        // trusts this round's `changed` — see [`Hyper::check_work`] — so stopping here only
        // shortens the latency between the cap being reached and that gate firing; it can never
        // by itself turn a truncated scan into a wrong answer.
        'nodes: for &Affected { node: x, full, via } in affected {
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
            if !full {
                // Reached through nothing but transitive closures: only a clause that reads one
                // of them can match anything new here, so only those are tried.
                for &index in self.clauses.transitive_readers() {
                    if self.clauses.reads(index) & via == 0 {
                        continue;
                    }
                    self.g.work().charge(1);
                    if self.g.work().exhausted() {
                        break 'nodes;
                    }
                    if !object_domain && self.clauses.is_tbox(index) {
                        continue;
                    }
                    if let Some(trigger) = self.clauses.clause(index).trigger()
                        && !self.g.has_concept(st, x, trigger)
                    {
                        continue;
                    }
                    let delta = self.single_reads[index]
                        .as_ref()
                        .and_then(|single| self.delta_of(st, single, x, via));
                    changed |= self.fire_over(st, index, x, delta.as_deref());
                    if st.clash {
                        return changed;
                    }
                }
                continue;
            }
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
                    changed |= self.fire(st, index, x);
                    if st.clash {
                        return changed;
                    }
                }
            }
            for index in self.untriggered_at(st, x) {
                self.g.work().charge(1);
                if self.g.work().exhausted() {
                    break 'nodes;
                }
                if !object_domain && self.clauses.is_tbox(index) {
                    continue;
                }
                changed |= self.fire(st, index, x);
                if st.clash {
                    return changed;
                }
            }
        }
        changed
    }

    /// Apply every match of clause `index` rooted at node `x`, if its head is not a
    /// disjunction. Returns whether the graph changed.
    fn fire(&self, st: &mut State, index: usize, x: usize) -> bool {
        self.fire_over(st, index, x, None)
    }

    /// [`Self::fire`], over the matches whose single read binds one of `delta` alone when it is
    /// given — see [`SingleRead`].
    fn fire_over(&self, st: &mut State, index: usize, x: usize, delta: Option<&[usize]>) -> bool {
        let clause = self.clauses.clause(index);
        let form = clause.head_form();
        if form == HeadForm::Disjunctive {
            return false;
        }
        // A head that only adds concepts to the root, all already in its label, is satisfied by
        // every instance the body could match: matching would find them and assert nothing.
        if let [disjunct] = clause.head.as_slice()
            && !disjunct.is_empty()
            && disjunct.iter().all(|atom| {
                matches!(*atom, HeadAtom::Concept { var: 0, concept } if self.g.has_concept(st, x, concept))
            })
        {
            return false;
        }
        let mut instances: Vec<Vec<usize>> = Vec::new();
        let mut collect = |frame: &[usize]| {
            instances.push(frame.to_vec());
            // An empty head is `false`: the first match refutes the state, so there is
            // nothing to learn from the rest.
            form == HeadForm::Inconsistency
        };
        match (delta, self.single_reads[index].as_ref()) {
            (Some(delta), Some(single)) => {
                Self::for_each_delta_match(&self.g, st, clause, single, x, delta, &mut collect);
            }
            _ => {
                Self::for_each_match(&self.g, st, clause, x, &mut collect);
            }
        }
        if instances.is_empty() {
            return false;
        }
        if form == HeadForm::Inconsistency {
            // The clash IS this clause instance, and an instrumented run writes it down before
            // the state closes over it: `for_each_match` stopped at the first match, so
            // `instances` holds exactly the frame that derived `false`.
            if let Some(frame) = instances.first() {
                self.record_clash(st, index, x, frame);
            }
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
            //
            // The one exception is an at-least whose filler is a NOMINAL that counts over an
            // inverse (`≥n R.{o}` with `o` bounded on `R⁻`). Satisfying it mints a witness
            // labelled `{o}`, but the nominal identification immediately merges that witness into
            // the EXISTING root `o` (OWL 2 has no unique-name assumption), so no PERSISTENT
            // blockable node survives and no unbounded blockable chain can grow from it —
            // termination is preserved, and the unravelling argument is unaffected because the
            // blocker carries the same obligation to the same root. Withholding it, on the other
            // hand, hides the blocked node from the nominal's own inverse-role count — exactly the
            // incompleteness the nominal-introduction rule repairs — so it must fire even under
            // blocking. See [`crate::owl_dl::tableau`] for the concept-tree's mirror.
            if disjunct.iter().any(|atom| {
                matches!(atom, Ground::AtLeast(node, _n, _role, filler)
                    if is_blocked(st, *node)
                        && !self.g.nominal_counts_over_inverse(st, *filler))
            }) {
                continue;
            }
            // The head was NOT satisfied, so asserting it moves the graph: every atom's
            // assertion is a change exactly when its satisfaction test was false (a concept
            // enters a label, a loop appears, a witness is minted, two nodes become one), which
            // is what makes this `true` rather than a second "did anything happen" flag —
            // and what makes the round loop terminate instead of re-firing a satisfied head.
            changed = true;
            if !self.apply(
                st,
                &disjunct,
                &ApplySite::Clause {
                    index,
                    frame: &frame,
                },
            ) {
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
    /// time, 9,800 now — and
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
    fn find_branch(&self, st: &mut State) -> Option<Branching> {
        // The scan this index replaced, run first and its charges handed back, so the tests
        // check every branch point against it while charging exactly what a shipped build does.
        #[cfg(test)]
        let expected = {
            let spent = self.g.work().spent();
            let scanned = (0..st.nodes.len())
                .find_map(|x| (find(st, x) == x).then(|| self.branch_at(st, x)).flatten());
            let exhausted = self.g.work().exhausted();
            self.g.work().restore(spent);
            (!exhausted).then(|| scanned.map(|branching| branching.alternatives))
        };
        let mut from = 0;
        let found = loop {
            let Some(x) = st.open.first_from(from) else {
                break None;
            };
            from = x + 1;
            self.g.work().charge(1);
            if x >= st.nodes.len() || find(st, x) != x {
                st.open.remove(x);
                continue;
            }
            if let Some(branching) = self.branch_at(st, x) {
                break Some(branching);
            }
            if self.g.work().exhausted() {
                // Out of budget mid-scan: stop rather than walk the rest of the graph for an
                // answer the driver is about to discard. The `None` is not read as "no open
                // disjunction" — `solve` checks the meter before it believes one — and the
                // root stays in the index, since its scan did not finish.
                break None;
            }
            st.open.remove(x);
        };
        #[cfg(test)]
        if let Some(expected) = expected
            && !self.g.work().exhausted()
        {
            assert_eq!(
                found.as_ref().map(|branching| &branching.alternatives),
                expected.as_ref(),
                "the open-disjunction index found a different branch point than a full scan"
            );
        }
        found
    }

    /// The first open disjunction at the root `x`, in the round's own order: the label's
    /// concepts ascending with their clauses in derivation order, then the untriggered ones.
    fn branch_at(&self, st: &State, x: usize) -> Option<Branching> {
        let triggers: Vec<u32> = st.nodes[x].label.iter().copied().collect();
        // The branch-point scan is charged exactly as the round's is, and it is charged for
        // the reason [`Hyper::find_branch`]'s own measurements give: a branch point is chosen
        // BETWEEN rounds, so every clause this scan matches — including the `≤n` clauses whose
        // bodies enumerate successor subsets — used to be work no budget could see. This is
        // the counter that sees it.
        self.g.work().charge(triggers.len() as u64 + 1);
        for concept in triggers {
            for &index in self.clauses.triggered_by(concept) {
                self.g.work().charge(1);
                if let Some(branch) = self.branch_of(st, index, x) {
                    return Some(branch);
                }
            }
        }
        for index in self.untriggered_at(st, x) {
            self.g.work().charge(1);
            if let Some(branch) = self.branch_of(st, index, x) {
                return Some(branch);
            }
        }
        None
    }

    /// The grounded alternatives of clause `index` at node `x`, if it is a disjunction with no
    /// satisfied disjunct.
    ///
    /// A RECORDING run also captures the site — the clause, the node, the frame and how many of
    /// the alternatives came from the clause head rather than from the `NI`-rule — because a
    /// proof term that named no clause instance would give a checker nothing to regenerate the
    /// alternatives from. The capture is one `Option` test and a frame clone; it charges
    /// nothing, decides nothing, and is absent from every non-recording run.
    fn branch_of(&self, st: &State, index: usize, x: usize) -> Option<Branching> {
        let clause = self.clauses.clause(index);
        if clause.head_form() != HeadForm::Disjunctive {
            return None;
        }
        let recording = self.trace.is_some();
        let mut found: Option<Branching> = None;
        Self::for_each_match(&self.g, st, clause, x, &mut |frame| {
            let mut disjuncts = ground_head(&clause.head, frame);
            let head_alternatives = disjuncts.len();
            // Motik–Shearer–Horrocks Table 5 `NI`-rule: at a NOMINAL at-most node pressed by a
            // BLOCKABLE predecessor, add the alternatives that branch that predecessor into the
            // reserved roots `u.⟨R,B,i⟩`. This is what bounds the corner the plain pairwise
            // `≤`-merges cannot, and it decides beside them rather than replacing them.
            self.append_ni_alternatives(st, clause, x, &mut disjuncts);
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
            found = Some(Branching {
                alternatives: disjuncts,
                site: recording.then(|| BranchSite {
                    index,
                    x,
                    frame: frame.to_vec(),
                    head_alternatives,
                }),
            });
            true
        });
        found
    }

    /// Append the Motik–Shearer–Horrocks Table 5 `NI`-rule alternatives for the `≤`-clause
    /// `clause` at node `x`.
    ///
    /// The rule fires only in the corner: `x` is a NOMINAL (named or generated reserved root,
    /// [`Graph::nominal_id`], never a mere anonymous root) whose at-most bound `≤n R.B` is pressed
    /// by a BLOCKABLE predecessor `y` satisfying `B` (`x` a completion-successor of `y` —
    /// [`Graph::blockable_predecessor_neighbours`]). For each such `y`, an alternative merges `y`
    /// into one of the reserved roots `u.⟨R,B,i⟩` (`u = ` the nominal identity of `x`, `i ∈
    /// 0..n`), which converts unbounded blockable predecessors into a finite reserved set and is
    /// what the plain pairwise `≤`-merges cannot do without renaming a named element or looping.
    /// Non-corner `≤`-clauses gain nothing: `nominal_id` is `None` for anonymous nodes and the
    /// blockable-predecessor set is empty when no inverse edge makes `y` an `R`-neighbour of `x`.
    fn append_ni_alternatives(
        &self,
        st: &State,
        clause: &DlClause,
        x: usize,
        disjuncts: &mut Vec<Vec<Ground<usize>>>,
    ) {
        // The `≤`-clause names its counted `(role, filler, n+1)` in a `Successors` body atom.
        let Some((role, filler, count)) = clause.body.iter().find_map(|atom| match *atom {
            BodyAtom::Successors {
                role,
                filler,
                count,
                ..
            } => Some((role, filler, count)),
            _ => None,
        }) else {
            return;
        };
        let n = count.saturating_sub(1);
        if n == 0 {
            return;
        }
        let Some(origin) = self.g.nominal_id(st, x) else {
            return;
        };
        for y in self.g.blockable_predecessor_neighbours(st, x, role) {
            // Only a `B`-neighbour presses `≤n R.B`.
            if !self.g.has_concept(st, y, filler) {
                continue;
            }
            for index in 0..n {
                disjuncts.push(vec![Ground::EqualReserved(
                    y,
                    GeneratedRoot {
                        origin: origin.clone(),
                        role,
                        filler,
                        index,
                    },
                )]);
            }
        }
    }

    /// Whether every atom of a grounded disjunct already holds.
    fn satisfied(&self, st: &State, disjunct: &[Ground<usize>]) -> bool {
        disjunct.iter().all(|atom| match atom {
            Ground::Concept(node, concept) => self.g.has_concept(st, *node, *concept),
            Ground::SelfLoop(node, role) => self.g.has_self_loop(st, *node, *role),
            Ground::AtLeast(node, n, role, filler) => {
                self.g.has_at_least(st, *node, *n, *role, *filler)
            }
            Ground::Equal(left, right) => find(st, *left) == find(st, *right),
            Ground::EqualIndividual(node, individual) => {
                st.nodes[find(st, *node)].nominals.contains(individual)
            }
            // Satisfied only once the reserved root exists and is already the node's identity.
            Ground::EqualReserved(node, key) => st
                .generated_root_of
                .get(key)
                .is_some_and(|&r| find(st, r) == find(st, *node)),
        })
    }

    /// Assert every atom of a grounded disjunct. Returns `false` if the state clashed.
    ///
    /// `site` says where the disjunct came from, so an identification can record the clause
    /// instance that LICENSED it. It is read only by [`Hyper::record_merge`], and only on a
    /// recording run: constructing one is an enum with a borrowed slice in it, which allocates
    /// nothing, decides nothing and charges nothing.
    fn apply(&self, st: &mut State, disjunct: &[Ground<usize>], site: &ApplySite<'_>) -> bool {
        for (at, atom) in disjunct.iter().enumerate() {
            match atom {
                Ground::Concept(node, concept) => {
                    self.g.add_concept(st, *node, *concept);
                }
                Ground::SelfLoop(node, role) => {
                    self.g.add_self_loop(st, *node, *role);
                }
                Ground::AtLeast(node, n, role, filler) => {
                    self.g.ensure_at_least(st, *node, *n, *role, *filler);
                }
                Ground::Equal(left, right) => {
                    // The two identities are read BEFORE the merge, because a merge is exactly
                    // the event that makes one of them stop being a representative. Both are
                    // frame slots the `≤n` clause's own body counted, which is what lets a
                    // checker re-derive the pair rather than believe it.
                    let (before_left, before_right) = (node_ref(st, *left), node_ref(st, *right));
                    self.g.merge_nodes(st, *left, *right);
                    let joined = node_ref(st, *left);
                    self.record_merge(
                        st,
                        Identified {
                            cause: MergeCause::AtMost,
                            left: before_left,
                            right: before_right,
                            joined,
                        },
                        site,
                        at,
                    );
                }
                Ground::EqualIndividual(node, individual) => {
                    let root = self.g.root(st, *individual);
                    let before_node = node_ref(st, *node);
                    self.g.merge_nodes(st, root, *node);
                    let joined = node_ref(st, *node);
                    // The absorbing side is recorded as the identity the `o`-clause NAMED,
                    // `{a}`, rather than as whichever name `node_ref` currently canonicalizes
                    // that root to: both are identities of the same node, and only the first is
                    // one a checker holding a proof term can re-derive. See `MergeStep::left`.
                    self.record_merge(
                        st,
                        Identified {
                            cause: MergeCause::Nominal,
                            left: NodeRef::Individual(*individual),
                            right: before_node,
                            joined,
                        },
                        site,
                        at,
                    );
                }
                Ground::EqualReserved(node, key) => {
                    // Mint (or reuse) the reserved root and merge the blockable node INTO it —
                    // `merge_nodes` keeps the root, the MSH named-/reserved-root merge direction.
                    let r = self.g.generated_root(st, key.clone());
                    let node = find(st, *node);
                    let before_node = node_ref(st, node);
                    self.g.merge_nodes(st, r, node);
                    let joined = node_ref(st, node);
                    // The reserved root's own identity `u.⟨R,B,i⟩`, for the reason above.
                    self.record_merge(
                        st,
                        Identified {
                            cause: MergeCause::NominalIntroduction,
                            left: NodeRef::of_generated(key),
                            right: before_node,
                            joined,
                        },
                        site,
                        at,
                    );
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

    /// Call `visit` on every binding frame of a [`SingleRead`] clause rooted at `x` whose read
    /// binds a node of `delta`, in `delta`'s order; stop early when it answers `true`.
    ///
    /// The atoms before the read test `x` alone and are checked once; each node of `delta` is
    /// then bound to the read's variable — every one of them is a member of a closure the read
    /// follows, so each IS one of its neighbours — and the rest of the body is matched as
    /// [`Self::walk`] matches it.
    fn for_each_delta_match(
        g: &Graph<'a>,
        st: &State,
        clause: &DlClause,
        single: &SingleRead,
        x: usize,
        delta: &[usize],
        visit: &mut dyn FnMut(&[usize]) -> bool,
    ) -> bool {
        g.work().charge(delta.len() as u64 + 1);
        let x = find(st, x);
        let ahead = clause.body[..single.at].iter().all(|atom| match *atom {
            BodyAtom::Concept { concept, .. } => g.has_concept(st, x, concept),
            BodyAtom::Denotes { individual, .. } => st.nodes[x].nominals.contains(&individual),
            BodyAtom::Role { .. } | BodyAtom::Successors { .. } => {
                unreachable!("a single read's leading atoms test the root alone")
            }
        });
        if !ahead {
            return false;
        }
        let mut frame = vec![x];
        for &y in delta {
            frame.push(find(st, y));
            let stopped = Self::walk(g, st, clause, single.at + 1, &mut frame, visit);
            frame.pop();
            if stopped {
                return true;
            }
        }
        false
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
                    if g.is_neighbour(st, source, role, frame[to as usize]) {
                        return Self::walk(g, st, clause, at + 1, frame, visit);
                    }
                } else if let Some(&BodyAtom::Denotes { var, individual }) = clause.body.get(at + 1)
                    && var == to
                {
                    // The next atom pins the new variable to ONE node — the root denoting the
                    // individual — so the read is a membership test of that node rather than an
                    // enumeration filtered down to it: the same single match, found without
                    // reading past it.
                    let Some(&root) = st.root_of.get(&individual) else {
                        return false;
                    };
                    let target = find(st, root);
                    if g.is_neighbour(st, source, role, target) {
                        frame.push(target);
                        let stopped = Self::walk(g, st, clause, at + 1, frame, visit);
                        frame.pop();
                        return stopped;
                    }
                } else {
                    // A pooled buffer, held while the rest of the body is matched under each
                    // neighbour: the recursion below takes buffers of its own.
                    let neighbours = g.neighbors(st, source, role);
                    for &y in neighbours.iter() {
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
                let mut counted = g.neighbors(st, frame[0], role);
                counted.retain(|&y| g.has_concept(st, y, filler));
                counted.sort_unstable();
                counted.dedup();
                return Self::walk_subsets(g, st, clause, at, frame, &counted, 0, count, visit);
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

    /// Bring [`State::blocking`] up to date with the nodes written since the last round —
    /// see the module docs for the discipline — and return the nodes whose status flipped.
    ///
    /// Blocking is the one ascending pass [`Self::blocking_reference`] makes, kept current
    /// rather than rerun. A node's status reads its own signature (its label, its incoming
    /// edge and its predecessor's label), its predecessor's status, and the earlier candidates
    /// with its signature, so the nodes to recompute are the ones written, the children of
    /// the ones written, and — as statuses flip — the children and later bucket-mates of what
    /// flipped. They are taken in ascending order from a queue that only ever gains LATER
    /// nodes, so every input a node reads is final when it is read: the same fixpoint the
    /// full pass computes, over what changed.
    ///
    /// A predecessor whose representative has a HIGHER index than the node — which only a
    /// merge can produce, since a successor is always created after its predecessor — is read
    /// as unblocked, because its status is not yet known in the pass. That under-blocks in
    /// that one case, which costs expansion and never a verdict: blocking withholds work, so
    /// doing the work anyway is what the calculus would have done without the optimization,
    /// and termination rests on DIRECT blocking alone (the signature argument in the module
    /// docs bounds the unblocked nodes whether or not indirect blocking fires).
    ///
    /// Charged one unit per node recomputed and per candidate compared.
    fn update_blocking(&self, st: &mut State, touched: &[usize]) -> Vec<usize> {
        let nodes = st.nodes.len();
        st.blocking.reserve(nodes);
        let mut queue: std::collections::BTreeSet<usize> = std::collections::BTreeSet::new();
        // The nodes whose signature may have been written: the ones written, and the children
        // of every root among them, whose signature reads that root's label.
        let mut rewritten: std::collections::BTreeSet<usize> = std::collections::BTreeSet::new();
        for &t in touched {
            rewritten.insert(t);
            rewritten.extend(st.children_of(find(st, t)).iter().copied());
        }
        queue.extend(rewritten.iter().copied());
        let mut flips: Vec<usize> = Vec::new();
        while let Some(x) = queue.pop_first() {
            self.g.work().charge(1);
            let parent = st.nodes[x].parent.map(|p| find(st, p));
            let candidate = find(st, x) == x && !st.nodes[x].root;
            let key = match (candidate, parent) {
                (true, Some(parent)) => Some(self.signature_key(st, x, parent)),
                _ => None,
            };
            let was_blocked = st.blocking.blocked[x];
            let old_key = st.blocking.key[x];
            if old_key != key {
                if let Some(old) = old_key {
                    st.blocking.buckets.remove(old, x);
                }
                if let Some(new) = key {
                    st.blocking.buckets.insert(new, x);
                }
                st.blocking.key[x] = key;
            }
            let (now, blocker) = match (key, parent) {
                (Some(key), Some(parent)) => {
                    let earlier = st.blocking.buckets.members(key);
                    let earlier = &earlier[..earlier.partition_point(|&y| y < x)];
                    self.g.work().charge(earlier.len() as u64);
                    let directly = earlier.iter().copied().find(|&y| {
                        !st.blocking.blocked[y] && self.same_signature(st, x, y, parent)
                    });
                    let indirectly = parent < x && st.blocking.blocked[parent];
                    (directly.is_some() || indirectly, directly)
                }
                _ => (false, None),
            };
            st.blocking.blocked[x] = now;
            st.blocking.blocker[x] = blocker;
            // A later candidate is directly blocked by the FIRST earlier unblocked candidate
            // with its signature, so it can be affected only when an unblocked candidate left
            // or joined its bucket — or when one in it flipped.
            let later = |key: Option<u64>, queue: &mut std::collections::BTreeSet<usize>| {
                if let Some(key) = key {
                    let members = st.blocking.buckets.members(key);
                    queue.extend(
                        members[members.partition_point(|&y| y <= x)..]
                            .iter()
                            .copied(),
                    );
                }
            };
            if rewritten.contains(&x) {
                if !was_blocked {
                    later(old_key, &mut queue);
                }
                if !now {
                    later(key, &mut queue);
                }
            }
            if now != was_blocked {
                flips.push(x);
                later(key, &mut queue);
                let children = st.children_of(x);
                queue.extend(
                    children[children.partition_point(|&c| c <= x)..]
                        .iter()
                        .copied(),
                );
            }
        }
        #[cfg(test)]
        {
            let expected = self.blocking_reference(st);
            let actual: Vec<bool> = (0..nodes).map(|x| st.blocking.is_blocked(x)).collect();
            assert_eq!(
                actual, expected,
                "the blocking kept current is not the blocking a full pass computes"
            );
        }
        if self.trace.is_some() {
            let pairs: Vec<(usize, usize)> = (0..nodes)
                .filter_map(|x| Some((x, st.blocking.blocker.get(x).copied().flatten()?)))
                .collect();
            self.record_blocking(st, &pairs);
        }
        flips
    }

    /// Which nodes are blocked, by node index, computed from scratch — what
    /// [`Self::update_blocking`] keeps current, and what the tests check it against after
    /// every round.
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
    #[cfg(test)]
    fn blocking_reference(&self, st: &State) -> Vec<bool> {
        let n = st.nodes.len();
        let mut blocked = vec![false; n];
        // Candidate blockers bucketed by their blocking signature's fingerprint, each bucket in
        // insertion order. Every candidate sharing a node's signature shares its bucket, so the
        // first exact match in the bucket is the first in the whole candidate order — the
        // blocker the full scan found — while a node pays only its own bucket.
        let mut candidates: std::collections::BTreeMap<u64, Vec<usize>> =
            std::collections::BTreeMap::new();
        for x in 0..n {
            if find(st, x) != x || st.nodes[x].root {
                continue;
            }
            let Some(parent) = st.nodes[x].parent.map(|p| find(st, p)) else {
                continue;
            };
            let key = self.signature_key(st, x, parent);
            let bucket = candidates.get(&key).map_or(&[] as &[usize], Vec::as_slice);
            // One unit per candidate blocker considered: the bucket, not every earlier
            // unblocked node. `find` keeps the blocker it stopped on, which is the witness a
            // countermodel needs; `same_signature` decides, so a fingerprint collision can
            // never block a node.
            let directly = bucket
                .iter()
                .copied()
                .find(|&y| self.same_signature(st, x, y, parent));
            let indirectly = parent < x && blocked[parent];
            if directly.is_some() || indirectly {
                blocked[x] = true;
            } else {
                candidates.entry(key).or_default().push(x);
            }
        }
        blocked
    }

    /// A fingerprint of `x`'s blocking signature, `x`'s predecessor being `parent`: the parts
    /// [`Self::same_signature`] compares, folded in a fixed order. Equal signatures have equal
    /// fingerprints; a collision only costs a [`Self::same_signature`] check that refuses it.
    fn signature_key(&self, st: &State, x: usize, parent: usize) -> u64 {
        use purrdf_hash::fnv::{BASIS, fold};
        let label = |state: u64, node: usize| {
            st.nodes[node].label.iter().fold(
                fold(state, &(st.nodes[node].label.len() as u64).to_le_bytes()),
                |h, c| fold(h, &c.to_le_bytes()),
            )
        };
        let mut key = label(BASIS, x);
        if self.g.kb().labels_alone_block() {
            return key;
        }
        key = match st.nodes[x].incoming {
            Some((property, forward)) => {
                fold(fold(key, &[1, u8::from(forward)]), &property.to_le_bytes())
            }
            None => fold(key, &[0]),
        };
        label(key, parent)
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

/// A clause whose body makes exactly ONE neighbourhood read: atoms on the root, one role atom
/// from the root to a new variable, and atoms on that variable — `∀r.C`'s propagation, a domain
/// or range axiom, an absorbed `∃r.C ⊑ D` re-rooted at its filler.
///
/// A new match of such a clause binds the read's variable to a node the read did not return
/// before, or to one whose own reading changed; [`Hyper::delta_of`] finds those for a root the
/// round reaches through transitive closures alone, and the clause is matched against them
/// only.
#[derive(Debug, Clone)]
struct SingleRead {
    /// The role atom's position in the body.
    at: usize,
    /// The transitive patterns the read follows: its role's achievers over transitive roles.
    patterns: Vec<usize>,
}

/// Each clause's [`SingleRead`] shape, if it has one.
fn single_reads(
    kb: &Kb,
    patterns: &TransitivePatterns,
    clauses: &ClauseSet,
) -> Vec<Option<SingleRead>> {
    (0..clauses.count())
        .map(|index| {
            let body = &clauses.clause(index).body;
            let mut read = None;
            for (at, atom) in body.iter().enumerate() {
                match *atom {
                    BodyAtom::Concept { var, .. } | BodyAtom::Denotes { var, .. } => {
                        if var > u32::from(read.is_some()) {
                            return None;
                        }
                    }
                    BodyAtom::Role {
                        from: 0,
                        to: 1,
                        role,
                    } if read.is_none() => read = Some((at, role)),
                    BodyAtom::Role { .. } | BodyAtom::Successors { .. } => return None,
                }
            }
            let (at, role) = read?;
            Some(SingleRead {
                at,
                patterns: patterns.indices(kb, role),
            })
        })
        .collect()
}

/// One root a round re-matches, and how much of its clause set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Affected {
    /// The root.
    node: usize,
    /// Every clause applicable here is re-matched: the root changed itself, or a change is
    /// within reach of a read that is not a transitive closure.
    full: bool,
    /// Otherwise, the transitive patterns ([`TransitivePatterns::bit`]) through which a change
    /// is within reach — only a clause reading one of them is re-matched.
    via: u64,
}

/// The region search's per-node scratch: stamped, so a search touches only what it reaches.
#[derive(Default)]
struct RegionScratch {
    /// `stamp[x] == epoch` exactly when this search has reached `x`.
    stamp: Vec<u32>,
    /// The current search's stamp.
    epoch: u32,
    /// Fewest reads from `x` to a change, for a reached `x`.
    depth: Vec<u32>,
    /// Fewest reads from `x` to a change whose FIRST read is not a transitive closure.
    plain: Vec<u32>,
    /// The transitive patterns some first read from `x` reaches a change through.
    via: Vec<u64>,
    /// Every node this search reached, in the order it did.
    reached: Vec<usize>,
    /// The layer being expanded and the next one.
    layer: Vec<usize>,
    /// The layer the current one discovers.
    next: Vec<usize>,
    /// A closure walk's frontier.
    walk: Vec<usize>,
    /// The roots whose own reading changed, ascending: the region's distance-0 seeds, kept for
    /// [`Hyper::delta_of`].
    changed: Vec<usize>,
}

impl RegionScratch {
    /// Start a search over a graph of `nodes` nodes.
    fn begin(&mut self, nodes: usize) {
        if self.stamp.len() < nodes {
            self.stamp.resize(nodes, 0);
            self.depth.resize(nodes, 0);
            self.plain.resize(nodes, 0);
            self.via.resize(nodes, 0);
        }
        self.epoch = self.epoch.wrapping_add(1);
        if self.epoch == 0 {
            self.stamp.fill(0);
            self.epoch = 1;
        }
        self.reached.clear();
        self.layer.clear();
        self.next.clear();
    }

    /// Reach `x` at `depth` reads, the first of them plain when `plain` says so; whether that
    /// shortened its depth.
    fn relax(&mut self, x: usize, depth: u32, plain: bool) -> bool {
        if self.stamp[x] != self.epoch {
            self.stamp[x] = self.epoch;
            self.depth[x] = u32::MAX;
            self.plain[x] = u32::MAX;
            self.via[x] = 0;
            self.reached.push(x);
        }
        if plain {
            self.plain[x] = self.plain[x].min(depth);
        }
        let shorter = depth < self.depth[x];
        if shorter {
            self.depth[x] = depth;
        }
        shorter
    }
}

/// What changed since the last round began: the region search's two kinds of seed.
struct Changes<'a> {
    /// The roots whose own reading changed — a write to the node, or its blocking flipped —
    /// ascending.
    nodes: &'a [usize],
    /// The edges appended since, by index.
    edges: std::ops::Range<usize>,
}

/// The roots whose matches a change since the last round can alter, ascending, and how much of
/// each one's clause set — the region a delta round re-matches.
///
/// A clause match rooted at `x` is a tree of neighbourhood READS ([`Graph::neighbors`]), at
/// most `radius` of them deep ([`ClauseSet::match_radius`]). A read is either one edge — any
/// edge of the read node's class, which the search takes in both directions, over-approximating
/// the read's own — or one TRANSITIVE CLOSURE, which follows a transitive pattern's edges to
/// any length but only in the pattern's own direction. So a match at `x` can see a change only
/// when the change is at most `radius` reads from `x`, and this search computes that distance
/// from the changes outward: a breadth-first search in layers of one read, where a layer's
/// plain step is one edge out of a node's class and its closure step is the whole closure
/// walked BACK along the pattern's mirror — the nodes whose read over the pattern reaches the
/// node being expanded.
///
/// The two kinds of change are seeded differently, because they are read differently:
///
/// * a node whose own reading changed is at distance 0 — every read that reaches it, plain or
///   through a closure, sees the change;
/// * an appended edge is seen by the reads that CROSS it: each endpoint's own one-edge read is
///   one read from it, and a transitive pattern the edge realizes as a step `src → dst` now
///   reaches past `src` to `dst` and on, so `src` and every node whose closure over the
///   pattern reaches `src` read the change in one read — through that pattern alone. An edge
///   is not a change to its endpoints' labels, so it seeds no closure walk from either
///   endpoint over any other pattern, and none from `dst` at all.
///
/// A root at distance 0, or whose first read toward a change is a plain edge, is re-matched in
/// FULL. A root reached only through first reads that are transitive closures is re-matched
/// only for the clauses reading one of those closures ([`ClauseSet::transitive_readers`]):
/// every other clause there reads nothing a change reached. That is what keeps a long
/// transitive chain from re-matching end to end on every change: a successor minted at one end
/// is in the closure of every node the chain leads to it from, so those nodes re-run their
/// clauses over the chain's role in that direction — and nothing else.
///
/// A head is not a read here, and need not be: a head the last round found satisfied stays
/// satisfied as labels and edges grow, one it found unsatisfied it asserted, and the one way a
/// satisfied head stops being so — a merge of two counted witnesses — writes both, which puts
/// every node counting them one plain read away.
///
/// Charged one unit per edge examined and per node reached, so the meter sees the region.
fn region(
    scratch: &mut RegionScratch,
    g: &Graph<'_>,
    st: &State,
    changes: &Changes<'_>,
    radius: usize,
    patterns: &TransitivePatterns,
) -> Vec<Affected> {
    let radius = u32::try_from(radius).unwrap_or(u32::MAX);
    scratch.begin(st.nodes.len());
    scratch.changed.clear();
    scratch.changed.extend_from_slice(changes.nodes);
    for &c in changes.nodes {
        if scratch.relax(c, 0, true) {
            scratch.layer.push(c);
        }
    }
    if radius > 0 {
        for edge in changes.edges.clone() {
            g.work().charge(1);
            let (from, to, property) = st.edges[edge];
            let (from, to) = (find(st, from), find(st, to));
            for endpoint in [from, to] {
                if scratch.relax(endpoint, 1, true) {
                    scratch.next.push(endpoint);
                }
            }
            for (bit, forward, mirror) in patterns.steps() {
                for (src, realized) in [
                    (from, forward.binary_search(&(property, true)).is_ok()),
                    (to, forward.binary_search(&(property, false)).is_ok()),
                ] {
                    if realized {
                        reach_back(scratch, g, st, src, bit, mirror, 1, true);
                    }
                }
            }
        }
    }
    for depth in 0..radius {
        if (scratch.layer.is_empty() && scratch.next.is_empty()) || g.work().exhausted() {
            break;
        }
        let layer = std::mem::take(&mut scratch.layer);
        for &y in &layer {
            let edges = st.class_edges(y);
            g.work().charge(edges.len() as u64 + 1);
            for &edge in edges {
                let (from, to, _) = st.edges[edge];
                for z in [find(st, from), find(st, to)] {
                    if scratch.relax(z, depth + 1, true) {
                        scratch.next.push(z);
                    }
                }
            }
            for (bit, _, mirror) in patterns.steps() {
                reach_back(scratch, g, st, y, bit, mirror, depth + 1, false);
            }
        }
        scratch.layer = std::mem::take(&mut scratch.next);
        scratch.next = layer;
        scratch.next.clear();
    }
    let mut out: Vec<Affected> = scratch
        .reached
        .iter()
        .map(|&node| Affected {
            node,
            full: scratch.depth[node] == 0 || scratch.plain[node] <= radius,
            via: scratch.via[node],
        })
        .filter(|affected| affected.full || affected.via != 0)
        .collect();
    out.sort_unstable_by_key(|affected| affected.node);
    out
}

/// Reach every node whose read over one transitive pattern (`bit`) reaches `y`, at `depth`
/// reads, by walking the pattern's `mirror` back from `y`; `y` itself too when `inclusive`.
///
/// A node already reached through this pattern was reached by an earlier walk — in this layer
/// or a shallower one, at no greater depth — which went on past it, so this walk stops there.
#[allow(clippy::too_many_arguments)]
fn reach_back(
    scratch: &mut RegionScratch,
    g: &Graph<'_>,
    st: &State,
    y: usize,
    bit: u64,
    mirror: &[(u32, bool)],
    depth: u32,
    inclusive: bool,
) {
    scratch.walk.clear();
    if inclusive {
        scratch.walk.push(y);
    } else {
        g.work().charge(st.class_edges(y).len() as u64);
        for_each_step(st, y, mirror, |z| scratch.walk.push(z));
    }
    while let Some(z) = scratch.walk.pop() {
        if scratch.stamp[z] == scratch.epoch && scratch.via[z] & bit != 0 {
            continue;
        }
        if scratch.relax(z, depth, false) {
            scratch.next.push(z);
        }
        scratch.via[z] |= bit;
        g.work().charge(st.class_edges(z).len() as u64 + 1);
        for_each_step(st, z, mirror, |w| scratch.walk.push(w));
    }
}

/// Whether the node a grounded atom names is blocked.
///
/// A node minted during the current round is absent from the round's blocking vector and reads
/// as unblocked, which is the same conservative direction [`Hyper::blocking`] documents.
fn is_blocked(st: &State, node: usize) -> bool {
    st.blocking.is_blocked(find(st, node))
}

/// Ground a clause's whole head against a matched `frame`, expanding the one schematic atom.
///
/// The result is the `⊔`-rule's alternatives in authored order: each `head` entry contributes
/// one disjunct, except [`HeadAtom::EqualSomePair`], which contributes one disjunct per PAIR of
/// the successors it counted — in ascending `(i, j)` order, so the branch order is a function
/// of the clause and the graph and nothing else.
///
/// # This function is the whole of branch EXHAUSTIVENESS, and it is shared
///
/// "These were all the alternatives" is exactly the statement that a branch point's recorded
/// alternative list is this function's output for the cited clause and frame. A
/// [`DlProof`](crate::owl_dl::proof::DlProof)'s checker therefore calls THIS function — over
/// the CALLER's own clause set and the recorded frame, with
/// [`NodeRef`](crate::owl_dl::proof::NodeRef) as the node carrier — and compares. That closes
/// the dropped-disjunct forgery, and it is honestly a `trusted` check rather than an
/// `attested` one: it rests on
/// [`Grounding`](crate::owl_dl::proof::TrustBaseEntry::Grounding), which names this function.
/// What it is INDEPENDENT of is the search driver — [`Hyper::solve`], [`Hyper::saturate`],
/// [`Hyper::find_branch`] and the branch stack — which is where the state, and therefore the
/// bugs, live.
///
/// # Panics
///
/// Indexes `frame`, so a frame narrower than the head's variables panics. Every producer call
/// is on a frame the matcher just bound. The checker validates the recorded frame's width
/// against [`head_frame_width`](crate::owl_dl::proof::head_frame_width) FIRST and rejects a
/// short one as a malformed proof.
pub(crate) fn ground_head<N: Clone>(head: &[Vec<HeadAtom>], frame: &[N]) -> Vec<Vec<Ground<N>>> {
    let mut out: Vec<Vec<Ground<N>>> = Vec::with_capacity(head.len());
    for disjunct in head {
        match disjunct.as_slice() {
            [HeadAtom::EqualSomePair { first, count }] => {
                let first = *first as usize;
                let count = *count as usize;
                for left in 0..count {
                    for right in (left + 1)..count {
                        out.push(vec![Ground::Equal(
                            frame[first + left].clone(),
                            frame[first + right].clone(),
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
fn ground<N: Clone>(disjunct: &[HeadAtom], frame: &[N]) -> Vec<Ground<N>> {
    disjunct
        .iter()
        .map(|atom| match *atom {
            HeadAtom::Concept { var, concept } => {
                Ground::Concept(frame[var as usize].clone(), concept)
            }
            HeadAtom::SelfLoop { var, role } => Ground::SelfLoop(frame[var as usize].clone(), role),
            HeadAtom::AtLeast {
                var,
                n,
                role,
                filler,
            } => Ground::AtLeast(frame[var as usize].clone(), n, role, filler),
            HeadAtom::EqualIndividual { var, individual } => {
                Ground::EqualIndividual(frame[var as usize].clone(), individual)
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
    use super::{Hyper, MergeLicence, decide, decide_recording};
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

    /// The nominal-introduction spy point: a nominal `≤2 p⁻.⊤`, everything `p`-related to it,
    /// and an individual forced to `≥3 r.⊤`. INCONSISTENT, and it closes through the reserved
    /// roots — so it is the fixture that actually exercises the clash and merge recorders.
    fn spy_point_kb() -> Kb {
        const P: u32 = 40;
        const RR: u32 = 41;
        const SPY: u32 = 50;
        const U: u32 = 51;
        let mut kb = Kb::empty();
        kb.push_gci(
            Concept::Top,
            Concept::Some(Role::Named(P), Box::new(Concept::Nominal(vec![SPY]))),
        );
        let max2 = kb
            .table
            .intern(Concept::Max(2, Role::Inv(P), Box::new(Concept::Top)));
        let min3 = kb
            .table
            .intern(Concept::Min(3, Role::Named(RR), Box::new(Concept::Top)));
        kb.abox_types.push((SPY, max2));
        kb.abox_types.push((U, min3));
        kb.individuals.insert(SPY);
        kb.individuals.insert(U);
        kb.finalize();
        kb
    }

    /// How the edges of [`transitive_chain_kb`]'s chain realize the transitive role `r`.
    #[derive(Clone, Copy, Debug)]
    enum Link {
        /// Every edge is labelled `r`.
        Named,
        /// Every edge is labelled `s`, with `s ⊑ r`.
        SubRole,
        /// Every edge is stored backwards and labelled `s`, with `s owl:inverseOf r`.
        InversePartner,
    }

    /// `x : ∀r.D` over a chain `x r y1 r … r yn` whose last node is `∃r.E`, with `E ⊑ ¬D`.
    ///
    /// With `r` transitive the fresh `r`-successor of `yn` is an `r`-neighbour of `x`, so it
    /// is both `D` and `E`: INCONSISTENT at every length. With `r` not transitive, or without
    /// `E ⊑ ¬D`, it is consistent. Built so `x` is as far from the change as the chain is long.
    /// `link` says which edges spell the chain: `r` itself, a sub-role of `r`, or an inverse
    /// partner of `r` stored the other way round — the three spellings of one `r`-path.
    fn transitive_chain_kb(len: u32, transitive: bool, disjoint: bool, link: Link) -> Kb {
        const R: u32 = 60;
        const D: u32 = 61;
        const E: u32 = 62;
        const S: u32 = 63;
        const X: u32 = 100;
        let mut kb = Kb::empty();
        if transitive {
            kb.transitive.insert(R);
        }
        match link {
            Link::Named => {}
            Link::SubRole => {
                kb.role_sub.entry(R).or_default().insert(S);
            }
            Link::InversePartner => {
                kb.inverses.entry(R).or_default().insert(S);
                kb.inverses.entry(S).or_default().insert(R);
            }
        }
        if disjoint {
            kb.push_gci(Concept::Named(E), Concept::Not(Box::new(Concept::Named(D))));
        }
        let all_d = kb
            .table
            .intern(Concept::All(Role::Named(R), Box::new(Concept::Named(D))));
        let d = kb.table.intern(Concept::Named(D));
        let some_e = kb
            .table
            .intern(Concept::Some(Role::Named(R), Box::new(Concept::Named(E))));
        kb.abox_types.push((X, all_d));
        kb.individuals.insert(X);
        for i in 1..=len {
            let (prev, y) = (X + i - 1, X + i);
            kb.abox_roles.push(match link {
                Link::Named => (prev, R, y),
                Link::SubRole => (prev, S, y),
                Link::InversePartner => (y, S, prev),
            });
            kb.abox_types.push((y, d));
            kb.individuals.insert(y);
        }
        kb.abox_types.push((X + len, some_e));
        kb.finalize();
        kb
    }

    /// DELTA SATURATION MUST SEE THROUGH A TRANSITIVE ROLE: a change at the end of an
    /// `r`-chain re-matches the clauses at its start, however long the chain — and whichever
    /// of the role's sub-roles or inverse partners spells the chain's edges.
    ///
    /// A clause body atom over a transitive role reads that role's whole closure, so the
    /// nodes whose matches a change can alter are not bounded by the body's hop count. The
    /// chain lengths straddle every radius a clause set here can have; a re-match region
    /// counted in raw hops answered CONSISTENT from length 3 on — a decided, wrong verdict —
    /// and a closure that stepped only over edges carrying the transitive role's own name
    /// answered CONSISTENT at every length for the two other spellings.
    #[test]
    fn a_change_at_the_end_of_a_transitive_chain_reaches_its_start() {
        for link in [Link::Named, Link::SubRole, Link::InversePartner] {
            for len in [1, 2, 3, 4, 8, 16] {
                let kb = transitive_chain_kb(len, true, true, link);
                let clash = decide(&kb, &Assumptions::of_kb(), Budget::for_kb(&kb));
                assert!(
                    !clash.exhausted && !clash.stopped,
                    "{link:?} len {len}: {clash:?}"
                );
                assert!(
                    !clash.consistent,
                    "{link:?} len {len}: transitive clash missed: {clash:?}"
                );
                for (transitive, disjoint) in [(true, false), (false, true)] {
                    let kb = transitive_chain_kb(len, transitive, disjoint, link);
                    let control = decide(&kb, &Assumptions::of_kb(), Budget::for_kb(&kb));
                    assert!(
                        !control.exhausted && !control.stopped,
                        "{link:?} len {len}: {control:?}"
                    );
                    assert!(
                        control.consistent,
                        "{link:?} len {len}, transitive {transitive}, disjoint {disjoint}: \
                         {control:?}"
                    );
                }
            }
        }
    }

    /// A LABEL change at the far end of a transitive chain reaches a guarded clause at its
    /// start: `x : A`, `A ⊓ ∃r.B ⊑ ⊥` with `r` transitive, the chain `x r y1 … r yn`, and
    /// `yn : C` with `C ⊑ C1 ⊑ C2 ⊑ B`. `yn` gains `B` three rounds in, long after `x`'s
    /// match read — and cached — `r`'s closure; that round reaches `x` through the closure
    /// alone, and the only thing new to its read is a member whose label changed, since no
    /// edge was added. INCONSISTENT at every length; consistent without the transitivity or
    /// without the derivation of `B`.
    #[test]
    fn a_label_change_at_the_end_of_a_transitive_chain_reaches_a_guarded_clause() {
        const R: u32 = 60;
        const A: u32 = 61;
        const B: u32 = 62;
        const C: u32 = 63;
        const C1: u32 = 64;
        const C2: u32 = 65;
        const X: u32 = 100;
        let build = |len: u32, transitive: bool, derived: bool| {
            let mut kb = Kb::empty();
            if transitive {
                kb.transitive.insert(R);
            }
            kb.push_gci(
                Concept::And(vec![
                    Concept::Named(A),
                    Concept::Some(Role::Named(R), Box::new(Concept::Named(B))),
                ]),
                Concept::Bottom,
            );
            kb.push_gci(Concept::Named(C), Concept::Named(C1));
            kb.push_gci(Concept::Named(C1), Concept::Named(C2));
            if derived {
                kb.push_gci(Concept::Named(C2), Concept::Named(B));
            }
            let (a, c) = (
                kb.table.intern(Concept::Named(A)),
                kb.table.intern(Concept::Named(C)),
            );
            kb.abox_types.push((X, a));
            kb.individuals.insert(X);
            for i in 1..=len {
                kb.abox_roles.push((X + i - 1, R, X + i));
                kb.individuals.insert(X + i);
            }
            kb.abox_types.push((X + len, c));
            kb.finalize();
            kb
        };
        for len in [1, 2, 3, 4, 8, 16] {
            let kb = build(len, true, true);
            let clash = decide(&kb, &Assumptions::of_kb(), Budget::for_kb(&kb));
            assert!(
                !clash.exhausted && !clash.consistent,
                "len {len}: {clash:?}"
            );
            for (transitive, derived) in [(false, true), (true, false)] {
                let kb = build(len, transitive, derived);
                let control = decide(&kb, &Assumptions::of_kb(), Budget::for_kb(&kb));
                assert!(
                    !control.exhausted && (control.consistent || len == 1 && derived),
                    "len {len}, transitive {transitive}, derived {derived}: {control:?}"
                );
            }
        }
    }

    /// THE REGION AN EDGE BETWEEN TWO EXISTING NODES REACHES: over `0 r 1 r 2 r 3` and
    /// `4 r 5` with `r` transitive, appending `3 r 4` re-matches its two endpoints in full, the
    /// nodes whose `r`-closure now runs past `3` — `0`, `1`, `2` — for the clauses reading
    /// `r` forward only, and `5`, whose `r⁻`-closure now runs past `4`, for those reading `r⁻`
    /// only. No label changed, so nothing else is reached.
    #[test]
    fn an_edge_between_existing_nodes_reaches_the_closures_it_extends() {
        const R: u32 = 60;
        let mut kb = Kb::empty();
        kb.transitive.insert(R);
        kb.finalize();
        let g = crate::owl_dl::graph::Graph::new(&kb, u64::MAX);
        let mut st = g.init_state(&Assumptions::of_kb());
        for index in 0..6 {
            g.generated_root(
                &mut st,
                crate::owl_dl::graph::GeneratedRoot {
                    origin: crate::owl_dl::graph::NominalId::Named(0),
                    role: Role::Named(R),
                    filler: 0,
                    index,
                },
            );
        }
        for (from, to) in [(0, 1), (1, 2), (2, 3), (4, 5)] {
            st.push_edge(from, to, R);
        }
        let before = st.edges.len();
        st.push_edge(3, 4, R);
        let patterns = g.patterns();
        let forward = crate::owl_dl::clause::TransitivePatterns::bit(
            patterns.index((R, true)).expect("r is transitive"),
        );
        let backward = crate::owl_dl::clause::TransitivePatterns::bit(
            patterns.index((R, false)).expect("r is transitive"),
        );
        let mut scratch = super::RegionScratch::default();
        let affected = super::region(
            &mut scratch,
            &g,
            &st,
            &super::Changes {
                nodes: &[],
                edges: before..st.edges.len(),
            },
            1,
            patterns,
        );
        let partial = |node: usize, via: u64| super::Affected {
            node,
            full: false,
            via,
        };
        let full = |node: usize, via: u64| super::Affected {
            node,
            full: true,
            via,
        };
        assert_eq!(
            affected,
            vec![
                partial(0, forward),
                partial(1, forward),
                partial(2, forward),
                full(3, forward),
                full(4, backward),
                partial(5, backward),
            ]
        );
    }

    /// An edge appended between two EXISTING nodes — the change a branch's assertion can make
    /// without minting a node — extends every closure that runs into its source, and what it
    /// adds is new to each of them although no label changed: `x : ∀r.D` over `x r y1 … r y4`
    /// with `r` transitive, and a separate `z : ¬D`. Saturated, that is consistent; appending
    /// `y4 r z` then must reach `x`, whose closure gained `z`, and close the state.
    #[test]
    fn an_appended_edge_between_existing_nodes_is_new_to_the_closures_it_extends() {
        const R: u32 = 60;
        const D: u32 = 61;
        const X: u32 = 100;
        const Z: u32 = 200;
        let mut kb = Kb::empty();
        kb.transitive.insert(R);
        let all_d = kb
            .table
            .intern(Concept::All(Role::Named(R), Box::new(Concept::Named(D))));
        let not_d = kb.table.intern(Concept::Not(Box::new(Concept::Named(D))));
        kb.abox_types.push((X, all_d));
        kb.abox_types.push((Z, not_d));
        kb.individuals.insert(X);
        kb.individuals.insert(Z);
        for i in 1..=4 {
            kb.abox_roles.push((X + i - 1, R, X + i));
            kb.individuals.insert(X + i);
        }
        kb.finalize();
        for full in [false, true] {
            kb.full_rematch = full;
            let mut h = Hyper::new(&kb, Budget::for_kb(&kb));
            let mut st = h.g.init_state(&Assumptions::of_kb());
            assert_eq!(
                h.saturate(&mut st).ok(),
                Some(true),
                "saturated, consistent"
            );
            let (y4, z) = (h.g.root(&mut st, X + 4), h.g.root(&mut st, Z));
            st.push_edge(y4, z, R);
            assert_eq!(
                h.saturate(&mut st).ok(),
                Some(false),
                "full re-match {full}: `z` joined `x`'s closure and must get `D`"
            );
        }
    }

    /// THE DELTA DIFFERENTIAL OVER LONG CHAINS: delta saturation and a full re-match every
    /// round reach the same verdict on chains longer than any match radius.
    ///
    /// The oracle corpus runs the same differential, but over a signature small enough to
    /// enumerate, whose chains never outgrow the radius — which is how a region counted in raw
    /// hops passed it. These knowledge bases are built to: a chain of 3 to 14 individuals
    /// whose edges are mostly one of a transitive role, a second transitive role, a plain
    /// sub-role of the first, or the first asserted backwards (read through its inverse), with
    /// universals over each direction, existentials that create the late change, and an
    /// optional disjointness that turns the late change into a clash at the chain's far end.
    /// Generated by a fixed linear congruential sequence, so the corpus is the same every run.
    #[test]
    fn delta_saturation_agrees_with_a_full_rematch_over_long_role_chains() {
        const R: u32 = 70;
        const T: u32 = 71;
        const S: u32 = 72;
        const D: u32 = 73;
        const E: u32 = 74;
        const X: u32 = 200;
        let mut seed: u64 = 0x9E37_79B9_7F4A_7C15;
        let mut next = |bound: u64| {
            seed = seed
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (seed >> 33) % bound
        };
        let (mut compared, mut clashes) = (0_u32, 0_u32);
        for case in 0..600 {
            let mut kb = Kb::empty();
            kb.transitive.insert(R);
            kb.transitive.insert(T);
            kb.role_sub.entry(R).or_default().insert(S);
            if next(4) != 0 {
                kb.push_gci(Concept::Named(E), Concept::Not(Box::new(Concept::Named(D))));
            }
            let fillers = [
                Concept::Named(D),
                Concept::All(Role::Named(R), Box::new(Concept::Named(D))),
                Concept::All(Role::Inv(R), Box::new(Concept::Named(D))),
                Concept::All(Role::Named(T), Box::new(Concept::Named(D))),
                Concept::Some(Role::Named(R), Box::new(Concept::Named(E))),
                Concept::Some(Role::Named(S), Box::new(Concept::Named(E))),
                Concept::Some(Role::Inv(T), Box::new(Concept::Named(E))),
                Concept::All(Role::Named(S), Box::new(Concept::Named(D))),
            ];
            let len = 3 + u32::try_from(next(12)).expect("below 12");
            let main = next(4);
            for i in 0..len {
                let (a, b) = (X + i, X + i + 1);
                kb.individuals.insert(a);
                kb.individuals.insert(b);
                // One dominant edge kind per chain, broken now and then: a transitive closure
                // walks ONE property's edges, so a mixed chain rarely connects end to end.
                match if next(8) == 0 { next(4) } else { main } {
                    0 => kb.abox_roles.push((a, R, b)),
                    1 => kb.abox_roles.push((a, T, b)),
                    2 => kb.abox_roles.push((a, S, b)),
                    _ => kb.abox_roles.push((b, R, a)),
                }
            }
            // A universal at one end and an existential at the other, so the clash — when there
            // is one — is as far from the late change as the chain is long. Half the time the
            // pair is AIMED: the universal reads along the chain's dominant edge kind toward the
            // end where the existential mints its successor. Then a few more anywhere.
            let (first, last) = (X, X + len);
            let place = |at: u32, c: Concept, kb: &mut Kb| {
                let c = kb.table.intern(c);
                kb.abox_types.push((at, c));
            };
            let all = |role: Role| Concept::All(role, Box::new(Concept::Named(D)));
            let some = |role: Role| Concept::Some(role, Box::new(Concept::Named(E)));
            if next(2) == 0 {
                let (from, to, role) = match main {
                    0 => (first, last, Role::Named(R)),
                    1 => (first, last, Role::Named(T)),
                    2 => (first, last, Role::Named(S)),
                    _ => (last, first, Role::Named(R)),
                };
                if next(2) == 0 {
                    place(from, all(role), &mut kb);
                    place(to, some(role), &mut kb);
                } else {
                    let inverse = match role {
                        Role::Named(p) => Role::Inv(p),
                        Role::Inv(p) => Role::Named(p),
                    };
                    place(to, all(inverse), &mut kb);
                    place(from, some(inverse), &mut kb);
                }
            } else {
                let (near, far) = if next(2) == 0 {
                    (first, last)
                } else {
                    (last, first)
                };
                let pick = [1, 2, 3, 7][usize::try_from(next(4)).expect("below 4")];
                place(near, fillers[pick].clone(), &mut kb);
                let pick = [4, 5, 6][usize::try_from(next(3)).expect("below 3")];
                place(far, fillers[pick].clone(), &mut kb);
            }
            // A QUIESCENT middle, three times in four: every chain node already carries `D`, so
            // the first round derives nothing along the chain and the existential's successor
            // is the only change the next round sees. A middle that is still deriving moves
            // every node between the two ends and drags the re-match region back to the
            // universal by accident — the case that hides a region counted too short.
            if next(4) != 0 {
                for i in 0..=len {
                    place(X + i, Concept::Named(D), &mut kb);
                }
            }
            for _ in 0..next(3) {
                let at = X + u32::try_from(next(u64::from(len) + 1)).expect("in range");
                let pick = usize::try_from(next(fillers.len() as u64)).expect("in range");
                place(at, fillers[pick].clone(), &mut kb);
            }
            kb.finalize();
            let cap = Budget::for_kb(&kb);
            let delta = decide(&kb, &Assumptions::of_kb(), cap);
            kb.full_rematch = true;
            let full = decide(&kb, &Assumptions::of_kb(), cap);
            assert!(
                !delta.exhausted && !full.exhausted,
                "case {case}: {delta:?} {full:?}"
            );
            assert_eq!(
                delta.consistent, full.consistent,
                "case {case}: delta {delta:?}, full {full:?}"
            );
            compared += 1;
            clashes += u32::from(!full.consistent);
        }
        // Both verdicts must be well represented, or the agreement is about one of them only.
        assert!(
            clashes >= 60 && compared - clashes >= 60,
            "{clashes} of {compared}"
        );
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

    /// INSTRUMENTATION IS AN OBSERVATION, NOT A LEVER: a recorded run reaches the same
    /// [`Decision`](crate::owl_dl::graph::Decision) as an unrecorded one, field for field.
    ///
    /// This is the standing obligation on [`Hyper::trace`]. Every record site is one `Option`
    /// test that reads no state the search reads, takes no branch of its own, and — critically
    /// — never calls a METERED graph operation, so the work figure cannot move. Were any of
    /// those to change, `work` would move first (it counts every scan rather than every round),
    /// then `steps`, then the three shape counters, then the verdict; comparing the WHOLE
    /// struct is what makes a fourth field unable to escape the comparison.
    ///
    /// Four knowledge bases, deliberately, so every RECORD SITE is inside the comparison: the
    /// branching fixture (case splits and a BRANCH-POINT recording, minted witnesses, a
    /// cardinality bound and a COMPLETION recording), the `NI` spy point (reserved roots, merges
    /// and an INCONSISTENT verdict — the run that records clash witnesses and a whole refutation
    /// tree), the blocking chain (whose completion carries BLOCKING WITNESSES, recorded from
    /// inside the per-round blocking pass), and a refutation that closes through a case split.
    #[test]
    fn a_recorded_decision_is_identical_to_an_unrecorded_one() {
        for kb in [
            branching_kb(),
            spy_point_kb(),
            blocking_chain_kb(),
            closing_disjunction_kb(),
        ] {
            let cap = Budget::for_kb(&kb);
            let plain = decide(&kb, &Assumptions::of_kb(), cap);
            let (recorded, _) = decide_recording(&kb, &Assumptions::of_kb(), cap);
            assert_eq!(
                plain, recorded,
                "recording must not move a single field of the decision"
            );
        }
    }

    /// …and the same holds under the ASSUMPTION SHAPES the reasoning services ask questions
    /// with, not only under the bare knowledge base.
    ///
    /// Every certified service now records, and every service question reaches the decision
    /// core through a non-default [`Assumptions`]: a fresh anonymous witness carrying a
    /// concept (subsumption, class satisfiability, disjointness), an extra concept assertion on
    /// a named individual (realization, instance retrieval, class assertion), and an extra role
    /// assertion (role assertion, role inclusion). If recording moved a decision under any of
    /// those, a proof-carrying service would answer differently from the one that shipped —
    /// which is the failure this extends the standing obligation to cover.
    #[test]
    fn a_recorded_service_question_decides_identically_to_an_unrecorded_one() {
        let kb = spy_point_kb();
        let cap = Budget::for_kb(&kb);
        let types = [(IND, A)];
        let roles = [(IND, R, IND)];
        let shapes = [
            Assumptions {
                fresh_types: &[A],
                ..Assumptions::of_kb()
            },
            Assumptions {
                fresh_types: &[A, B],
                ..Assumptions::of_kb()
            },
            Assumptions {
                types: &types,
                ..Assumptions::of_kb()
            },
            Assumptions {
                roles: &roles,
                ..Assumptions::of_kb()
            },
            Assumptions {
                include_abox: false,
                fresh_types: &[C],
                ..Assumptions::of_kb()
            },
        ];
        for assumptions in &shapes {
            let plain = decide(&kb, assumptions, cap);
            let (recorded, _) = decide_recording(&kb, assumptions, cap);
            assert_eq!(
                plain, recorded,
                "recording must not move a decision under a service's own assumptions"
            );
        }
    }

    /// A recorded MERGE names the clause instance that licensed it.
    ///
    /// Without this the licence could be [`MergeLicence::Unrecorded`] at every site and every
    /// merge-replay test would still pass by refusing to check anything — the same reason the
    /// clash and branch recordings have an obligation of their own.
    #[test]
    fn a_recorded_merge_names_the_clause_instance_that_licensed_it() {
        let kb = spy_point_kb();
        let (_, recorder) = decide_recording(&kb, &Assumptions::of_kb(), Budget::for_kb(&kb));
        let merges = recorder.merges();
        assert!(
            !merges.is_empty(),
            "the nominal-introduction fixture identifies nodes"
        );
        assert!(
            merges
                .iter()
                .all(|merge| !matches!(merge.licence(), MergeLicence::Unrecorded)),
            "every identification the search made came from a grounded head atom, and the \
             record must name it: {merges:?}"
        );
    }

    /// A recorded refutation actually WRITES SOMETHING DOWN — otherwise the equality above
    /// would be satisfied by a recorder that recorded nothing.
    #[test]
    fn a_recorded_refutation_carries_a_clash_witness() {
        let kb = spy_point_kb();
        let (decision, recorder) =
            decide_recording(&kb, &Assumptions::of_kb(), Budget::for_kb(&kb));
        assert!(!decision.consistent && !decision.exhausted, "{decision:?}");
        assert!(
            !recorder.is_empty(),
            "an inconsistent run closes on something, and the recorder must name it"
        );
    }

    /// …and so do the two record sites this stage ADDED, for the same reason: the equality
    /// above is only evidence that recording is free if recording happened.
    ///
    /// A branching refutation must write a branch point down; a consistent branching run must
    /// write a completion down; and the blocking chain's completion must carry the blocking
    /// witnesses the per-round blocking pass observed. Without this, an instrumentation that
    /// quietly recorded neither would satisfy every other test here.
    #[test]
    fn a_recorded_run_writes_down_its_branch_points_and_its_completion() {
        let kb = closing_disjunction_kb();
        let (decision, recorder) =
            decide_recording(&kb, &Assumptions::of_kb(), Budget::for_kb(&kb));
        assert!(!decision.consistent && !decision.exhausted, "{decision:?}");
        assert!(
            decision.disjunctions > 0,
            "the fixture refutes through a case split: {decision:?}"
        );
        assert!(
            !recorder.branches().is_empty(),
            "a search that branched must record the branch point it branched at"
        );

        let kb = branching_kb();
        let (decision, recorder) =
            decide_recording(&kb, &Assumptions::of_kb(), Budget::for_kb(&kb));
        assert!(decision.consistent && !decision.exhausted, "{decision:?}");
        assert!(
            recorder.recorded_completion().is_some(),
            "a consistent run stopped at a completion, and must record it"
        );

        let kb = blocking_chain_kb();
        let (decision, recorder) =
            decide_recording(&kb, &Assumptions::of_kb(), Budget::for_kb(&kb));
        assert!(decision.consistent && !decision.exhausted, "{decision:?}");
        let completion = recorder
            .recorded_completion()
            .expect("a consistent run records a completion");
        assert!(
            !completion.blocks().is_empty(),
            "`⊤ ⊑ ∃r.⊤` builds a chain that terminates only by blocking, so the completion \
             carries blocking witnesses"
        );
    }

    /// `⊤ ⊑ ∃r.⊤` over one individual: an infinite chain that terminates ONLY by blocking, so
    /// its completion is the one that exercises the blocking-witness recorder.
    fn blocking_chain_kb() -> Kb {
        let mut kb = Kb::empty();
        kb.push_gci(
            Concept::Top,
            Concept::Some(Role::Named(R), Box::new(Concept::Top)),
        );
        kb.individuals.insert(IND);
        kb.finalize();
        kb
    }

    /// `a : A ⊔ B` with `A ⊑ ⊥` and `B ⊑ ⊥`: INCONSISTENT, and it gets there through a two-way
    /// case split both of whose alternatives close on a clause instance — a refutation TREE
    /// rather than a single leaf.
    fn closing_disjunction_kb() -> Kb {
        let mut kb = Kb::empty();
        kb.push_gci(Concept::Named(A), Concept::Bottom);
        kb.push_gci(Concept::Named(B), Concept::Bottom);
        let disjunction = kb
            .table
            .intern(Concept::Or(vec![Concept::Named(A), Concept::Named(B)]));
        kb.abox_types.push((IND, disjunction));
        kb.individuals.insert(IND);
        kb.finalize();
        kb
    }

    /// The Motik–Shearer–Horrocks `NI`-rule's decision is a pure function of the knowledge base,
    /// and the reserved roots it mints are CHARGED to the work meter — deterministic budget
    /// accounting for the nominal-introduction path. The spy-point inconsistency (a nominal
    /// `≤2 p⁻.⊤`, everything `p`-related to it, an individual forced to `≥3 r.⊤`) decides
    /// INCONSISTENT, byte-identically run to run, having spent work on the reserved roots. The
    /// byte-identical `Decision` is this crate's replayable certificate: re-deciding reproduces
    /// the verdict and every cost figure exactly.
    #[test]
    fn nn_ni_spy_point_decision_is_deterministic_and_charged() {
        let kb = spy_point_kb();
        let cap = Budget::for_kb(&kb);
        let first = decide(&kb, &Assumptions::of_kb(), cap);
        let again = decide(&kb, &Assumptions::of_kb(), cap);
        assert_eq!(first, again, "the NI decision is byte-identical run to run");
        assert!(
            !first.consistent && !first.exhausted,
            "the spy point is inconsistent, decided (not exhausted): {first:?}"
        );
        assert!(
            first.work > 0,
            "the reserved-root minting is charged to the work meter: {first:?}"
        );
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
        let branch = h.find_branch(&mut st).expect("two disjunctions are open");
        assert_eq!(
            branch.alternatives.len(),
            3,
            "the three-way disjunction is met first, so it is the branch point: {:?}",
            branch.alternatives
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

        // A first round: nothing was seen before, so every node is affected.
        let affected: Vec<super::Affected> = (0..st.nodes.len())
            .map(|node| super::Affected {
                node,
                full: true,
                via: 0,
            })
            .collect();
        h.round(&mut st, &affected);

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
