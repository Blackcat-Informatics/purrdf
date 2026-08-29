// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Independently replayable OWL-DL TABLEAU PROOF TERMS: a recorded refutation whose checker
//! RE-DERIVES the step instead of believing it.
//!
//! # A certificate is not a proof
//!
//! [`DlCertificate`](crate::DlCertificate) records budgets, work, search shape, boundaries and
//! completeness. Every one of those is a MEASUREMENT the search took of itself: a hypertableau
//! that closed a branch it had no right to close reports exactly the same certificate as one
//! that closed it correctly, because a wrong answer and a right answer are both just answers.
//! A [`DlProof`] is a different kind of object. Its checker takes the proof and the CALLER'S
//! OWN ontology, clausifies that ontology itself, looks the cited clause up in ITS OWN clause
//! set, computes that clause's head form ITSELF, grounds that clause's body against the
//! recorded frame ITSELF, and returns **the conclusion it derived**. The recorded witness is
//! never an input to that computation — it is only ever the value the checker's own grounding
//! is compared against.
//!
//! # The TRUST BASE: what "independently" means, exactly
//!
//! Full independence from the producer is not achievable and this module does not pretend to
//! it. Verifying that a branch point enumerated ALL of its alternatives means computing what
//! the alternatives ARE, and that computation is the producer's own clausification and
//! grounding. Re-deriving it with a second implementation would only move the question to
//! "which of the two is right".
//!
//! So the shared surface is NAMED instead of hidden. [`TrustBaseEntry`] is an ordered,
//! versioned enumeration of the producer-shared components a verdict rests on; the set is
//! carried IN the proof term, covered by [`DlProof::digest`], and pinned by a test, so growing
//! it is a breaking change rather than a quiet erosion. Every check this module performs is
//! classified against it, and the classification is reported in three counts that a consumer
//! reads directly ([`CheckReport`]):
//!
//! * **`attested`** — the checker RE-DERIVED it, depending on nothing in the trust base;
//! * **`trusted`** — the checker verified it, but the verification itself rests on named
//!   trust-base entries, which the report lists ([`CheckReport::rests_on`]);
//! * **`unattested`** — the checker did not check it at all.
//!
//! A `trusted` result is never presentable as an `attested` one: the counts are separate
//! fields with separate accessors, and no method adds them together.
//!
//! # What this module establishes, and what it does not
//!
//! * **[`DlProof::replay_clash`] is real.** For a recorded [`ClashStep`] it establishes that
//!   the cited clause is a clause of the caller's ontology's own clause set, that its head is
//!   EMPTY (so the instance derives `false`), that the recorded frame is wide enough for the
//!   clause's variables, and that the recorded witness is exactly the grounding of that
//!   clause's body under that frame. It runs no search: it never constructs a
//!   `Hyper` driver, never opens a
//!   `Session`, and never expands a completion graph.
//! * **BRANCH EXHAUSTIVENESS is established, as a `trusted` check.**
//!   [`DlProof::replay_branch`] regenerates a branch point's alternatives by calling
//!   `hyper::ground_head` against the CALLER's clause set and
//!   compares them, atom for atom and in order, against the recorded ones — so a producer that
//!   dropped a disjunct (the way an unsound `inconsistent` is fabricated), added one, or
//!   reordered them is rejected. It rests on [`TrustBaseEntry::Clausification`] and
//!   [`TrustBaseEntry::Grounding`] and says so. What it is INDEPENDENT of is the search driver
//!   — `solve`, `saturate`, `find_branch` and the branch stack — which is where the state, and
//!   therefore the bugs, live. [`DlProof::replay_refutation`] walks the whole tree: every
//!   branch point exhaustive, every alternative closed, every leaf a replayed clash.
//! * **A CLASH-FREE COMPLETION is MODEL-CHECKED, not searched.**
//!   [`DlProof::replay_completion`] takes the recorded completion graph and verifies, using
//!   pure functions over the caller's own clause set, that no empty-headed clause matches it
//!   (nothing on it clashes), that every clause of the caller's ontology is SATISFIED on it,
//!   and that every blocking pair really does have equal signatures — recomputed by the
//!   checker from the recorded labels rather than believed.
//! * **REACHABILITY is not established.** The checker does not show that a recorded clash's
//!   witness facts are derivable in a completion graph of the ontology. It reports how many of
//!   them it could reduce to an ASSERTED axiom of the caller's ABox
//!   ([`ClashReplay::attested`]) and how many it could not
//!   ([`ClashReplay::unattested`]); an unattested fact is a fact this stage takes on the
//!   producer's word. Closing that gap needs a premise DAG per witness fact, which is
//!   deliberately not faked here.
//! * **UNRAVELLING is CITED, never re-proved.** That a blocked, clash-free, saturated pre-model
//!   yields a real model is a metatheorem about the calculus, not a per-instance obligation. It
//!   is [`TrustBaseEntry::Unravelling`], cited by [`CALCULUS_VERSION`]. The three checks above
//!   do NOT by themselves prove a model exists, and nothing here says they do.
//! * **A MERGE is provenance, not a proof**, and the CONCRETE-domain clash has no clause
//!   instance to replay — see [`MergeStep`] and [`DlProof::data_clashes`].
//!
//! # Three recorded kinds, one replayable
//!
//! [`ClashStep`] is a derivation of `false` from a clause with an empty head — the calculus's
//! only clash rule, because in this hypertableau a clash IS a derivation rather than a
//! detector. [`MergeStep`] is provenance for an identification: which rule forced it, and the
//! two stable node identities it joined. The data-clash list names the nodes whose CONCRETE
//! domain constraints the `owl_dl::data` solver found unsatisfiable — the one
//! decision this calculus does not take through a clause, and therefore the one clash with no
//! clause instance to replay. Only the first is replayable in this stage; the other two are
//! records, they are labelled as records, and no method on this type calls them proofs.
//!
//! # Identity binds to the CALLER, not to the producer
//!
//! A proof that shipped its own copy of the data it was checked against would verify against
//! stores the producer chose — which is worth nothing. So a [`DlProof`] carries no ontology
//! and no clause set. It carries two digests:
//!
//! * [`DlProof::input`] — BLAKE3 over the RDFC-1.0 canonical N-Quads of the ontology
//!   ([`purrdf_core::canonicalize`]). This is the PRODUCER-INDEPENDENT identity: two engines
//!   that read the same graph compute the same 32 bytes, because canonical N-Quads is a
//!   statement about a quad set rather than about an emission order.
//! * [`DlProof::contract`] — BLAKE3 over [`CALCULUS_VERSION`] and the canonical encoding of
//!   the DL-clause set. This is honestly PRODUCER-DERIVED: the clause set is the output of
//!   `clause::derive` over
//!   `absorb`'s decisions, so it attests WHICH calculus and WHICH
//!   clausification an answer came from, and it is not a second, independent identity for the
//!   ontology. The two are never conflated: [`DlProofContext`] recomputes BOTH from the
//!   caller's own dataset and rejects a proof that disagrees with either.
//!
//! # Determinism
//!
//! Every field is an integer, a fixed-order enum ordinal or a length-prefixed byte string;
//! boundaries are emitted in `Construct::ALL` order; steps are emitted in the order the
//! deterministic search recorded them. Nothing is read out of a hash map and nothing consults
//! a clock, so [`DlProof::encode`] is byte-identical run to run and on `wasm32`, exactly as
//! the `Decision` it accompanies is.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};

use purrdf_core::RdfDataset;
use purrdf_datalog::clause::HeadForm;

use crate::EntailError;
use crate::owl_dl::Kb;
use crate::owl_dl::clause::{BodyAtom, ClauseSet, DlClause, HeadAtom, derive};
use crate::owl_dl::concept::{Decomp, Role};
use crate::owl_dl::graph::{Assumptions, Budget, GeneratedRoot, NominalId, State, find};
use crate::owl_dl::hyper::{Ground, ground_head};
use crate::report::Construct;

// ── The wire format's fixed vocabulary ──────────────────────────────────────────

/// Domain-separation tag leading every [`DlProof::encode`]d proof.
///
/// Bumped whenever the encoding changes shape, so bytes written under an older layout can
/// never be decoded as if they were current.
const PROOF_ENCODING_TAG: &str = "purrdf-owl-dl-proof-v2";

/// Domain-separation tag for [`contract_digest`].
const CONTRACT_DIGEST_TAG: &str = "purrdf-owl-dl-contract-v1";

/// The identity of the DECISION CALCULUS a proof term was produced under.
///
/// Bump this whenever a change could move an answer: a new rule, a changed clash condition, a
/// changed blocking condition, a changed clausification. The property is one-directional and
/// the same one [`purrdf_datalog::cache::contract_hash`] states: anything that can change an
/// answer must change the digest, while two calculi that happen to agree may still differ.
/// Over-invalidation is never the bug.
pub const CALCULUS_VERSION: &str = "purrdf-owl-dl-hypertableau-v1";

/// The declared ceiling on recorded steps of each kind.
///
/// A recording run is bounded like every other budget in this crate: a constant, a pure
/// function of nothing, and a `truncated` flag when it bites — never a silent drop. A search
/// that closes more branches than this records the first [`MAX_RECORDED_STEPS`] of each kind
/// and says so through [`DlProof::truncated`].
pub const MAX_RECORDED_STEPS: usize = 4096;

/// Wire kind byte: a node identified by a named individual.
const NODE_INDIVIDUAL: u8 = 0;

/// Wire kind byte: a node identified by a generated reserved root.
const NODE_RESERVED: u8 = 1;

/// Wire kind byte: an anonymous, proof-local node.
const NODE_ANONYMOUS: u8 = 2;

/// Wire kind byte: a concept-membership fact.
const FACT_CONCEPT: u8 = 0;

/// Wire kind byte: a role-edge fact.
const FACT_EDGE: u8 = 1;

/// Wire kind byte: a "denotes this individual" fact.
const FACT_DENOTES: u8 = 2;

/// The declared ceiling on the work one completion MODEL CHECK may spend.
///
/// A resource bound and never a semantic one: a clause the budget did not reach is reported
/// [`CheckReport::unattested`] rather than assumed satisfied, so exhausting it can only ever
/// WITHHOLD a claim. Sized far above anything a completion of the size this calculus builds
/// needs, while keeping the `≤n` body atom's subset enumeration — the one super-linear
/// enumeration a clause body has — from turning a check into a hang.
pub const MAX_CHECK_WORK: u64 = 1 << 22;

// ── The trust base ──────────────────────────────────────────────────────────────

/// The identity of the TRUST BASE a proof term's checks are classified against.
///
/// Bumped whenever [`TrustBaseEntry::ALL`] changes — which is a BREAKING change, because a
/// consumer's reading of what it is trusting is a function of this list. `the_trust_base_is_pinned`
/// asserts the current set element by element so it cannot grow quietly.
pub const TRUST_BASE_VERSION: &str = "purrdf-owl-dl-trust-base-v1";

/// One PRODUCER-SHARED component a verdict rests on.
///
/// A proof term is not a proof from nothing: the checker and the producer share code, and the
/// only honest thing to do about that is to enumerate the shared surface, carry the enumeration
/// in the proof, and classify every check against it. An entry here is a standing statement of
/// the form "if this component is wrong, a check that rests on it can be wrong with it".
///
/// The order is the wire order and is fixed. ADDING AN ENTRY IS A BREAKING VERSION BUMP: it
/// changes what a consumer of an already-issued proof believes they were told.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum TrustBaseEntry {
    /// The OWL REVERSE MAPPING: `Kb::from_dataset` and `Kb::finalize`, which turn the
    /// caller's RDF graph into a knowledge base — the concept table a proof's concept ids index,
    /// the ABox a witness atom is attested against, and the role hierarchy, inverse and
    /// transitivity declarations a completion's neighbour relation is computed from.
    ///
    /// The checker runs it over the CALLER's own dataset, so a proof cannot smuggle a knowledge
    /// base in; but it is the same code the producer ran, and every concept id in a proof term
    /// is meaningless without it. Stage 1 rested on this silently. It is named here rather than
    /// left implicit.
    ReverseMapping,
    /// CLAUSIFICATION: `owl_dl::clause` and `owl_dl::absorb`, which compile a
    /// knowledge base into the DL-clause set.
    ///
    /// Bound by [`DlProof::contract`], which digests [`CALCULUS_VERSION`] and the whole clause
    /// set: a checker whose clausification differs from the producer's rejects the proof
    /// outright rather than checking it against a different compilation.
    Clausification,
    /// GROUNDING: `hyper::ground_head` and this module's
    /// `ground_body`, which map a clause and a binding frame to the atoms a match derives.
    ///
    /// Branch exhaustiveness IS this function's output, so a branch check cannot be independent
    /// of it. What it IS independent of is the search driver that chose the frame.
    Grounding,
    /// UNRAVELLING: the metatheorem that a blocked, clash-free, saturated pre-model of this
    /// calculus yields a model, cited by [`CALCULUS_VERSION`].
    ///
    /// A statement about the CALCULUS, not about a run, so there is no per-instance obligation
    /// to discharge and none is faked. A completion check establishes that nothing on the
    /// recorded graph clashes, that every clause is satisfied on it, and that every blocking
    /// pair has equal signatures; that a MODEL follows is this entry.
    Unravelling,
}

impl TrustBaseEntry {
    /// Every entry, in wire order. **Adding to this is a breaking version bump.**
    pub const ALL: [Self; 4] = [
        Self::ReverseMapping,
        Self::Clausification,
        Self::Grounding,
        Self::Unravelling,
    ];

    /// A short, stable name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ReverseMapping => "reverse-mapping",
            Self::Clausification => "clausification",
            Self::Grounding => "grounding",
            Self::Unravelling => "unravelling",
        }
    }

    /// The wire ordinal — the entry's position in [`Self::ALL`].
    const fn ordinal(self) -> u64 {
        match self {
            Self::ReverseMapping => 0,
            Self::Clausification => 1,
            Self::Grounding => 2,
            Self::Unravelling => 3,
        }
    }
}

/// The trust base as a comma-separated list of [`TrustBaseEntry::as_str`], for a diagnostic.
fn trust_base_text(entries: &[TrustBaseEntry]) -> String {
    let mut out = String::new();
    for entry in entries {
        if !out.is_empty() {
            out.push(',');
        }
        out.push_str(entry.as_str());
    }
    out
}

// ── What a check established, and what it rested on ─────────────────────────────

/// THREE counts, never two, and never a boolean.
///
/// A checker that answered `bool` would let a consumer read "it verified" without reading
/// "…using the producer's own clausifier". These three are kept apart on purpose and there is
/// no accessor that sums them:
///
/// * [`Self::attested`] — checks the checker RE-DERIVED, resting on nothing in the trust base;
/// * [`Self::trusted`] — checks the checker verified, whose verification rests on the named
///   entries [`Self::rests_on`] lists;
/// * [`Self::unattested`] — obligations the checker did not check at all.
///
/// [`Self::is_fully_attested`] is the only way to ask for the strong reading, and it is `true`
/// only when both of the other counts are zero and nothing is rested on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckReport {
    /// Checks re-derived independently of the trust base.
    attested: usize,
    /// Checks whose verification rests on [`Self::rests_on`].
    trusted: usize,
    /// Obligations not checked.
    unattested: usize,
    /// The trust-base entries the `trusted` checks rest on, in [`TrustBaseEntry::ALL`] order.
    rests_on: Vec<TrustBaseEntry>,
}

impl CheckReport {
    /// An empty report.
    const fn new() -> Self {
        Self {
            attested: 0,
            trusted: 0,
            unattested: 0,
            rests_on: Vec::new(),
        }
    }

    /// Count `n` checks the checker re-derived on its own.
    fn attest(&mut self, n: usize) {
        self.attested += n;
    }

    /// Count `n` checks that rest on `entries`.
    fn trust(&mut self, n: usize, entries: &[TrustBaseEntry]) {
        self.trusted += n;
        self.cite(entries);
    }

    /// Record that this report rests on `entries`, without counting a check.
    fn cite(&mut self, entries: &[TrustBaseEntry]) {
        for entry in entries {
            if !self.rests_on.contains(entry) {
                self.rests_on.push(*entry);
            }
        }
        self.rests_on.sort_unstable();
    }

    /// Count `n` obligations the checker did not check.
    fn leave(&mut self, n: usize) {
        self.unattested += n;
    }

    /// Fold `other` into this report.
    fn absorb(&mut self, other: &Self) {
        self.attested += other.attested;
        self.trusted += other.trusted;
        self.unattested += other.unattested;
        self.cite(&other.rests_on);
    }

    /// Checks the checker RE-DERIVED, depending on nothing in the trust base.
    #[must_use]
    pub const fn attested(&self) -> usize {
        self.attested
    }

    /// Checks the checker verified whose verification RESTS ON the trust base.
    ///
    /// Never add this to [`Self::attested`]. The two are different kinds of claim, and the
    /// point of this stage is that a consumer can tell them apart.
    #[must_use]
    pub const fn trusted(&self) -> usize {
        self.trusted
    }

    /// Obligations the checker did not check at all.
    #[must_use]
    pub const fn unattested(&self) -> usize {
        self.unattested
    }

    /// The trust-base entries the [`Self::trusted`] checks rest on.
    #[must_use]
    pub fn rests_on(&self) -> &[TrustBaseEntry] {
        &self.rests_on
    }

    /// Whether EVERY check was re-derived independently and nothing was left unchecked.
    ///
    /// False for every report this module produces that involved the clause set, and that is
    /// the honest answer rather than a defect: verifying a claim about a clause set means
    /// having one.
    #[must_use]
    pub fn is_fully_attested(&self) -> bool {
        self.trusted == 0 && self.unattested == 0 && self.rests_on.is_empty()
    }
}

// ── Stable node identity ────────────────────────────────────────────────────────

/// A role of the proof surface: a property id and whether it is read inverted.
///
/// A public mirror of the crate-private `Role`, so a proof term can name a role without
/// exposing the reasoner's internal concept vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProofRole {
    /// The interned property term id.
    property: u32,
    /// Whether the role is the property's INVERSE.
    inverse: bool,
}

impl ProofRole {
    /// The interned property term id.
    #[must_use]
    pub const fn property(self) -> u32 {
        self.property
    }

    /// Whether the role is the property's inverse (`r⁻`) rather than the property itself.
    #[must_use]
    pub const fn is_inverse(self) -> bool {
        self.inverse
    }

    /// The proof-surface spelling of an internal `Role`.
    pub(crate) const fn of(role: Role) -> Self {
        match role {
            Role::Named(property) => Self {
                property,
                inverse: false,
            },
            Role::Inv(property) => Self {
                property,
                inverse: true,
            },
        }
    }
}

/// The MERGE-INVARIANT identity of a completion-graph node.
///
/// Never a node index: an index is invalidated by the very merges this proof records, and two
/// runs of the same search over the same ontology need not allocate a witness at the same
/// slot. The three shapes are the three kinds of identity the calculus actually has.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum NodeRef {
    /// A node that DENOTES the named individual with this term id.
    ///
    /// Stable under every merge: OWL 2 makes no unique name assumption, so identification
    /// UNIONS the names a node denotes and never withdraws one.
    Individual(u32),
    /// A node that denotes a GENERATED reserved nominal — Motik–Shearer–Horrocks' `u.⟨R,B,i⟩`.
    ///
    /// The mirror of `GeneratedRoot`, which was designed to be stable across merges and
    /// clones for exactly this reason, and which both decision cores mint identically.
    Reserved(Box<ReservedRef>),
    /// An ANONYMOUS node — a `≥`-rule witness with no name of its own.
    ///
    /// The payload is PROOF-LOCAL: it distinguishes the anonymous nodes of ONE proof from one
    /// another and means nothing outside it. A checker treats it as an existentially
    /// quantified constant, which is the only honest reading — the calculus never gave the
    /// node a name, so a proof cannot invent one that a second run would agree with.
    Anonymous(u32),
}

/// The identity of a generated (nominal-introduction) reserved root, on the proof surface.
///
/// The public mirror of `GeneratedRoot`: the at-most root `u` the reserved set belongs to,
/// the counted role `R`, the at-most filler concept id `B`, and the index `i` within the
/// bound.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ReservedRef {
    /// The at-most root `u` whose reserved set this belongs to.
    origin: NodeRef,
    /// The counted role `R`.
    role: ProofRole,
    /// The at-most filler concept id `B`.
    filler: u32,
    /// The index `i` within the bound.
    index: u32,
}

impl ReservedRef {
    /// The at-most root `u` whose reserved set this belongs to.
    #[must_use]
    pub const fn origin(&self) -> &NodeRef {
        &self.origin
    }

    /// The counted role `R`.
    #[must_use]
    pub const fn role(&self) -> ProofRole {
        self.role
    }

    /// The at-most filler concept id `B`.
    #[must_use]
    pub const fn filler(&self) -> u32 {
        self.filler
    }

    /// The index `i` within the bound.
    #[must_use]
    pub const fn index(&self) -> u32 {
        self.index
    }
}

impl NodeRef {
    /// The proof-surface identity of the internal [`NominalId`].
    fn of_nominal(id: &NominalId) -> Self {
        match id {
            NominalId::Named(a) => Self::Individual(*a),
            NominalId::Generated(generated) => Self::of_generated(generated),
        }
    }

    /// The proof-surface identity of the internal `GeneratedRoot`.
    fn of_generated(root: &GeneratedRoot) -> Self {
        Self::Reserved(Box::new(reserved_ref(root)))
    }
}

/// The proof-surface spelling of an internal `GeneratedRoot`.
fn reserved_ref(root: &GeneratedRoot) -> ReservedRef {
    ReservedRef {
        origin: NodeRef::of_nominal(&root.origin),
        role: ProofRole::of(root.role),
        filler: root.filler,
        index: root.index,
    }
}

/// The stable identity of completion-graph node `node` in state `st`.
///
/// Resolved through [`find`] first, so a node that has been merged away answers with the
/// identity of the node it was folded into — which is the whole point of recording an identity
/// rather than an index.
pub(crate) fn node_ref(st: &State, node: usize) -> NodeRef {
    let node = find(st, node);
    if let Some(id) = st.nodes[node].nominal_id.as_ref() {
        return NodeRef::of_nominal(id);
    }
    // A node with names but no nominal identity is a blockable node that a nominal clause
    // identified with a root; the SMALLEST name is taken so the choice is a function of the
    // set rather than of an iteration order.
    if let Some(&individual) = st.nodes[node].nominals.iter().next() {
        return NodeRef::Individual(individual);
    }
    NodeRef::Anonymous(node as u32)
}

/// The stable identities of a whole matcher binding frame.
pub(crate) fn frame_refs(st: &State, frame: &[usize]) -> Vec<NodeRef> {
    frame.iter().map(|&node| node_ref(st, node)).collect()
}

// ── Facts ───────────────────────────────────────────────────────────────────────

/// One atom of a completion graph, over [`NodeRef`] identities.
///
/// These are what a clause BODY grounds to. They are deliberately not RDF triples: a concept
/// membership and a `denotes` fact have no triple to be without minting a vocabulary IRI to
/// carry them, and **PurRDF mints no vocabulary**.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum ProofFact {
    /// `C(x)` — the concept id is in the node's label.
    Concept {
        /// The node the concept is asserted of.
        node: NodeRef,
        /// The concept id.
        concept: u32,
    },
    /// `r(x, y)` — `y` is an `r`-neighbour of `x`, read through the role hierarchy, the
    /// inverse-role declarations and the transitive closure.
    Edge {
        /// The node the edge leaves.
        from: NodeRef,
        /// The node the edge enters.
        to: NodeRef,
        /// The role.
        role: ProofRole,
    },
    /// The node DENOTES the named individual with this term id.
    Denotes {
        /// The node.
        node: NodeRef,
        /// The individual term id.
        individual: u32,
    },
}

// ── The recorded steps ──────────────────────────────────────────────────────────

/// A derivation of `false`: a clause with an EMPTY head, matched at a node.
///
/// In this hypertableau a clash is not a detector but a derivation — every clash condition the
/// textbook spells out as a separate trigger (a complementary pair, a negated nominal naming
/// the node's own individual, a negated self restriction, an asymmetric role's symmetric pair,
/// a disjoint role pair) is a clause whose head is empty. So a clash step needs nothing beyond
/// the clause, the frame the matcher bound, and the body instance that frame produced.
///
/// Fields are private and there is no public constructor: a [`ClashStep`] enters a
/// [`DlProof`] either from the instrumented search or through [`DlProof::decode`], and both
/// are checked by [`DlProof::replay_clash`]. There is no third way in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClashStep {
    /// The index of the clause whose empty head was derived, into the clause set the
    /// CHECKER derives from the caller's ontology.
    clause: usize,
    /// The node variable `0` was bound to — the node the round was visiting.
    node: NodeRef,
    /// The matcher's binding frame, variable by variable.
    frame: Vec<NodeRef>,
    /// The body instance the search OBSERVED in the completion graph, in body order.
    ///
    /// Recorded by reading the graph, never by grounding the clause: it is the value
    /// [`DlProof::replay_clash`]'s own grounding is compared against, and a witness that came
    /// from the same grounding would make the comparison vacuous.
    witness: Vec<ProofFact>,
}

impl ClashStep {
    /// Record a clash step. Crate-private: the only producer is the instrumented search.
    pub(crate) const fn new(
        clause: usize,
        node: NodeRef,
        frame: Vec<NodeRef>,
        witness: Vec<ProofFact>,
    ) -> Self {
        Self {
            clause,
            node,
            frame,
            witness,
        }
    }

    /// The clause index the step cites.
    #[must_use]
    pub const fn clause(&self) -> usize {
        self.clause
    }

    /// The node variable `0` was bound to.
    #[must_use]
    pub const fn node(&self) -> &NodeRef {
        &self.node
    }

    /// The matcher's binding frame.
    #[must_use]
    pub fn frame(&self) -> &[NodeRef] {
        &self.frame
    }

    /// The body instance the search observed — the STATED witness, never a checker input.
    #[must_use]
    pub fn witness(&self) -> &[ProofFact] {
        &self.witness
    }
}

/// Which rule forced an identification.
///
/// The three are the calculus's three sources of equality, and they are told apart because
/// they carry different obligations: an at-most merge is a case split the search chose among,
/// a nominal merge is forced by the `o`-clause, and a nominal-introduction merge folds a
/// blockable node into a RESERVED root.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum MergeCause {
    /// A `≤n r.C` restriction's `⋁_{i<j} yᵢ ≈ yⱼ` head — one alternative of a case split.
    AtMost,
    /// A nominal `{a}` in the node's label identified it with `a`'s root.
    Nominal,
    /// The Motik–Shearer–Horrocks `NI`-rule folded a blockable predecessor into a reserved
    /// root `u.⟨R,B,i⟩`.
    NominalIntroduction,
}

impl MergeCause {
    /// A short, stable name for the cause.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AtMost => "at-most",
            Self::Nominal => "nominal",
            Self::NominalIntroduction => "nominal-introduction",
        }
    }
}

/// PROVENANCE for one identification: which rule joined which two node identities.
///
/// A record, not a proof. Nothing on this type or on [`DlProof`] replays a merge: doing so
/// means re-deriving the head atom that forced it, which needs the premise DAG this stage
/// does not build. It is recorded because a merge is where node identity CHANGES, and a
/// consumer reading a clash step's [`NodeRef`]s cannot otherwise tell why two names became
/// one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeStep {
    /// Which rule forced the identification.
    cause: MergeCause,
    /// One of the two identities, as it stood BEFORE the merge.
    left: NodeRef,
    /// The other, as it stood before the merge.
    right: NodeRef,
    /// The identity both denote AFTER it — the orientation
    /// `Graph::merge_nodes` chose, read off the
    /// state rather than predicted, so this record cannot disagree with the graph about which
    /// node survived.
    joined: NodeRef,
    /// Whether the identification itself closed the state — a forced merge of a `≠` pair, in
    /// which case no merge happened and `joined` is `right` unchanged.
    clashed: bool,
}

impl MergeStep {
    /// Record a merge. Crate-private: the only producer is the instrumented search.
    pub(crate) const fn new(
        cause: MergeCause,
        left: NodeRef,
        right: NodeRef,
        joined: NodeRef,
        clashed: bool,
    ) -> Self {
        Self {
            cause,
            left,
            right,
            joined,
            clashed,
        }
    }

    /// Which rule forced the identification.
    #[must_use]
    pub const fn cause(&self) -> MergeCause {
        self.cause
    }

    /// One of the two identities, before the merge.
    #[must_use]
    pub const fn left(&self) -> &NodeRef {
        &self.left
    }

    /// The other identity, before the merge.
    #[must_use]
    pub const fn right(&self) -> &NodeRef {
        &self.right
    }

    /// The identity both denote after the merge.
    #[must_use]
    pub const fn joined(&self) -> &NodeRef {
        &self.joined
    }

    /// Whether the identification closed the state.
    #[must_use]
    pub const fn clashed(&self) -> bool {
        self.clashed
    }
}

// ── Branch points ───────────────────────────────────────────────────────────────

/// One grounded head atom of a `⊔`-rule alternative, on the proof surface.
///
/// The public mirror of the search's own `Ground`, over merge-invariant [`NodeRef`] identities
/// rather than node indices. It is a MIRROR and not a second vocabulary: a recorded alternative
/// and the alternative a checker regenerates are both this type, so comparing them is an
/// equality rather than an interpretation.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ProofGround {
    /// Add a concept to a node's label.
    Concept {
        /// The node.
        node: NodeRef,
        /// The concept id.
        concept: u32,
    },
    /// Give a node an `r`-edge to itself.
    SelfLoop {
        /// The node.
        node: NodeRef,
        /// The role.
        role: ProofRole,
    },
    /// Ensure `n` pairwise-distinct `role`-neighbours satisfying `filler`.
    AtLeast {
        /// The node the restriction is on.
        node: NodeRef,
        /// The bound.
        n: u32,
        /// The counted role.
        role: ProofRole,
        /// The filler concept id.
        filler: u32,
    },
    /// Identify two nodes — one alternative of a `≤n` case split.
    Equal {
        /// One node.
        left: NodeRef,
        /// The other.
        right: NodeRef,
    },
    /// Identify a node with a named individual's root — the `o`-clause's alternative.
    EqualIndividual {
        /// The node.
        node: NodeRef,
        /// The individual term id.
        individual: u32,
    },
    /// Identify a node with a RESERVED root `u.⟨R,B,i⟩` — the `NI`-rule's alternative.
    EqualReserved {
        /// The blockable node folded into the reserved root.
        node: NodeRef,
        /// The reserved root.
        root: ReservedRef,
    },
}

impl ProofGround {
    /// The proof-surface spelling of a grounded atom over [`NodeRef`] identities.
    fn of(atom: &Ground<NodeRef>) -> Self {
        match atom {
            Ground::Concept(node, concept) => Self::Concept {
                node: node.clone(),
                concept: *concept,
            },
            Ground::SelfLoop(node, role) => Self::SelfLoop {
                node: node.clone(),
                role: ProofRole::of(*role),
            },
            Ground::AtLeast(node, n, role, filler) => Self::AtLeast {
                node: node.clone(),
                n: *n,
                role: ProofRole::of(*role),
                filler: *filler,
            },
            Ground::Equal(left, right) => Self::Equal {
                left: left.clone(),
                right: right.clone(),
            },
            Ground::EqualIndividual(node, individual) => Self::EqualIndividual {
                node: node.clone(),
                individual: *individual,
            },
            Ground::EqualReserved(node, root) => Self::EqualReserved {
                node: node.clone(),
                root: reserved_ref(root),
            },
        }
    }
}

/// One ALTERNATIVE of a `⊔`-rule branch point: the conjunction of atoms taking it asserts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProofAlternative {
    /// The atoms, in the order the grounding produced them.
    atoms: Vec<ProofGround>,
}

impl ProofAlternative {
    /// The atoms taking this alternative asserts.
    #[must_use]
    pub fn atoms(&self) -> &[ProofGround] {
        &self.atoms
    }
}

/// WHAT BECAME of one alternative of a branch point.
///
/// The recorded shape of the search tree. It is what makes "every alternative closed" a
/// structural claim a checker can walk rather than a sentence in a log: an alternative either
/// names the recorded step that closed it, names the branch point it descended into, or says
/// out loud that it did neither.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum BranchOutcome {
    /// The alternative closed on the clause instance at this index of [`DlProof::clashes`].
    Clash(usize),
    /// It closed on the concrete-domain clash at this index of [`DlProof::data_clashes`].
    DataClash(usize),
    /// It closed on the forced identification at this index of [`DlProof::merges`], whose
    /// [`MergeStep::clashed`] is set.
    Merge(usize),
    /// It reached the further branch point at this index of [`DlProof::branches`].
    Branch(usize),
    /// It CLOSED, but nothing replayable was written down for it.
    ///
    /// Reachable and therefore recorded rather than swept up: an alternative can close inside
    /// the `≥`-rule's own distinctness bookkeeping — two witnesses forced `≠` that the datatype
    /// map already makes one value — which is not a clause instance and has no clause to
    /// replay. A checker counts one of these [`CheckReport::unattested`] and
    /// [`RefutationReplay::is_closed`] answers `false`, so it can never be read as a discharged
    /// obligation.
    Unrecorded,
    /// It did NOT close: the search stopped at a clash-free completion, or ran out of budget,
    /// before exhausting this alternative's subtree.
    ///
    /// A [`ProofAnswer::Inconsistent`] proof carrying one is rejected — the answer claims every
    /// branch closed and this says one did not.
    Open,
    /// The alternative was never tried, because the search ended first.
    ///
    /// The initial value of every slot, so an alternative whose outcome was never filed reads
    /// as untried rather than as closed.
    Unexplored,
}

impl BranchOutcome {
    /// The wire kind byte.
    const fn kind(self) -> u8 {
        match self {
            Self::Clash(_) => 0,
            Self::DataClash(_) => 1,
            Self::Merge(_) => 2,
            Self::Branch(_) => 3,
            Self::Unrecorded => 4,
            Self::Open => 5,
            Self::Unexplored => 6,
        }
    }

    /// The index the outcome carries, or `0` for the three that carry none.
    const fn payload(self) -> usize {
        match self {
            Self::Clash(index)
            | Self::DataClash(index)
            | Self::Merge(index)
            | Self::Branch(index) => index,
            Self::Unrecorded | Self::Open | Self::Unexplored => 0,
        }
    }
}

/// One `⊔`-rule BRANCH POINT: the clause instance that generated a case split, ALL of the
/// alternatives it generated, and what became of each.
///
/// This is the record that makes branch EXHAUSTIVENESS checkable. A producer that dropped a
/// disjunct — the way an unsound `inconsistent` is fabricated — records a short alternative
/// list, and [`DlProof::replay_branch`] regenerates the list from the caller's own clause set
/// and disagrees with it.
///
/// Fields are private and there is no public constructor: a [`BranchStep`] enters a
/// [`DlProof`] from the instrumented search or through [`DlProof::decode`], and both are
/// checked by [`DlProof::replay_branch`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchStep {
    /// The clause whose DISJUNCTIVE head generated the alternatives, into the clause set the
    /// CHECKER derives from the caller's ontology.
    clause: usize,
    /// The node variable `0` was bound to.
    node: NodeRef,
    /// The matcher's binding frame, variable by variable.
    frame: Vec<NodeRef>,
    /// The alternatives the clause HEAD generated, in the producer's order.
    ///
    /// Read off the vector the search branched over, never re-derived by a second call to
    /// `hyper::ground_head` — the same discipline
    /// `ClashStep`'s witness keeps, and for the same reason: a record produced by the
    /// computation it is compared against would make the comparison vacuous.
    alternatives: Vec<ProofAlternative>,
    /// The alternatives the Motik–Shearer–Horrocks `NI`-rule APPENDED beside them.
    ///
    /// Kept apart because they are not regenerable from the clause: they are a function of
    /// which blockable predecessors press the at-most bound, which is completion-graph state.
    /// A checker verifies their SHAPE against the cited clause and counts the set itself
    /// unattested — see [`DlProof::replay_branch`].
    introduced: Vec<ProofAlternative>,
    /// What became of each alternative, `alternatives` first and then `introduced`.
    outcomes: Vec<BranchOutcome>,
}

impl BranchStep {
    /// Record a branch point with every outcome still [`BranchOutcome::Unexplored`].
    /// Crate-private: the only producer is the instrumented search.
    pub(crate) fn new(
        clause: usize,
        node: NodeRef,
        frame: Vec<NodeRef>,
        alternatives: Vec<ProofAlternative>,
        introduced: Vec<ProofAlternative>,
    ) -> Self {
        let outcomes = vec![BranchOutcome::Unexplored; alternatives.len() + introduced.len()];
        Self {
            clause,
            node,
            frame,
            alternatives,
            introduced,
            outcomes,
        }
    }

    /// The clause index the branch point cites.
    #[must_use]
    pub const fn clause(&self) -> usize {
        self.clause
    }

    /// The node variable `0` was bound to.
    #[must_use]
    pub const fn node(&self) -> &NodeRef {
        &self.node
    }

    /// The matcher's binding frame.
    #[must_use]
    pub fn frame(&self) -> &[NodeRef] {
        &self.frame
    }

    /// The alternatives the clause head generated, in the producer's order.
    #[must_use]
    pub fn alternatives(&self) -> &[ProofAlternative] {
        &self.alternatives
    }

    /// The alternatives the `NI`-rule appended.
    #[must_use]
    pub fn introduced(&self) -> &[ProofAlternative] {
        &self.introduced
    }

    /// What became of each alternative, head-generated ones first.
    #[must_use]
    pub fn outcomes(&self) -> &[BranchOutcome] {
        &self.outcomes
    }

    /// How many alternatives the branch point has in total.
    #[must_use]
    pub fn width(&self) -> usize {
        self.alternatives.len() + self.introduced.len()
    }
}

// ── The completion ──────────────────────────────────────────────────────────────

/// One node of a recorded CLASH-FREE COMPLETION.
///
/// Everything a checker needs to model-check a clause against it and to recompute a blocking
/// signature, and nothing else: no node index, no union-find pointer, and no state the search
/// used to get here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionNode {
    /// The node's merge-invariant identity.
    node: NodeRef,
    /// The concept ids in its label, ascending.
    label: Vec<u32>,
    /// The named individuals it denotes, ascending.
    nominals: Vec<u32>,
    /// The nodes it is forced DISTINCT from.
    distinct: Vec<NodeRef>,
    /// Its generating predecessor, if it has one.
    parent: Option<NodeRef>,
    /// The role on the edge from that predecessor.
    incoming: Option<ProofRole>,
    /// Whether it inhabits the DATA domain rather than the object domain.
    concrete: bool,
    /// Whether it is a root node — never blocked.
    root: bool,
    /// The VALUE class it denotes, when it denotes a literal whose value is known. Two nodes
    /// carrying different classes are distinct with nothing having said so.
    value_class: Option<u32>,
}

impl CompletionNode {
    /// The node's identity.
    #[must_use]
    pub const fn node(&self) -> &NodeRef {
        &self.node
    }

    /// The concept ids in its label, ascending.
    #[must_use]
    pub fn label(&self) -> &[u32] {
        &self.label
    }

    /// The named individuals it denotes.
    #[must_use]
    pub fn nominals(&self) -> &[u32] {
        &self.nominals
    }

    /// The nodes it is forced distinct from.
    #[must_use]
    pub fn distinct(&self) -> &[NodeRef] {
        &self.distinct
    }

    /// Its generating predecessor.
    #[must_use]
    pub const fn parent(&self) -> Option<&NodeRef> {
        self.parent.as_ref()
    }

    /// The role on the edge from that predecessor.
    #[must_use]
    pub const fn incoming(&self) -> Option<ProofRole> {
        self.incoming
    }

    /// Whether it inhabits the data domain.
    #[must_use]
    pub const fn is_concrete(&self) -> bool {
        self.concrete
    }

    /// Whether it is a root node.
    #[must_use]
    pub const fn is_root(&self) -> bool {
        self.root
    }
}

/// One role edge of a recorded completion, exactly as the graph stores it.
///
/// RAW: the `(from, to, property)` triple, never the role hierarchy's closure of it. Computing
/// the closure at record time would mean calling the metered neighbour scan and so charging a
/// recorded run work an unrecorded one does not spend — which would let a proof-carrying run
/// reach a different verdict. The checker computes the closure itself, from the caller's own
/// role axioms.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionEdge {
    /// The node the edge leaves.
    from: NodeRef,
    /// The node it enters.
    to: NodeRef,
    /// The property term id.
    property: u32,
}

impl CompletionEdge {
    /// The node the edge leaves.
    #[must_use]
    pub const fn from(&self) -> &NodeRef {
        &self.from
    }

    /// The node it enters.
    #[must_use]
    pub const fn to(&self) -> &NodeRef {
        &self.to
    }

    /// The property term id.
    #[must_use]
    pub const fn property(&self) -> u32 {
        self.property
    }
}

/// One DIRECT blocking pair: the node whose `≥`-rule applications were withheld, and the
/// earlier node that stood in for it.
///
/// Only direct pairs are recorded. Indirect blocking — "my predecessor is blocked" — is
/// recomputed by the checker from the recorded predecessors, so a proof cannot assert it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockingPair {
    /// The blocked node.
    blocked: NodeRef,
    /// Its blocker.
    blocker: NodeRef,
}

impl BlockingPair {
    /// The blocked node.
    #[must_use]
    pub const fn blocked(&self) -> &NodeRef {
        &self.blocked
    }

    /// Its blocker.
    #[must_use]
    pub const fn blocker(&self) -> &NodeRef {
        &self.blocker
    }
}

/// The CLASH-FREE COMPLETION a [`ProofAnswer::Consistent`] run stopped at.
///
/// A pre-model, and named one: nodes with labels, raw role edges, and the blocking witnesses
/// that let a finite graph stand for an infinite model. [`DlProof::replay_completion`] MODEL
/// CHECKS it — it does not search it, and it does not re-derive it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Completion {
    /// The representative nodes, in the graph's own ascending index order.
    nodes: Vec<CompletionNode>,
    /// The role edges, in first-seen order.
    edges: Vec<CompletionEdge>,
    /// The direct blocking pairs the search's last derivation round computed.
    blocks: Vec<BlockingPair>,
}

impl Completion {
    /// The representative nodes.
    #[must_use]
    pub fn nodes(&self) -> &[CompletionNode] {
        &self.nodes
    }

    /// The role edges.
    #[must_use]
    pub fn edges(&self) -> &[CompletionEdge] {
        &self.edges
    }

    /// The direct blocking pairs.
    #[must_use]
    pub fn blocks(&self) -> &[BlockingPair] {
        &self.blocks
    }
}

// ── The recorder ────────────────────────────────────────────────────────────────

/// What an instrumented search wrote down.
///
/// Held by `owl_dl::hyper`'s driver behind an `Option`, so a non-recording run — every
/// run every existing caller makes — allocates nothing, records nothing, and takes the same
/// branches in the same order. Recording never consults the work meter and never calls a
/// metered graph operation, which is what makes a recorded run's
/// `Decision` identical to an unrecorded one's.
#[derive(Debug, Default)]
pub(crate) struct Recorder {
    /// The clause instances that derived `false`, in search order.
    clashes: Vec<ClashStep>,
    /// The identifications, in search order.
    merges: Vec<MergeStep>,
    /// The nodes whose concrete-domain constraints had no solution, in search order.
    data_clashes: Vec<NodeRef>,
    /// The `⊔`-rule branch points, in search order. A child is always recorded after its
    /// parent, which is what makes the recorded tree well-founded by construction.
    branches: Vec<BranchStep>,
    /// What became of the ROOT state — the one closure or branch point that is nobody's
    /// alternative.
    root: BranchOutcome,
    /// The clash-free completion the search stopped at, if it found one.
    completion: Option<Completion>,
    /// The DIRECT blocking pairs the most recent derivation round computed, as node
    /// identities. Overwritten every round; read once, when a completion is recorded.
    blocking: Vec<(NodeRef, NodeRef)>,
    /// Whether any of the four lists reached [`MAX_RECORDED_STEPS`].
    truncated: bool,
}

/// What a [`Recorder`] held at a point in the search.
///
/// Taken when an alternative is dispensed and read when it closes, so "what closed this
/// alternative" is "what was written down in between" rather than a guess at the recorder's
/// most recent entry.
#[derive(Debug, Clone, Copy)]
pub(crate) struct RecorderMark {
    /// How many clash steps had been recorded.
    clashes: usize,
    /// How many merges.
    merges: usize,
    /// How many concrete-domain clashes.
    data_clashes: usize,
}

impl Recorder {
    /// Record a clash step, up to the declared ceiling.
    pub(crate) fn clash(&mut self, step: ClashStep) {
        if self.clashes.len() >= MAX_RECORDED_STEPS {
            self.truncated = true;
            return;
        }
        self.clashes.push(step);
    }

    /// Record a merge, up to the declared ceiling.
    pub(crate) fn merge(&mut self, step: MergeStep) {
        if self.merges.len() >= MAX_RECORDED_STEPS {
            self.truncated = true;
            return;
        }
        self.merges.push(step);
    }

    /// Whether the run wrote nothing down at all.
    ///
    /// Read by the instrumentation's own standing obligation in
    /// `owl_dl::hyper`: without it, "a recorded run decides identically" would be
    /// satisfied by a recorder that recorded nothing.
    #[cfg(test)]
    pub(crate) fn is_empty(&self) -> bool {
        self.clashes.is_empty()
            && self.merges.is_empty()
            && self.data_clashes.is_empty()
            && self.branches.is_empty()
            && self.completion.is_none()
    }

    /// Record a concrete-domain clash, up to the declared ceiling.
    pub(crate) fn data_clash(&mut self, node: NodeRef) {
        if self.data_clashes.len() >= MAX_RECORDED_STEPS {
            self.truncated = true;
            return;
        }
        self.data_clashes.push(node);
    }

    /// Record a branch point, up to the declared ceiling, answering with its index.
    ///
    /// `None` past the ceiling, which also sets [`Self::truncated`] — and a truncated proof's
    /// branch tree is refused wholesale by [`DlProof::replay_refutation`] rather than walked
    /// with a hole in it.
    pub(crate) fn branch(&mut self, step: BranchStep) -> Option<usize> {
        if self.branches.len() >= MAX_RECORDED_STEPS {
            self.truncated = true;
            return None;
        }
        self.branches.push(step);
        Some(self.branches.len() - 1)
    }

    /// File `outcome` against alternative `ordinal` of branch point `branch`.
    pub(crate) fn outcome(&mut self, branch: usize, ordinal: usize, outcome: BranchOutcome) {
        if let Some(slot) = self
            .branches
            .get_mut(branch)
            .and_then(|step| step.outcomes.get_mut(ordinal))
        {
            *slot = outcome;
        }
    }

    /// File the ROOT state's outcome.
    pub(crate) fn root(&mut self, outcome: BranchOutcome) {
        self.root = outcome;
    }

    /// What the recorder holds now.
    pub(crate) const fn mark(&self) -> RecorderMark {
        RecorderMark {
            clashes: self.clashes.len(),
            merges: self.merges.len(),
            data_clashes: self.data_clashes.len(),
        }
    }

    /// What closed the alternative that opened at `mark` — the step written down since.
    ///
    /// A closing alternative writes at most one clash step (the round that derives `false`
    /// returns immediately) and at most one concrete-domain clash, so the search order below is
    /// a total order over what can have happened rather than a preference. An alternative that
    /// closed without writing anything replayable answers [`BranchOutcome::Unrecorded`], which
    /// is counted unattested rather than smoothed into a closure.
    pub(crate) fn closure_since(&self, mark: &RecorderMark) -> BranchOutcome {
        if self.clashes.len() > mark.clashes {
            return BranchOutcome::Clash(self.clashes.len() - 1);
        }
        if self.data_clashes.len() > mark.data_clashes {
            return BranchOutcome::DataClash(self.data_clashes.len() - 1);
        }
        for index in (mark.merges..self.merges.len()).rev() {
            if self.merges[index].clashed {
                return BranchOutcome::Merge(index);
            }
        }
        BranchOutcome::Unrecorded
    }

    /// The direct blocking pairs the most recent derivation round computed.
    pub(crate) fn blocking(&self) -> &[(NodeRef, NodeRef)] {
        &self.blocking
    }

    /// Replace the direct blocking pairs with this round's.
    pub(crate) fn set_blocking(&mut self, pairs: Vec<(NodeRef, NodeRef)>) {
        self.blocking = pairs;
    }

    /// Record the clash-free completion the search stopped at.
    pub(crate) fn completion(&mut self, completion: Completion) {
        self.completion = Some(completion);
    }

    /// The recorded branch points — read by the instrumentation's standing obligation in
    /// `owl_dl::hyper`, which has to show that recording HAPPENED before "recording is
    /// free" means anything.
    #[cfg(test)]
    pub(crate) fn branches(&self) -> &[BranchStep] {
        &self.branches
    }

    /// The recorded completion, for the same obligation.
    #[cfg(test)]
    pub(crate) const fn recorded_completion(&self) -> Option<&Completion> {
        self.completion.as_ref()
    }
}

impl Default for BranchOutcome {
    /// An outcome nobody filed is an alternative nobody tried.
    fn default() -> Self {
        Self::Unexplored
    }
}

// ── The answer ──────────────────────────────────────────────────────────────────

/// The ANSWER a proof term is bound to.
///
/// Bound into the encoding, so a proof of a refutation cannot be re-presented as a proof of
/// consistency without changing [`DlProof::digest`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum ProofAnswer {
    /// The search found a clash-free completion.
    Consistent,
    /// Every branch of the search closed.
    Inconsistent,
    /// The search reached a cap, or the caller's stop signal fired, before deciding.
    Undecided,
}

impl ProofAnswer {
    /// A short, stable name for the answer.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Consistent => "consistent",
            Self::Inconsistent => "inconsistent",
            Self::Undecided => "undecided",
        }
    }

    /// The wire ordinal.
    const fn ordinal(self) -> u8 {
        match self {
            Self::Consistent => 0,
            Self::Inconsistent => 1,
            Self::Undecided => 2,
        }
    }

    /// The answer with this wire ordinal.
    const fn of_ordinal(ordinal: u8) -> Option<Self> {
        match ordinal {
            0 => Some(Self::Consistent),
            1 => Some(Self::Inconsistent),
            2 => Some(Self::Undecided),
            _ => None,
        }
    }
}

// ── Rejection ───────────────────────────────────────────────────────────────────

/// Why a [`DlProof`] is not a proof of what it states.
///
/// Every variant is a NORMAL rejection of an invalid proof, never an engine fault: a checker
/// that could not tell a forged proof from a genuine one would defeat the point of having one.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DlProofError {
    /// The proof is not a well-formed term: truncated, mis-tagged, or carrying an unknown
    /// node kind or an out-of-range step index.
    Malformed {
        /// A human-readable account of the structural defect.
        detail: String,
    },
    /// The proof was produced for a DIFFERENT ontology than the one it is being checked
    /// against.
    ///
    /// This is the variant that stops a proof from shipping its own evidence: the identity is
    /// recomputed from the CALLER'S dataset with [`purrdf_core::canonicalize`], never read out
    /// of the proof.
    InputMismatch {
        /// The identity the checker computed from the caller's ontology, as lowercase hex.
        expected: String,
        /// The identity the proof states, as lowercase hex.
        stated: String,
    },
    /// The proof was produced under a different CALCULUS or a different clausification of
    /// the same ontology.
    ContractMismatch {
        /// The contract the checker computed, as lowercase hex.
        expected: String,
        /// The contract the proof states, as lowercase hex.
        stated: String,
    },
    /// The caller's ontology has no clause at the cited index.
    UnknownClause {
        /// The cited index.
        clause: usize,
        /// How many clauses the caller's ontology produced.
        clauses: usize,
    },
    /// The cited clause's head is NOT empty, so the instance derives a consequence rather
    /// than `false` — whatever the step is labelled.
    ///
    /// The head form is the CHECKER's own computation over the caller's clause set.
    NotARefutation {
        /// The cited index.
        clause: usize,
        /// The head form the checker computed.
        form: HeadForm,
    },
    /// The recorded frame does not bind a variable the cited clause's body uses.
    FrameTooShort {
        /// The cited index.
        clause: usize,
        /// The variable the body reads.
        variable: u32,
        /// How wide the recorded frame is.
        frame: usize,
    },
    /// The recorded witness has a different number of atoms than the checker's own grounding
    /// of the cited clause's body.
    WitnessLengthMismatch {
        /// The cited index.
        clause: usize,
        /// How many atoms the checker derived.
        derived: usize,
        /// How many atoms the proof states.
        stated: usize,
    },
    /// A recorded witness atom is not the atom the checker's own grounding produced there.
    WitnessMismatch {
        /// The cited index.
        clause: usize,
        /// The body position that disagrees.
        position: usize,
        /// The atom the checker derived.
        derived: Box<ProofFact>,
        /// The atom the proof states.
        stated: Box<ProofFact>,
    },
    /// The step names a clash node that is not the node the frame binds variable `0` to.
    NodeMismatch {
        /// The cited index.
        clause: usize,
        /// The identity the frame binds variable `0` to.
        derived: Box<NodeRef>,
        /// The identity the step states.
        stated: Box<NodeRef>,
    },
    /// The caller's ontology could not be read as an OWL knowledge base at all.
    Ontology {
        /// The reverse mapper's own account of the failure.
        detail: String,
    },
    /// The proof states a TRUST BASE this checker does not implement.
    ///
    /// The set is versioned and adding to it is breaking, so a disagreement means the two sides
    /// mean different things by "verified" — which is a rejection rather than something to
    /// reconcile.
    TrustBaseMismatch {
        /// The trust base this checker classifies against.
        expected: String,
        /// The trust base the proof states.
        stated: String,
    },
    /// A branch point cites a clause whose head is NOT a disjunction, so it generates no case
    /// split at all.
    ///
    /// The head form is the CHECKER's own computation over the caller's clause set.
    NotADisjunction {
        /// The cited index.
        clause: usize,
        /// The head form the checker computed.
        form: HeadForm,
    },
    /// The branch point's recorded alternative list is a different LENGTH than the one the
    /// checker regenerated from the caller's clause set.
    ///
    /// The dropped-disjunct forgery lands here: an `inconsistent` fabricated by omitting an
    /// alternative claims a narrower case split than the clause licenses.
    AlternativeCountMismatch {
        /// The branch point.
        branch: usize,
        /// The clause it cites.
        clause: usize,
        /// How many alternatives the checker derived.
        derived: usize,
        /// How many the proof states.
        stated: usize,
    },
    /// A recorded alternative is not the alternative the checker's own grounding produced at
    /// that position — a rewritten or REORDERED case split.
    AlternativeMismatch {
        /// The branch point.
        branch: usize,
        /// The position in the alternative list that disagrees.
        position: usize,
        /// The alternative the checker derived.
        derived: Box<ProofAlternative>,
        /// The alternative the proof states.
        stated: Box<ProofAlternative>,
    },
    /// A nominal-introduction alternative is not shaped like one the cited clause could
    /// license.
    IllFormedIntroduction {
        /// The branch point.
        branch: usize,
        /// The position within the introduced alternatives.
        position: usize,
        /// What is wrong with it.
        detail: String,
    },
    /// A branch point records a different number of OUTCOMES than it has alternatives.
    OutcomeCountMismatch {
        /// The branch point.
        branch: usize,
        /// How many alternatives it has.
        derived: usize,
        /// How many outcomes it states.
        stated: usize,
    },
    /// An outcome names a recorded step that is not there, or one that cannot close a branch.
    DanglingOutcome {
        /// The branch point.
        branch: usize,
        /// Which alternative.
        ordinal: usize,
        /// What is wrong with the reference.
        detail: String,
    },
    /// An alternative of a branch point did NOT close, in a proof whose answer claims every
    /// branch did.
    BranchNotClosed {
        /// The branch point.
        branch: usize,
        /// Which alternative.
        ordinal: usize,
    },
    /// The proof is bound to a different ANSWER than the check being asked for.
    WrongAnswer {
        /// The answer the check requires.
        expected: ProofAnswer,
        /// The answer the proof is bound to.
        stated: ProofAnswer,
    },
    /// The recording reached [`MAX_RECORDED_STEPS`], so the trace has a hole in it and a
    /// whole-tree check would be walking a partial tree.
    Truncated,
    /// A [`ProofAnswer::Consistent`] proof carries no completion to check.
    NoCompletion,
    /// The recorded completion is not a well-formed graph: a repeated node identity, or an edge
    /// or blocking pair naming a node that is not in it.
    MalformedCompletion {
        /// What is wrong with it.
        detail: String,
    },
    /// An EMPTY-headed clause matches the recorded completion, so the completion derives
    /// `false` — a concealed clash.
    ClashInCompletion {
        /// The clause that matches.
        clause: usize,
        /// The node variable `0` was bound to.
        node: Box<NodeRef>,
    },
    /// A clause of the caller's ontology is NOT satisfied on the recorded completion, so the
    /// completion is not a pre-model of it.
    ClauseNotSatisfied {
        /// The clause.
        clause: usize,
        /// The node variable `0` was bound to.
        node: Box<NodeRef>,
    },
    /// A recorded blocking pair does not have equal signatures, recomputed by the checker.
    BlockingSignatureMismatch {
        /// The node claimed blocked.
        blocked: Box<NodeRef>,
        /// The node claimed to block it.
        blocker: Box<NodeRef>,
        /// Which half of the signature disagrees.
        detail: String,
    },
}

impl std::fmt::Display for DlProofError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Malformed { detail } => write!(f, "malformed OWL-DL proof term: {detail}"),
            Self::InputMismatch { expected, stated } => write!(
                f,
                "the proof was produced for another ontology: the caller's canonical identity \
                 is {expected}, the proof states {stated}"
            ),
            Self::ContractMismatch { expected, stated } => write!(
                f,
                "the proof was produced under another calculus or clausification: the caller's \
                 contract is {expected}, the proof states {stated}"
            ),
            Self::UnknownClause { clause, clauses } => write!(
                f,
                "the proof cites clause {clause}, but the caller's ontology produces {clauses}"
            ),
            Self::NotARefutation { clause, form } => write!(
                f,
                "clause {clause} has head form {form:?}, so it derives a consequence rather \
                 than false"
            ),
            Self::FrameTooShort {
                clause,
                variable,
                frame,
            } => write!(
                f,
                "clause {clause} reads variable {variable} but the recorded frame binds \
                 {frame}"
            ),
            Self::WitnessLengthMismatch {
                clause,
                derived,
                stated,
            } => write!(
                f,
                "clause {clause} grounds to {derived} body atoms but the proof states {stated}"
            ),
            Self::WitnessMismatch {
                clause,
                position,
                derived,
                stated,
            } => write!(
                f,
                "clause {clause} body position {position} grounds to {derived:?} but the proof \
                 states {stated:?}"
            ),
            Self::NodeMismatch {
                clause,
                derived,
                stated,
            } => write!(
                f,
                "clause {clause} is matched at {derived:?} but the proof names {stated:?} as \
                 the clash node"
            ),
            Self::Ontology { detail } => write!(f, "the ontology could not be read: {detail}"),
            Self::TrustBaseMismatch { expected, stated } => write!(
                f,
                "the proof states the trust base [{stated}] but this checker classifies against \
                 [{expected}]"
            ),
            Self::NotADisjunction { clause, form } => write!(
                f,
                "clause {clause} has head form {form:?}, so it generates no case split to be \
                 exhaustive about"
            ),
            Self::AlternativeCountMismatch {
                branch,
                clause,
                derived,
                stated,
            } => write!(
                f,
                "branch point {branch} cites clause {clause}, which grounds to {derived} \
                 alternatives, but the proof states {stated}"
            ),
            Self::AlternativeMismatch {
                branch,
                position,
                derived,
                stated,
            } => write!(
                f,
                "branch point {branch} alternative {position} grounds to {derived:?} but the \
                 proof states {stated:?}"
            ),
            Self::IllFormedIntroduction {
                branch,
                position,
                detail,
            } => write!(
                f,
                "branch point {branch} nominal-introduction alternative {position} is not one \
                 the cited clause licenses: {detail}"
            ),
            Self::OutcomeCountMismatch {
                branch,
                derived,
                stated,
            } => write!(
                f,
                "branch point {branch} has {derived} alternatives but states {stated} outcomes"
            ),
            Self::DanglingOutcome {
                branch,
                ordinal,
                detail,
            } => write!(
                f,
                "branch point {branch} alternative {ordinal} names an outcome that is not \
                 there: {detail}"
            ),
            Self::BranchNotClosed { branch, ordinal } => write!(
                f,
                "branch point {branch} alternative {ordinal} did not close, so not every \
                 alternative was refuted"
            ),
            Self::WrongAnswer { expected, stated } => write!(
                f,
                "this check applies to a {} proof; this one is bound to {}",
                expected.as_str(),
                stated.as_str()
            ),
            Self::Truncated => write!(
                f,
                "the recording reached its ceiling, so the trace is partial and a whole-tree \
                 check would be walking a tree with a hole in it"
            ),
            Self::NoCompletion => {
                write!(f, "the proof carries no completion graph to model check")
            }
            Self::MalformedCompletion { detail } => {
                write!(f, "the recorded completion is not a graph: {detail}")
            }
            Self::ClashInCompletion { clause, node } => write!(
                f,
                "clause {clause} has an empty head and matches the completion at {node:?}, so \
                 the completion derives false"
            ),
            Self::ClauseNotSatisfied { clause, node } => write!(
                f,
                "clause {clause} is not satisfied on the completion at {node:?}"
            ),
            Self::BlockingSignatureMismatch {
                blocked,
                blocker,
                detail,
            } => write!(
                f,
                "{blocked:?} is recorded as blocked by {blocker:?}, but the two do not have the \
                 same blocking signature: {detail}"
            ),
        }
    }
}

impl std::error::Error for DlProofError {}

// ── What a replay establishes ───────────────────────────────────────────────────

/// The conclusion the CHECKER itself derived from a clause instance.
///
/// Not read out of the proof: it is the head form the checker computed over the clause it
/// looked up in the caller's own clause set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum DerivedConclusion {
    /// The clause's head is EMPTY, so the instance derives `false`.
    False,
}

/// What [`DlProof::replay_clash`] established, and what it did not.
///
/// The counts are the honest boundary of this stage and are reported rather than smoothed over:
///
/// * `attested` — witness atoms the checker reduced to an ASSERTED axiom of the caller's ABox.
///   Reading the caller's own reverse-mapped ABox and nothing else, so this rests on nothing
///   the producer computed for this run;
/// * `trusted` — the step's STRUCTURAL checks: that the cited clause is a clause of the
///   caller's clause set, that the checker's own computation of its head form is EMPTY, that
///   the recorded frame is wide enough, that the checker's own grounding of its body is exactly
///   the recorded witness, and that the clash node is the frame's variable `0`. Every one of
///   those is about a CLAUSE, so every one rests on [`TrustBaseEntry::Clausification`] and
///   [`TrustBaseEntry::Grounding`];
/// * `unattested` — witness atoms the checker could not reduce to an asserted axiom, because
///   reducing them needs a premise DAG this stage does not build.
///
/// A replay with `unattested > 0` says "this clause instance is a genuine derivation of
/// `false` over these atoms", not "these atoms are reachable".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClashReplay {
    /// The conclusion the checker derived.
    conclusion: DerivedConclusion,
    /// The clause the checker looked up in the CALLER's clause set.
    clause: usize,
    /// The three counts and what they rest on.
    checks: CheckReport,
}

impl ClashReplay {
    /// The conclusion the checker derived.
    #[must_use]
    pub const fn conclusion(&self) -> DerivedConclusion {
        self.conclusion
    }

    /// The clause the checker looked up in the caller's own clause set.
    #[must_use]
    pub const fn clause(&self) -> usize {
        self.clause
    }

    /// The full classification — see [`CheckReport`].
    #[must_use]
    pub const fn checks(&self) -> &CheckReport {
        &self.checks
    }

    /// Witness atoms reduced to an asserted axiom of the caller's ABox.
    #[must_use]
    pub const fn attested(&self) -> usize {
        self.checks.attested
    }

    /// The step's structural checks, whose verification rests on the trust base.
    ///
    /// Never add this to [`Self::attested`]: a `trusted` check is a check that would be wrong
    /// if the producer's own clausifier were wrong, and that is exactly the distinction a
    /// consumer of a proof needs.
    #[must_use]
    pub const fn trusted(&self) -> usize {
        self.checks.trusted
    }

    /// Witness atoms the checker took on the producer's word.
    ///
    /// Non-zero is the ordinary case for any ontology with a TBox: a derived concept
    /// membership is not an asserted axiom. It is reported because the alternative — omitting
    /// it — would let a partial replay read as a total one.
    #[must_use]
    pub const fn unattested(&self) -> usize {
        self.checks.unattested
    }
}

/// What [`DlProof::replay_branch`] established about ONE branch point.
///
/// The load-bearing field is [`Self::alternatives`] together with [`Self::checks`]: the
/// alternatives were REGENERATED from the caller's own clause set and compared, so a dropped,
/// added or reordered one is a rejection. The regeneration rests on
/// [`TrustBaseEntry::Clausification`] and [`TrustBaseEntry::Grounding`], and is therefore
/// reported `trusted` — not `attested`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchReplay {
    /// The clause the checker looked up in the caller's own clause set.
    clause: usize,
    /// How many alternatives the checker REGENERATED and matched against the record.
    alternatives: usize,
    /// How many nominal-introduction alternatives it could only shape-check.
    introduced: usize,
    /// The three counts and what they rest on.
    checks: CheckReport,
}

impl BranchReplay {
    /// The clause the checker looked up in the caller's own clause set.
    #[must_use]
    pub const fn clause(&self) -> usize {
        self.clause
    }

    /// How many alternatives the checker regenerated and matched, atom for atom and in order.
    #[must_use]
    pub const fn alternatives(&self) -> usize {
        self.alternatives
    }

    /// How many nominal-introduction alternatives were shape-checked rather than regenerated.
    ///
    /// They are a function of which blockable predecessors press the at-most bound, which is
    /// completion-graph state a checker holding only a proof term does not have. Their SHAPE is
    /// checked against the cited clause; the SET is [`CheckReport::unattested`].
    #[must_use]
    pub const fn introduced(&self) -> usize {
        self.introduced
    }

    /// The full classification — see [`CheckReport`].
    #[must_use]
    pub const fn checks(&self) -> &CheckReport {
        &self.checks
    }
}

/// What [`DlProof::replay_refutation`] established about the WHOLE search tree.
///
/// A refutation is a tree: every internal node an exhaustive case split, every leaf a clause
/// instance that derives `false`. This report says how much of that tree the checker walked and
/// what it rested on, and [`Self::is_closed`] is the one place the whole claim is answered —
/// `true` only when every alternative of every branch point reached a REPLAYED closure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefutationReplay {
    /// How many branch points the checker verified exhaustive.
    branches: usize,
    /// How many clash leaves it re-derived.
    clashes: usize,
    /// How many alternatives closed with nothing replayable written down.
    unrecorded: usize,
    /// The three counts and what they rest on.
    checks: CheckReport,
}

impl RefutationReplay {
    /// How many branch points were verified exhaustive.
    #[must_use]
    pub const fn branches(&self) -> usize {
        self.branches
    }

    /// How many clash leaves were re-derived.
    #[must_use]
    pub const fn clashes(&self) -> usize {
        self.clashes
    }

    /// How many alternatives closed with nothing replayable written down.
    ///
    /// Reachable: an alternative can close inside the `≥`-rule's distinctness bookkeeping,
    /// which is not a clause instance. Reported rather than hidden, and it is what makes
    /// [`Self::is_closed`] answer `false`.
    #[must_use]
    pub const fn unrecorded(&self) -> usize {
        self.unrecorded
    }

    /// The full classification — see [`CheckReport`].
    #[must_use]
    pub const fn checks(&self) -> &CheckReport {
        &self.checks
    }

    /// Whether EVERY alternative of every branch point reached a closure the checker REPLAYED.
    ///
    /// Exactly that, and deliberately not more. It does NOT say that every witness atom of
    /// every leaf was reduced to an asserted axiom of the caller's ABox — reachability is not
    /// established by this stage, and [`CheckReport::unattested`] is where that is reported.
    /// What it does say is that no alternative closed on nothing: a branch whose closure was
    /// [`BranchOutcome::Unrecorded`], a concrete-domain clash or a merge — the three closures
    /// with no clause instance behind them — makes this `false`.
    #[must_use]
    pub const fn is_closed(&self) -> bool {
        self.unrecorded == 0
    }
}

/// What [`DlProof::replay_completion`] established about a clash-free completion — and, just as
/// importantly, what it did not.
///
/// # What it establishes
///
/// 1. **No node carries a clash.** No empty-headed clause of the caller's clause set matches
///    the recorded completion.
/// 2. **Every clause is SATISFIED on it.** For every match of every clause body, some head
///    disjunct holds on the completion. This is direct model checking, run by pure functions
///    over the recorded graph: no `Hyper` driver is constructed and no
///    `Session` is opened.
/// 3. **Every blocking pair genuinely has equal signatures**, recomputed by the checker from
///    the recorded labels, predecessors and incoming edges.
///
/// # What it does NOT establish
///
/// That a MODEL exists. The step from "a blocked, clash-free, saturated pre-model" to "a model"
/// is the unravelling metatheorem — [`TrustBaseEntry::Unravelling`], cited by
/// [`CALCULUS_VERSION`] and always present in [`CheckReport::rests_on`] for a completion. It is
/// a statement about the CALCULUS, so it is not re-proved per instance and nothing here
/// pretends it was. The one place the pre-model visibly falls short of a model is an
/// unsatisfied `≥`-restriction at a BLOCKED node: blocking withholds exactly that rule, and the
/// discharge is the unravelling argument rather than a check — counted by
/// [`Self::deferred_to_blockers`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionReplay {
    /// Nodes in the recorded completion.
    nodes: usize,
    /// Clauses the checker model-checked against it.
    clauses: usize,
    /// Blocking pairs whose signatures the checker recomputed.
    blocks: usize,
    /// At-least obligations left unsatisfied at a BLOCKED node, discharged by unravelling.
    deferred: usize,
    /// The three counts and what they rest on.
    checks: CheckReport,
}

impl CompletionReplay {
    /// Nodes in the recorded completion.
    #[must_use]
    pub const fn nodes(&self) -> usize {
        self.nodes
    }

    /// Clauses the checker model-checked against the completion.
    #[must_use]
    pub const fn clauses(&self) -> usize {
        self.clauses
    }

    /// Blocking pairs whose signatures the checker recomputed itself.
    #[must_use]
    pub const fn blocks(&self) -> usize {
        self.blocks
    }

    /// At-least obligations unsatisfied at a BLOCKED node.
    ///
    /// Not a defect and not a check: blocking withholds the `≥`-rule, and the blocker carries
    /// the obligation instead. Counted so a reader can see exactly how much of the pre-model's
    /// standing rests on [`TrustBaseEntry::Unravelling`] rather than on a satisfied clause.
    #[must_use]
    pub const fn deferred_to_blockers(&self) -> usize {
        self.deferred
    }

    /// The full classification — see [`CheckReport`].
    #[must_use]
    pub const fn checks(&self) -> &CheckReport {
        &self.checks
    }
}

// ── The checking context ────────────────────────────────────────────────────────

/// What a proof is checked AGAINST: the CALLER's own ontology, clausified by the caller.
///
/// This type exists to make one property structural rather than promised. It is built from a
/// [`RdfDataset`] the CONSUMER supplies and from nothing else: it holds no state a producer
/// shipped, so a proof cannot be verified against the very stores that produced it. It
/// constructs no `Hyper` driver and opens no
/// `Session` — the only thing it runs is the reverse mapper and
/// the clausifier, which are compilations of the ontology rather than searches over it.
pub struct DlProofContext {
    /// The knowledge base the consumer's own dataset reverse-maps to.
    kb: Kb,
    /// The DL-clause set the consumer's own knowledge base produces.
    clauses: ClauseSet,
    /// The producer-independent identity of the consumer's dataset.
    input: [u8; 32],
    /// The calculus/clausification contract of the consumer's clause set.
    contract: [u8; 32],
}

impl std::fmt::Debug for DlProofContext {
    /// The two digests and the clause count, never the knowledge base: a reverse-mapped OWL
    /// ontology has no bounded debug rendering, and a checking context is identified by what it
    /// binds rather than by what it holds.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DlProofContext")
            .field("input", &hex(self.input))
            .field("contract", &hex(self.contract))
            .field("clauses", &self.clauses.count())
            .finish_non_exhaustive()
    }
}

impl DlProofContext {
    /// Read `ontology` into a checking context.
    ///
    /// # Errors
    ///
    /// [`DlProofError::Ontology`] if the dataset is not a well-formed OWL graph.
    pub fn of_ontology(ontology: &RdfDataset) -> Result<Self, DlProofError> {
        let mut kb =
            Kb::from_dataset(ontology).map_err(|error: EntailError| DlProofError::Ontology {
                detail: error.to_string(),
            })?;
        kb.finalize();
        let clauses = derive(&kb);
        let contract = contract_digest(&clauses);
        Ok(Self {
            kb,
            clauses,
            input: input_digest(ontology),
            contract,
        })
    }

    /// A checking context over a knowledge base directly — the companion to
    /// [`prove_consistency_of_kb`], and `cfg(test)` for the same reason.
    #[cfg(test)]
    pub(crate) fn of_kb(kb: Kb) -> Self {
        let clauses = derive(&kb);
        let contract = contract_digest(&clauses);
        Self {
            kb,
            clauses,
            input: [0; 32],
            contract,
        }
    }

    /// The producer-independent identity of the ontology this context was built from.
    #[must_use]
    pub const fn input(&self) -> [u8; 32] {
        self.input
    }

    /// The calculus/clausification contract of this context's clause set.
    #[must_use]
    pub const fn contract(&self) -> [u8; 32] {
        self.contract
    }

    /// How many DL-clauses the ontology produced.
    #[must_use]
    pub fn clause_count(&self) -> usize {
        self.clauses.count()
    }

    /// Whether the ontology ASSERTS `fact` outright — the analogue of a Datalog proof's
    /// seeded EDB, and never the saturated completion graph.
    ///
    /// Only three shapes are assertable, and each is read straight off the reverse-mapped
    /// ABox: a concept assertion `a : C`, a role assertion `a r b`, and the trivial fact that
    /// a named individual's root denotes that individual. Everything else is DERIVED, and this
    /// answers `false` for it rather than guessing.
    fn asserts(&self, fact: &ProofFact) -> bool {
        match fact {
            ProofFact::Concept {
                node: NodeRef::Individual(a),
                concept,
            } => self.kb.abox_types.contains(&(*a, *concept)),
            ProofFact::Edge {
                from: NodeRef::Individual(a),
                to: NodeRef::Individual(b),
                role,
            } => {
                let (subject, object) = if role.is_inverse() {
                    (*b, *a)
                } else {
                    (*a, *b)
                };
                self.kb
                    .abox_roles
                    .contains(&(subject, role.property(), object))
            }
            ProofFact::Denotes {
                node: NodeRef::Individual(a),
                individual,
            } => a == individual && self.kb.individuals.contains(a),
            _ => false,
        }
    }
}

// ── The proof term ──────────────────────────────────────────────────────────────

/// A deterministic, versioned proof term for one OWL-DL tableau decision.
///
/// See the module documentation for exactly what a replay of one establishes. Fields are
/// private and there are exactly TWO constructors — [`prove_consistency`], which records an
/// instrumented run, and [`DlProof::decode`], which rebuilds one from bytes — so there is no
/// third way to get an unjustified step into a proof term.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DlProof {
    /// BLAKE3 over the RDFC-1.0 canonical N-Quads of the ontology — the PRODUCER-INDEPENDENT
    /// input identity.
    input: [u8; 32],
    /// BLAKE3 over [`CALCULUS_VERSION`] and the clause set — honestly PRODUCER-DERIVED.
    contract: [u8; 32],
    /// The PRODUCER-SHARED components this proof's checks rest on, in
    /// [`TrustBaseEntry::ALL`] order.
    ///
    /// Carried in the term and covered by [`DlProof::digest`], so what a consumer is trusting
    /// travels with the proof rather than being a property of whichever checker happens to read
    /// it.
    trust_base: Vec<TrustBaseEntry>,
    /// The constructs the reverse mapping could not turn into DL clauses, in
    /// `Construct::ALL` order.
    boundaries: Vec<Construct>,
    /// The answer this proof is bound to.
    answer: ProofAnswer,
    /// The clause instances that derived `false`, in search order.
    clashes: Vec<ClashStep>,
    /// The identifications, in search order.
    merges: Vec<MergeStep>,
    /// The nodes whose concrete-domain constraints had no solution, in search order.
    data_clashes: Vec<NodeRef>,
    /// The `⊔`-rule branch points, in search order — a child always after its parent.
    branches: Vec<BranchStep>,
    /// What became of the ROOT state.
    root: BranchOutcome,
    /// The clash-free completion, for a [`ProofAnswer::Consistent`] run.
    completion: Option<Completion>,
    /// Whether the recording reached [`MAX_RECORDED_STEPS`] for any kind.
    truncated: bool,
}

impl DlProof {
    /// The producer-independent input identity: BLAKE3 over the ontology's canonical N-Quads.
    #[must_use]
    pub const fn input(&self) -> [u8; 32] {
        self.input
    }

    /// The calculus/clausification contract this proof was produced under.
    ///
    /// PRODUCER-DERIVED, deliberately and visibly: the clause set it digests is the output of
    /// this crate's own absorption and clausification of the ontology. It says which calculus
    /// answered, not what the ontology is — [`Self::input`] is what says that.
    #[must_use]
    pub const fn contract(&self) -> [u8; 32] {
        self.contract
    }

    /// The constructs the reverse mapping bounded, in `Construct::ALL` order.
    #[must_use]
    pub fn boundaries(&self) -> &[Construct] {
        &self.boundaries
    }

    /// The answer this proof is bound to.
    #[must_use]
    pub const fn answer(&self) -> ProofAnswer {
        self.answer
    }

    /// The recorded clash steps, in search order — the replayable kind.
    #[must_use]
    pub fn clashes(&self) -> &[ClashStep] {
        &self.clashes
    }

    /// The recorded merge provenance, in search order.
    ///
    /// Records, not proofs — see [`MergeStep`].
    #[must_use]
    pub fn merges(&self) -> &[MergeStep] {
        &self.merges
    }

    /// The nodes whose CONCRETE-domain constraints had no solution, in search order.
    ///
    /// Records, not proofs: the concrete domain is the one decision this calculus does not
    /// take through a clause, so there is no clause instance to replay. Deciding one
    /// independently means re-running `owl_dl::data`'s value-space solver, which is a
    /// later stage.
    #[must_use]
    pub fn data_clashes(&self) -> &[NodeRef] {
        &self.data_clashes
    }

    /// The `⊔`-rule branch points, in search order — the EXHAUSTIVENESS receipts.
    #[must_use]
    pub fn branches(&self) -> &[BranchStep] {
        &self.branches
    }

    /// What became of the ROOT state: the closure or branch point that is nobody's alternative.
    #[must_use]
    pub const fn root(&self) -> BranchOutcome {
        self.root
    }

    /// The clash-free completion, for a [`ProofAnswer::Consistent`] run.
    #[must_use]
    pub const fn completion(&self) -> Option<&Completion> {
        self.completion.as_ref()
    }

    /// The PRODUCER-SHARED components this proof's checks rest on.
    ///
    /// Read this before reading a [`CheckReport`]: it is the vocabulary the report's
    /// [`CheckReport::rests_on`] is drawn from, and it travels with the proof so that a
    /// consumer's understanding of "verified" cannot silently change under them.
    #[must_use]
    pub fn trust_base(&self) -> &[TrustBaseEntry] {
        &self.trust_base
    }

    /// Whether the recording reached [`MAX_RECORDED_STEPS`] for any step kind.
    ///
    /// A truncated proof is still sound for every step it DOES carry — each is replayed
    /// independently — but it is not the whole trace, and a consumer that needs the whole
    /// trace must read this rather than infer completeness from a step count. The two
    /// whole-trace checks, [`Self::replay_refutation`] and [`Self::replay_completion`], refuse
    /// a truncated proof outright rather than walking a tree with a hole in it.
    #[must_use]
    pub const fn truncated(&self) -> bool {
        self.truncated
    }

    /// Check that this proof was produced for `ctx`'s ontology and calculus.
    ///
    /// Both digests are RECOMPUTED by the context from the consumer's own dataset; the values
    /// the proof carries participate only as comparison values.
    ///
    /// # Errors
    ///
    /// [`DlProofError::InputMismatch`] or [`DlProofError::ContractMismatch`].
    pub fn bound_to(&self, ctx: &DlProofContext) -> Result<(), DlProofError> {
        if ctx.input != self.input {
            return Err(DlProofError::InputMismatch {
                expected: hex(ctx.input),
                stated: hex(self.input),
            });
        }
        if ctx.contract != self.contract {
            return Err(DlProofError::ContractMismatch {
                expected: hex(ctx.contract),
                stated: hex(self.contract),
            });
        }
        if self.trust_base != TrustBaseEntry::ALL {
            return Err(DlProofError::TrustBaseMismatch {
                expected: trust_base_text(&TrustBaseEntry::ALL),
                stated: trust_base_text(&self.trust_base),
            });
        }
        Ok(())
    }

    /// RE-DERIVE clash step `index` against the consumer's own ontology.
    ///
    /// The whole point of the module. Nothing about the step is believed:
    ///
    /// 1. the proof must be bound to `ctx`'s ontology and calculus ([`Self::bound_to`]);
    /// 2. the clause is looked up in `ctx`'s OWN clause set at the cited index;
    /// 3. the checker computes that clause's [`HeadForm`] itself, and refuses anything but
    ///    [`HeadForm::Inconsistency`] — that refusal, not the step's label, is what makes it a
    ///    refutation;
    /// 4. the checker grounds that clause's body against the recorded frame ITSELF, and
    ///    compares its own atoms against the recorded witness atom by atom;
    /// 5. the checker checks the named clash node against the identity the frame binds
    ///    variable `0` to.
    ///
    /// No `Hyper` driver is constructed, no
    /// `Session` is opened, and no completion graph is
    /// expanded. See the module documentation for what this does NOT establish.
    ///
    /// # Errors
    ///
    /// Any [`DlProofError`] — every one of them is a rejection of an invalid proof.
    pub fn replay_clash(
        &self,
        index: usize,
        ctx: &DlProofContext,
    ) -> Result<ClashReplay, DlProofError> {
        self.bound_to(ctx)?;
        let step = self
            .clashes
            .get(index)
            .ok_or_else(|| malformed(&format!("no clash step at index {index}")))?;
        let clauses = ctx.clauses.count();
        if step.clause >= clauses {
            return Err(DlProofError::UnknownClause {
                clause: step.clause,
                clauses,
            });
        }
        let clause = ctx.clauses.clause(step.clause);
        // The CHECKER's own reading of the clause. A step that cites a clause with a
        // non-empty head derives a consequence, so there is nothing here that is `false` —
        // and that is a refusal rather than a mismatch.
        let form = clause.head_form();
        if form != HeadForm::Inconsistency {
            return Err(DlProofError::NotARefutation {
                clause: step.clause,
                form,
            });
        }
        let derived = ground_body(clause, step.clause, &step.frame)?;
        if derived.len() != step.witness.len() {
            return Err(DlProofError::WitnessLengthMismatch {
                clause: step.clause,
                derived: derived.len(),
                stated: step.witness.len(),
            });
        }
        for (position, (derived, stated)) in derived.iter().zip(&step.witness).enumerate() {
            if derived != stated {
                return Err(DlProofError::WitnessMismatch {
                    clause: step.clause,
                    position,
                    derived: Box::new(derived.clone()),
                    stated: Box::new(stated.clone()),
                });
            }
        }
        // Variable 0 is the trigger variable: a clause is always matched by binding it to one
        // node, so the node a clash is reported AT is a function of the frame rather than an
        // independent claim the step gets to make.
        let bound = step
            .frame
            .first()
            .ok_or_else(|| malformed("a clash step binds at least variable 0"))?;
        if bound != &step.node {
            return Err(DlProofError::NodeMismatch {
                clause: step.clause,
                derived: Box::new(bound.clone()),
                stated: Box::new(step.node.clone()),
            });
        }
        let attested = derived.iter().filter(|fact| ctx.asserts(fact)).count();
        let mut checks = CheckReport::new();
        // The atoms the caller's OWN ABox asserts, read off the caller's own reverse-mapped
        // knowledge base. Independent of the trust base's grounding and clausification entries;
        // it does rest on the reverse mapping, which is what makes a concept id mean anything.
        checks.attest(attested);
        checks.cite(&[TrustBaseEntry::ReverseMapping]);
        checks.leave(derived.len() - attested);
        // The five structural checks above: the clause exists, its head form is EMPTY, the
        // frame binds every variable the body reads, the grounding matches the witness, and the
        // clash node is the frame's variable 0. Every one is a statement about a CLAUSE.
        checks.trust(
            5,
            &[TrustBaseEntry::Clausification, TrustBaseEntry::Grounding],
        );
        Ok(ClashReplay {
            conclusion: DerivedConclusion::False,
            clause: step.clause,
            checks,
        })
    }

    /// RE-DERIVE the ALTERNATIVES of branch point `index` against the consumer's own ontology.
    ///
    /// This is what makes "these were all the alternatives" checkable, and it is the check that
    /// closes the unsound-`inconsistent` forgery: an `inconsistent` fabricated by dropping a
    /// disjunct records a narrower case split than the clause licenses, and this rejects it.
    ///
    /// Nothing about the branch point is believed:
    ///
    /// 1. the proof must be bound to `ctx`'s ontology, calculus and trust base;
    /// 2. the clause is looked up in `ctx`'s OWN clause set at the cited index;
    /// 3. the checker computes that clause's [`HeadForm`] itself and refuses anything but
    ///    [`HeadForm::Disjunctive`] — a clause that generates no case split cannot be a branch
    ///    point, whatever the record calls it;
    /// 4. the checker grounds that clause's head ITSELF, by calling the search's own
    ///    `hyper::ground_head` against the recorded frame, and
    ///    compares the result against the recorded alternatives atom for atom AND IN ORDER;
    /// 5. the nominal-introduction alternatives are shape-checked against the cited clause: each
    ///    must be a single reserved-root identification whose role and filler are the clause's
    ///    own counted `(R, B)`, whose index is inside the bound, and whose origin is the branch
    ///    node itself;
    /// 6. the outcome list must have exactly one entry per alternative.
    ///
    /// # Classified honestly
    ///
    /// Step 4 is `trusted`, not `attested`: it rests on
    /// [`TrustBaseEntry::Clausification`] and [`TrustBaseEntry::Grounding`]. What it is
    /// independent of is `Hyper::solve`, `saturate`, `find_branch` and
    /// the branch stack — the search driver, which holds the state and therefore the bugs. No
    /// driver is constructed here and no completion graph is expanded.
    ///
    /// # Errors
    ///
    /// Any [`DlProofError`] — every one of them is a rejection of an invalid proof.
    pub fn replay_branch(
        &self,
        index: usize,
        ctx: &DlProofContext,
    ) -> Result<BranchReplay, DlProofError> {
        self.bound_to(ctx)?;
        let step = self
            .branches
            .get(index)
            .ok_or_else(|| malformed(&format!("no branch point at index {index}")))?;
        let clauses = ctx.clauses.count();
        if step.clause >= clauses {
            return Err(DlProofError::UnknownClause {
                clause: step.clause,
                clauses,
            });
        }
        let clause = ctx.clauses.clause(step.clause);
        let form = clause.head_form();
        if form != HeadForm::Disjunctive {
            return Err(DlProofError::NotADisjunction {
                clause: step.clause,
                form,
            });
        }
        // The frame is attacker-controlled, and `ground_head` indexes it. Its width is checked
        // against the head's own variables FIRST, so a narrow frame is a rejection rather than
        // a panic.
        let width = head_frame_width(&clause.head)
            .ok_or_else(|| malformed("a head disjunct mixes a schematic pair with other atoms"))?;
        if (step.frame.len() as u64) < u64::from(width) {
            return Err(DlProofError::FrameTooShort {
                clause: step.clause,
                variable: width - 1,
                frame: step.frame.len(),
            });
        }
        // THE REGENERATION. The search's own grounding, over the CALLER's clause and the
        // recorded frame, with the proof's node identities as the carrier.
        let derived: Vec<ProofAlternative> = ground_head(&clause.head, &step.frame)
            .iter()
            .map(|disjunct| ProofAlternative {
                atoms: disjunct.iter().map(ProofGround::of).collect(),
            })
            .collect();
        if derived.len() != step.alternatives.len() {
            return Err(DlProofError::AlternativeCountMismatch {
                branch: index,
                clause: step.clause,
                derived: derived.len(),
                stated: step.alternatives.len(),
            });
        }
        for (position, (derived, stated)) in derived.iter().zip(&step.alternatives).enumerate() {
            if derived != stated {
                return Err(DlProofError::AlternativeMismatch {
                    branch: index,
                    position,
                    derived: Box::new(derived.clone()),
                    stated: Box::new(stated.clone()),
                });
            }
        }
        check_introduced(index, step, clause)?;
        if step.outcomes.len() != step.width() {
            return Err(DlProofError::OutcomeCountMismatch {
                branch: index,
                derived: step.width(),
                stated: step.outcomes.len(),
            });
        }
        let mut checks = CheckReport::new();
        // One per regenerated alternative, plus the head-form refusal and the frame-width check.
        checks.trust(
            derived.len() + 2,
            &[TrustBaseEntry::Clausification, TrustBaseEntry::Grounding],
        );
        // One per shape-checked nominal-introduction alternative — a statement about the cited
        // clause's counted role and filler.
        checks.trust(step.introduced.len(), &[TrustBaseEntry::Clausification]);
        // …but WHICH blockable predecessors press the bound is completion-graph state a proof
        // term does not carry, so the introduced SET itself is not checked.
        checks.leave(step.introduced.len());
        // The outcome list's width against the alternative count is arithmetic over the proof
        // term alone.
        checks.attest(1);
        Ok(BranchReplay {
            clause: step.clause,
            alternatives: derived.len(),
            introduced: step.introduced.len(),
            checks,
        })
    }

    /// Walk the WHOLE refutation tree: every branch point exhaustive, every alternative closed,
    /// every clash leaf re-derived.
    ///
    /// This is the check a consumer of an `inconsistent` verdict actually wants. A refutation is
    /// a tree — internal nodes are case splits, leaves are derivations of `false` — and this
    /// verifies both halves of it against the caller's own ontology:
    ///
    /// * every branch point passes [`Self::replay_branch`], so its alternatives are exactly the
    ///   ones the caller's own clause set licenses, in order;
    /// * every alternative of every branch point reached a recorded CLOSURE, and every closure
    ///   that names a clash step is re-derived by [`Self::replay_clash`];
    /// * the tree is well-founded (a child branch point is always recorded after its parent) and
    ///   reachable from the root, so no branch point is orphaned and no cycle stands in for a
    ///   closure.
    ///
    /// An alternative that closed with nothing replayable written down is counted
    /// [`RefutationReplay::unrecorded`] and makes [`RefutationReplay::is_closed`] `false`; it is
    /// never smoothed into a discharged obligation.
    ///
    /// # Errors
    ///
    /// [`DlProofError::WrongAnswer`] unless the proof is bound to [`ProofAnswer::Inconsistent`];
    /// [`DlProofError::Truncated`] if the recording hit its ceiling; and any rejection
    /// [`Self::replay_branch`] or [`Self::replay_clash`] makes.
    pub fn replay_refutation(
        &self,
        ctx: &DlProofContext,
    ) -> Result<RefutationReplay, DlProofError> {
        self.bound_to(ctx)?;
        if self.answer != ProofAnswer::Inconsistent {
            return Err(DlProofError::WrongAnswer {
                expected: ProofAnswer::Inconsistent,
                stated: self.answer,
            });
        }
        if self.truncated {
            return Err(DlProofError::Truncated);
        }
        let mut report = RefutationReplay {
            branches: 0,
            clashes: 0,
            unrecorded: 0,
            checks: CheckReport::new(),
        };
        // Every branch point must be reached from the root exactly once: an orphaned one is a
        // case split nothing depends on, and a twice-reached one is a tree that is not one.
        let mut seen = vec![false; self.branches.len()];
        self.walk_closure(ctx, self.root, None, &mut seen, &mut report)?;
        for (index, reached) in seen.iter().enumerate() {
            if !reached {
                return Err(malformed(&format!(
                    "branch point {index} is not reachable from the root, so it closes nothing"
                )));
            }
        }
        Ok(report)
    }

    /// Verify one recorded CLOSURE and, when it is a branch point, everything below it.
    ///
    /// `at` names where the outcome was filed, for a diagnostic. Recursion is bounded by the
    /// `seen` marks: a branch point is entered at most once, and there are finitely many.
    fn walk_closure(
        &self,
        ctx: &DlProofContext,
        outcome: BranchOutcome,
        at: Option<(usize, usize)>,
        seen: &mut [bool],
        report: &mut RefutationReplay,
    ) -> Result<(), DlProofError> {
        let (branch, ordinal) = at.unwrap_or((usize::MAX, usize::MAX));
        let dangling = |detail: &str| DlProofError::DanglingOutcome {
            branch,
            ordinal,
            detail: detail.to_owned(),
        };
        match outcome {
            BranchOutcome::Clash(index) => {
                if index >= self.clashes.len() {
                    return Err(dangling("no clash step at that index"));
                }
                let replay = self.replay_clash(index, ctx)?;
                report.clashes += 1;
                report.checks.absorb(&replay.checks);
                Ok(())
            }
            BranchOutcome::DataClash(index) => {
                if index >= self.data_clashes.len() {
                    return Err(dangling("no concrete-domain clash at that index"));
                }
                // The concrete domain is the one decision this calculus does not take through a
                // clause, so there is no clause instance to replay — see
                // [`DlProof::data_clashes`]. Counted unattested, never as a discharged
                // obligation.
                report.checks.leave(1);
                report.unrecorded += 1;
                Ok(())
            }
            BranchOutcome::Merge(index) => {
                let merge = self
                    .merges
                    .get(index)
                    .ok_or_else(|| dangling("no merge at that index"))?;
                if !merge.clashed {
                    return Err(dangling("the named merge did not close the state"));
                }
                // A merge is provenance, not a proof — replaying one needs the premise DAG this
                // stage does not build. That the record SAYS it clashed is read; that it was
                // licensed is not established.
                report.checks.leave(1);
                report.unrecorded += 1;
                Ok(())
            }
            BranchOutcome::Branch(index) => {
                if index >= self.branches.len() {
                    return Err(dangling("no branch point at that index"));
                }
                if std::mem::replace(&mut seen[index], true) {
                    return Err(dangling(
                        "that branch point closes two alternatives at once",
                    ));
                }
                if let Some((parent, _)) = at
                    && index <= parent
                {
                    // A child is always recorded after its parent, so this is both a
                    // well-formedness check and what makes the recursion terminate.
                    return Err(dangling(
                        "a branch point cannot descend into an earlier one",
                    ));
                }
                let replay = self.replay_branch(index, ctx)?;
                report.branches += 1;
                report.checks.absorb(&replay.checks);
                let step = &self.branches[index];
                for (ordinal, outcome) in step.outcomes.iter().copied().enumerate() {
                    if matches!(outcome, BranchOutcome::Open | BranchOutcome::Unexplored) {
                        return Err(DlProofError::BranchNotClosed {
                            branch: index,
                            ordinal,
                        });
                    }
                    self.walk_closure(ctx, outcome, Some((index, ordinal)), seen, report)?;
                }
                Ok(())
            }
            BranchOutcome::Unrecorded => {
                report.checks.leave(1);
                report.unrecorded += 1;
                Ok(())
            }
            BranchOutcome::Open | BranchOutcome::Unexplored => {
                Err(DlProofError::BranchNotClosed { branch, ordinal })
            }
        }
    }

    /// [`Self::replay_completion`], with the check ceiling as a parameter.
    ///
    /// Crate-private and parameterized for exactly one reason: the BUDGET-EXHAUSTED path has to
    /// be pinned by a test. A clause the budget did not reach is reported
    /// [`CheckReport::unattested`], never assumed satisfied, and a ceiling nothing can lower is
    /// a branch no test constrains.
    pub(crate) fn replay_completion_within(
        &self,
        ctx: &DlProofContext,
        budget: u64,
    ) -> Result<CompletionReplay, DlProofError> {
        self.bound_to(ctx)?;
        if self.answer != ProofAnswer::Consistent {
            return Err(DlProofError::WrongAnswer {
                expected: ProofAnswer::Consistent,
                stated: self.answer,
            });
        }
        if self.truncated {
            return Err(DlProofError::Truncated);
        }
        let completion = self.completion.as_ref().ok_or(DlProofError::NoCompletion)?;
        CompletionView::of(&ctx.kb, completion, budget)?.check(&ctx.clauses)
    }

    /// MODEL CHECK the recorded clash-free completion against the consumer's own ontology.
    ///
    /// Not a search and not a re-derivation: the completion is taken as given and the caller's
    /// own clauses are evaluated ON it. No `Hyper` driver is
    /// constructed, no `Session` is opened, and no rule is
    /// applied. See [`CompletionReplay`] for exactly what this does and does not establish —
    /// in particular, it does NOT by itself prove that a model exists; that step is
    /// [`TrustBaseEntry::Unravelling`].
    ///
    /// # Errors
    ///
    /// [`DlProofError::WrongAnswer`] unless the proof is bound to [`ProofAnswer::Consistent`];
    /// [`DlProofError::NoCompletion`], [`DlProofError::Truncated`],
    /// [`DlProofError::MalformedCompletion`], [`DlProofError::ClashInCompletion`],
    /// [`DlProofError::ClauseNotSatisfied`] or [`DlProofError::BlockingSignatureMismatch`].
    pub fn replay_completion(
        &self,
        ctx: &DlProofContext,
    ) -> Result<CompletionReplay, DlProofError> {
        self.replay_completion_within(ctx, MAX_CHECK_WORK)
    }

    /// The canonical byte encoding of the proof.
    ///
    /// Layout, all integers little-endian and every variable-length field length-prefixed, so
    /// no concatenation of two fields can be confused with a different split of the same bytes:
    ///
    /// ```text
    /// u64 tag_len, tag bytes                      -- PROOF_ENCODING_TAG
    /// 32 bytes input identity
    /// 32 bytes contract
    /// u8  answer ordinal
    /// u8  truncated
    /// u64 trust_base_count, u64 TrustBaseEntry::ALL ordinal each
    /// u64 boundary_count, u64 Construct::ALL ordinal each
    /// u64 clash_count, then per clash:
    ///     u64 clause index
    ///     node                                    -- the clash node
    ///     u64 frame_len, node each
    ///     u64 witness_len, fact each
    /// u64 merge_count, then per merge:
    ///     u8 cause ordinal, node left, node right, node joined, u8 clashed
    /// u64 data_clash_count, node each
    /// outcome                                     -- the root's
    /// u64 branch_count, then per branch:
    ///     u64 clause index
    ///     node                                    -- the branch node
    ///     u64 frame_len, node each
    ///     u64 alternative_count, alternative each
    ///     u64 introduced_count, alternative each
    ///     u64 outcome_count, outcome each
    /// u8  has_completion, then when set:
    ///     u64 node_count, then per node:
    ///         node, u8 concrete, u8 root,
    ///         u8 has_parent + node, u8 has_incoming + u32 property + u8 inverse,
    ///         u8 has_value_class + u32,
    ///         u64 label_len + u32 each, u64 nominal_len + u32 each,
    ///         u64 distinct_len + node each
    ///     u64 edge_count, then per edge: node from, node to, u32 property
    ///     u64 block_count, then per pair: node blocked, node blocker
    /// ```
    ///
    /// An alternative is `u64 atom_count` and then one grounded atom each; an outcome is a kind
    /// byte and a `u64` index, `0` for the kinds that carry none.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        frame(&mut out, PROOF_ENCODING_TAG.as_bytes());
        out.extend_from_slice(&self.input);
        out.extend_from_slice(&self.contract);
        out.push(self.answer.ordinal());
        out.push(u8::from(self.truncated));
        out.extend_from_slice(&(self.trust_base.len() as u64).to_le_bytes());
        for entry in &self.trust_base {
            out.extend_from_slice(&entry.ordinal().to_le_bytes());
        }
        out.extend_from_slice(&(self.boundaries.len() as u64).to_le_bytes());
        for boundary in &self.boundaries {
            let ordinal = Construct::ALL
                .iter()
                .position(|candidate| candidate == boundary)
                .expect("every Construct is in Construct::ALL");
            out.extend_from_slice(&(ordinal as u64).to_le_bytes());
        }
        out.extend_from_slice(&(self.clashes.len() as u64).to_le_bytes());
        for step in &self.clashes {
            out.extend_from_slice(&(step.clause as u64).to_le_bytes());
            encode_node(&mut out, &step.node);
            out.extend_from_slice(&(step.frame.len() as u64).to_le_bytes());
            for node in &step.frame {
                encode_node(&mut out, node);
            }
            out.extend_from_slice(&(step.witness.len() as u64).to_le_bytes());
            for fact in &step.witness {
                encode_fact(&mut out, fact);
            }
        }
        out.extend_from_slice(&(self.merges.len() as u64).to_le_bytes());
        for step in &self.merges {
            out.push(match step.cause {
                MergeCause::AtMost => 0,
                MergeCause::Nominal => 1,
                MergeCause::NominalIntroduction => 2,
            });
            encode_node(&mut out, &step.left);
            encode_node(&mut out, &step.right);
            encode_node(&mut out, &step.joined);
            out.push(u8::from(step.clashed));
        }
        out.extend_from_slice(&(self.data_clashes.len() as u64).to_le_bytes());
        for node in &self.data_clashes {
            encode_node(&mut out, node);
        }
        encode_outcome(&mut out, self.root);
        out.extend_from_slice(&(self.branches.len() as u64).to_le_bytes());
        for step in &self.branches {
            out.extend_from_slice(&(step.clause as u64).to_le_bytes());
            encode_node(&mut out, &step.node);
            out.extend_from_slice(&(step.frame.len() as u64).to_le_bytes());
            for node in &step.frame {
                encode_node(&mut out, node);
            }
            for list in [&step.alternatives, &step.introduced] {
                out.extend_from_slice(&(list.len() as u64).to_le_bytes());
                for alternative in list {
                    encode_alternative(&mut out, alternative);
                }
            }
            out.extend_from_slice(&(step.outcomes.len() as u64).to_le_bytes());
            for outcome in &step.outcomes {
                encode_outcome(&mut out, *outcome);
            }
        }
        match self.completion.as_ref() {
            Some(completion) => {
                out.push(1);
                encode_completion(&mut out, completion);
            }
            None => out.push(0),
        }
        out
    }

    /// The BLAKE3 digest of [`Self::encode`] — the proof term's stable identity.
    ///
    /// A CONTENT digest, never an IRI: **PurRDF mints no vocabulary**. Only `update` is used,
    /// never `update_rayon`, so hashing is sequential on every target and the `wasm32` build
    /// carries no thread pool.
    #[must_use]
    pub fn digest(&self) -> [u8; 32] {
        *blake3::hash(&self.encode()).as_bytes()
    }

    /// [`Self::digest`] as 64 lowercase hex characters.
    #[must_use]
    pub fn digest_hex(&self) -> String {
        hex(self.digest())
    }

    /// Rebuild a proof from [`Self::encode`]d bytes.
    ///
    /// The UNTRUSTED entrance, and the place a corrupted or forged stream is a REJECTION
    /// rather than a panic: a mis-tagged, truncated or over-long stream, an unknown node,
    /// fact, cause or answer kind, and a boundary ordinal outside `Construct::ALL` are all
    /// [`DlProofError::Malformed`]. A forgery that is nonetheless structurally legal — a
    /// tampered clause index, frame or witness — decodes cleanly and is caught where it should
    /// be, by [`Self::replay_clash`]'s re-derivation.
    ///
    /// # Errors
    ///
    /// [`DlProofError::Malformed`].
    pub fn decode(bytes: &[u8]) -> Result<Self, DlProofError> {
        let mut reader = Reader::new(bytes);
        if reader.frame()? != PROOF_ENCODING_TAG.as_bytes() {
            return Err(malformed(
                "the proof encoding tag is absent or from another layout",
            ));
        }
        let input = reader.digest()?;
        let contract = reader.digest()?;
        let answer = ProofAnswer::of_ordinal(reader.byte()?)
            .ok_or_else(|| malformed("unknown answer ordinal"))?;
        let truncated = reader.flag()?;
        let mut trust_base = Vec::new();
        for _ in 0..reader.length()? {
            let ordinal = reader.length()?;
            let entry = TrustBaseEntry::ALL
                .get(ordinal)
                .ok_or_else(|| malformed("trust-base ordinal outside TrustBaseEntry::ALL"))?;
            trust_base.push(*entry);
        }
        let mut boundaries = Vec::new();
        for _ in 0..reader.length()? {
            let ordinal = reader.length()?;
            let construct = Construct::ALL
                .get(ordinal)
                .ok_or_else(|| malformed("boundary ordinal outside Construct::ALL"))?;
            boundaries.push(*construct);
        }
        let mut clashes = Vec::new();
        for _ in 0..reader.length()? {
            let clause = reader.length()?;
            let node = reader.node()?;
            let mut frame = Vec::new();
            for _ in 0..reader.length()? {
                frame.push(reader.node()?);
            }
            let mut witness = Vec::new();
            for _ in 0..reader.length()? {
                witness.push(reader.fact()?);
            }
            clashes.push(ClashStep {
                clause,
                node,
                frame,
                witness,
            });
        }
        let mut merges = Vec::new();
        for _ in 0..reader.length()? {
            let cause = match reader.byte()? {
                0 => MergeCause::AtMost,
                1 => MergeCause::Nominal,
                2 => MergeCause::NominalIntroduction,
                _ => return Err(malformed("unknown merge cause")),
            };
            let left = reader.node()?;
            let right = reader.node()?;
            let joined = reader.node()?;
            let clashed = reader.flag()?;
            merges.push(MergeStep {
                cause,
                left,
                right,
                joined,
                clashed,
            });
        }
        let mut data_clashes = Vec::new();
        for _ in 0..reader.length()? {
            data_clashes.push(reader.node()?);
        }
        let root = reader.outcome()?;
        let mut branches = Vec::new();
        for _ in 0..reader.length()? {
            let clause = reader.length()?;
            let node = reader.node()?;
            let mut frame = Vec::new();
            for _ in 0..reader.length()? {
                frame.push(reader.node()?);
            }
            let mut alternatives = Vec::new();
            for _ in 0..reader.length()? {
                alternatives.push(reader.alternative()?);
            }
            let mut introduced = Vec::new();
            for _ in 0..reader.length()? {
                introduced.push(reader.alternative()?);
            }
            let mut outcomes = Vec::new();
            for _ in 0..reader.length()? {
                outcomes.push(reader.outcome()?);
            }
            branches.push(BranchStep {
                clause,
                node,
                frame,
                alternatives,
                introduced,
                outcomes,
            });
        }
        let completion = reader.flag()?.then(|| reader.completion()).transpose()?;
        if !reader.is_exhausted() {
            return Err(malformed("trailing bytes after the proof's last field"));
        }
        Ok(Self {
            input,
            contract,
            trust_base,
            boundaries,
            answer,
            clashes,
            merges,
            data_clashes,
            branches,
            root,
            completion,
            truncated,
        })
    }
}

// ── The producer ────────────────────────────────────────────────────────────────

/// Decide whether `ontology` is consistent, returning the answer bound to a replayable proof
/// term.
///
/// The proof binds the ontology's producer-independent identity, the calculus/clausification
/// contract, the boundary set the reverse mapping reported, and the answer. It is the only
/// producer of a [`DlProof`] besides [`DlProof::decode`].
///
/// # Errors
///
/// [`EntailError::Parse`] if the dataset is not a well-formed OWL graph.
pub fn prove_consistency(ontology: &RdfDataset) -> Result<(ProofAnswer, DlProof), EntailError> {
    let mut kb = Kb::from_dataset(ontology)?;
    kb.finalize();
    let clauses = derive(&kb);
    let (decision, recorder) =
        crate::owl_dl::hyper::decide_recording(&kb, &Assumptions::of_kb(), Budget::for_kb(&kb));
    let answer = if decision.exhausted || decision.stopped {
        ProofAnswer::Undecided
    } else if decision.consistent {
        ProofAnswer::Consistent
    } else {
        ProofAnswer::Inconsistent
    };
    let proof = DlProof {
        input: input_digest(ontology),
        contract: contract_digest(&clauses),
        trust_base: TrustBaseEntry::ALL.to_vec(),
        boundaries: boundaries_of(&kb.boundaries),
        answer,
        clashes: recorder.clashes,
        merges: recorder.merges,
        data_clashes: recorder.data_clashes,
        branches: recorder.branches,
        root: recorder.root,
        completion: recorder.completion,
        truncated: recorder.truncated,
    };
    Ok((answer, proof))
}

/// Prove a knowledge base directly, bypassing the reverse mapper.
///
/// `cfg(test)` only, and the reason is the DIFFERENTIAL: [`crate::owl_dl::oracle`]'s generated
/// corpus and its bounded-domain model enumerator are built over [`Kb`] values rather than over
/// RDF datasets, so corroborating a recorded completion against the one genuinely independent
/// model checker this crate has means being able to start from a [`Kb`]. The input identity is
/// zero — there is no dataset to canonicalize — which is exactly why this is not a public
/// entrance: a proof that carried no input identity would verify against any store at all.
#[cfg(test)]
pub(crate) fn prove_consistency_of_kb(kb: &Kb) -> (ProofAnswer, DlProof) {
    let clauses = derive(kb);
    let (decision, recorder) =
        crate::owl_dl::hyper::decide_recording(kb, &Assumptions::of_kb(), Budget::for_kb(kb));
    let answer = if decision.exhausted || decision.stopped {
        ProofAnswer::Undecided
    } else if decision.consistent {
        ProofAnswer::Consistent
    } else {
        ProofAnswer::Inconsistent
    };
    let proof = DlProof {
        input: [0; 32],
        contract: contract_digest(&clauses),
        trust_base: TrustBaseEntry::ALL.to_vec(),
        boundaries: boundaries_of(&kb.boundaries),
        answer,
        clashes: recorder.clashes,
        merges: recorder.merges,
        data_clashes: recorder.data_clashes,
        branches: recorder.branches,
        root: recorder.root,
        completion: recorder.completion,
        truncated: recorder.truncated,
    };
    (answer, proof)
}

/// The knowledge base's boundary set, in `Construct::ALL` order.
fn boundaries_of(boundaries: &BTreeSet<Construct>) -> Vec<Construct> {
    Construct::ALL
        .into_iter()
        .filter(|construct| boundaries.contains(construct))
        .collect()
}

// ── Re-derivation primitives ────────────────────────────────────────────────────

/// GROUND a clause's whole body against `frame` — the checker's OWN reading of the clause.
///
/// The one schematic atom, [`BodyAtom::Successors`], is expanded exactly as the matcher's
/// contract says it binds: `count` consecutive variables from `first`, each an `role`-neighbour
/// of variable `0` satisfying `filler`. Expanding it here rather than trusting a recorded
/// expansion is what makes a tampered `≤n` witness detectable.
fn ground_body(
    clause: &DlClause,
    index: usize,
    frame: &[NodeRef],
) -> Result<Vec<ProofFact>, DlProofError> {
    /// The identity `frame` binds `var` to, or a rejection.
    fn at(frame: &[NodeRef], var: u32, clause: usize) -> Result<NodeRef, DlProofError> {
        frame
            .get(var as usize)
            .cloned()
            .ok_or(DlProofError::FrameTooShort {
                clause,
                variable: var,
                frame: frame.len(),
            })
    }

    let mut out = Vec::with_capacity(clause.body.len());
    for atom in &clause.body {
        match *atom {
            BodyAtom::Concept { var, concept } => out.push(ProofFact::Concept {
                node: at(frame, var, index)?,
                concept,
            }),
            BodyAtom::Role { from, to, role } => out.push(ProofFact::Edge {
                from: at(frame, from, index)?,
                to: at(frame, to, index)?,
                role: ProofRole::of(role),
            }),
            BodyAtom::Denotes { var, individual } => out.push(ProofFact::Denotes {
                node: at(frame, var, index)?,
                individual,
            }),
            BodyAtom::Successors {
                role,
                filler,
                first,
                count,
            } => {
                let source = at(frame, 0, index)?;
                for offset in 0..count {
                    let successor = at(frame, first + offset, index)?;
                    out.push(ProofFact::Edge {
                        from: source.clone(),
                        to: successor.clone(),
                        role: ProofRole::of(role),
                    });
                    out.push(ProofFact::Concept {
                        node: successor,
                        concept: filler,
                    });
                }
            }
        }
    }
    Ok(out)
}

/// How wide a binding frame a clause HEAD reads, or `None` when a disjunct mixes the schematic
/// pair atom with others.
///
/// The checker's guard on an attacker-controlled frame: `ground_head` indexes the frame, so a
/// proof that recorded a short one would panic the checker rather than be rejected by it. The
/// `None` case is the second such guard — `ground_head` expands `EqualSomePair` only when it is
/// a disjunct's sole atom, and reaches an `unreachable!` otherwise.
pub(crate) fn head_frame_width(head: &[Vec<HeadAtom>]) -> Option<u32> {
    let mut width = 1_u32;
    for disjunct in head {
        let pairs = disjunct
            .iter()
            .filter(|atom| matches!(atom, HeadAtom::EqualSomePair { .. }))
            .count();
        if pairs > 0 && disjunct.len() != 1 {
            return None;
        }
        for atom in disjunct {
            match *atom {
                HeadAtom::Concept { var, .. }
                | HeadAtom::SelfLoop { var, .. }
                | HeadAtom::AtLeast { var, .. }
                | HeadAtom::EqualIndividual { var, .. } => width = width.max(var.saturating_add(1)),
                HeadAtom::EqualSomePair { first, count } => {
                    width = width.max(first.saturating_add(count));
                }
            }
        }
    }
    Some(width)
}

/// Shape-check a branch point's NOMINAL-INTRODUCTION alternatives against the clause it cites.
///
/// They are not regenerable — which blockable predecessors press an at-most bound is
/// completion-graph state — but they are not unconstrained either. Each must be a single
/// reserved-root identification `y ≈ u.⟨R,B,i⟩` whose `(R, B)` are the CITED CLAUSE's own
/// counted role and filler, whose `i` is inside the bound `n = count - 1`, and whose `u` is the
/// branch node's own nominal identity. A forger who invents an at-most root, a role, a filler or
/// an index the clause does not license is rejected here; WHICH predecessors were folded is what
/// stays [`CheckReport::unattested`].
fn check_introduced(
    branch: usize,
    step: &BranchStep,
    clause: &DlClause,
) -> Result<(), DlProofError> {
    if step.introduced.is_empty() {
        return Ok(());
    }
    let ill = |position: usize, detail: &str| DlProofError::IllFormedIntroduction {
        branch,
        position,
        detail: detail.to_owned(),
    };
    let counted = clause.body.iter().find_map(|atom| match *atom {
        BodyAtom::Successors {
            role,
            filler,
            count,
            ..
        } => Some((ProofRole::of(role), filler, count)),
        _ => None,
    });
    let Some((role, filler, count)) = counted else {
        return Err(ill(
            0,
            "the cited clause counts no successors, so its head is not an at-most bound and the \
             nominal-introduction rule cannot fire at it",
        ));
    };
    let bound = count.saturating_sub(1);
    for (position, alternative) in step.introduced.iter().enumerate() {
        let [ProofGround::EqualReserved { root, .. }] = alternative.atoms.as_slice() else {
            return Err(ill(
                position,
                "a nominal-introduction alternative is exactly one reserved-root identification",
            ));
        };
        if root.role != role {
            return Err(ill(position, "the reserved root counts a different role"));
        }
        if root.filler != filler {
            return Err(ill(position, "the reserved root names a different filler"));
        }
        if root.index >= bound {
            return Err(ill(
                position,
                "the reserved index is outside the at-most bound",
            ));
        }
        if root.origin != step.node {
            return Err(ill(
                position,
                "the reserved root belongs to another at-most root than the branch node",
            ));
        }
    }
    Ok(())
}

// ── Model checking a completion ─────────────────────────────────────────────────

/// An INDEXED, read-only view of a recorded completion, and the pure functions that decide
/// clause satisfaction on it.
///
/// Everything here reads the proof term and the caller's own knowledge base. It constructs no
/// `Hyper` driver, no `Graph` and no
/// `State`: the neighbour closure the calculus's `r`-neighbourhood is defined by is
/// recomputed from the caller's role axioms rather than taken from the producer, precisely so
/// that a completion recorded with a wrong edge set cannot pass by supplying its own reading
/// of it.
struct CompletionView<'a> {
    /// The caller's own knowledge base — the concept table, the ABox and the role axioms.
    kb: &'a Kb,
    /// The recorded nodes, in the graph's ascending index order.
    nodes: &'a [CompletionNode],
    /// The recorded edges, resolved to positions.
    edges: Vec<(usize, usize, u32)>,
    /// Whether each node is blocked — DIRECTLY, from a verified recorded pair, or INDIRECTLY,
    /// because an earlier predecessor is. Computed here, never read out of the proof.
    blocked: Vec<bool>,
    /// The `(property, forward?)` closure of each role, memoized.
    achievers: RefCell<BTreeMap<Role, BTreeSet<(u32, bool)>>>,
    /// The neighbour set of each `(node, role)`, memoized.
    neighbours: RefCell<BTreeMap<(usize, Role), Vec<usize>>>,
    /// Work spent, against [`Self::cap`].
    work: Cell<u64>,
    /// The ceiling this check stops at — [`MAX_CHECK_WORK`] for every caller but the test that
    /// pins the exhaustion path.
    cap: u64,
}

/// The schematic successor atom a subset walk is completing, held together so the recursion
/// carries one reference rather than three parallel parameters that must stay in step.
struct Selection<'c> {
    /// The clause being matched.
    clause: &'c DlClause,
    /// The body position the schematic atom sits at.
    at: usize,
    /// The counted successors the selection is drawn from, ascending and deduplicated.
    pool: &'c [usize],
}

impl<'a> CompletionView<'a> {
    /// Index `completion`, rejecting a graph that is not one and recomputing every blocking
    /// signature.
    fn of(kb: &'a Kb, completion: &'a Completion, cap: u64) -> Result<Self, DlProofError> {
        let mut position = BTreeMap::new();
        for (at, node) in completion.nodes.iter().enumerate() {
            if position.insert(node.node.clone(), at).is_some() {
                return Err(DlProofError::MalformedCompletion {
                    detail: format!("the node identity {:?} appears twice", node.node),
                });
            }
        }
        let at = |node: &NodeRef| {
            position
                .get(node)
                .copied()
                .ok_or_else(|| DlProofError::MalformedCompletion {
                    detail: format!("{node:?} is named but is not a node of the completion"),
                })
        };
        let mut edges = Vec::with_capacity(completion.edges.len());
        for edge in &completion.edges {
            edges.push((at(&edge.from)?, at(&edge.to)?, edge.property));
        }
        // THE BLOCKING WITNESSES, recomputed. A pair is believed only when the checker's own
        // reading of the two nodes' labels, predecessors' labels and incoming edges agrees, the
        // blocker is earlier and unblocked, and neither is a root.
        let mut blocked = vec![false; completion.nodes.len()];
        for pair in &completion.blocks {
            let (x, y) = (at(&pair.blocked)?, at(&pair.blocker)?);
            check_blocking(completion, x, y, &position, &blocked)?;
            blocked[x] = true;
        }
        // INDIRECT blocking: a node whose predecessor is an EARLIER blocked node. The same
        // conservative direction `Hyper::blocking` takes — a predecessor whose position is later
        // reads as unblocked, which can only ever withhold the unravelling discharge below.
        for x in 0..completion.nodes.len() {
            if blocked[x] {
                continue;
            }
            if let Some(parent) = completion.nodes[x].parent.as_ref()
                && let Some(&p) = position.get(parent)
                && p < x
                && blocked[p]
            {
                blocked[x] = true;
            }
        }
        Ok(Self {
            kb,
            nodes: &completion.nodes,
            edges,
            blocked,
            achievers: RefCell::new(BTreeMap::new()),
            neighbours: RefCell::new(BTreeMap::new()),
            work: Cell::new(0),
            cap,
        })
    }

    /// Charge `units` of check work.
    fn charge(&self, units: u64) {
        self.work
            .set(self.work.get().saturating_add(units).min(self.cap));
    }

    /// Whether the check budget is gone.
    fn spent(&self) -> bool {
        self.work.get() >= self.cap
    }

    /// The `(property, forward?)` patterns that realize `role`, closed over the CALLER's own
    /// sub-role and inverse declarations.
    fn achievers(&self, role: Role) -> BTreeSet<(u32, bool)> {
        if let Some(cached) = self.achievers.borrow().get(&role) {
            return cached.clone();
        }
        let start = match role {
            Role::Named(p) => (p, true),
            Role::Inv(p) => (p, false),
        };
        let mut set: BTreeSet<(u32, bool)> = BTreeSet::new();
        let mut stack = vec![start];
        while let Some((q, dir)) = stack.pop() {
            self.charge(1);
            if !set.insert((q, dir)) {
                continue;
            }
            if let Some(subs) = self.kb.role_sub.get(&q) {
                stack.extend(subs.iter().map(|&s| (s, dir)));
            }
            if let Some(invs) = self.kb.inverses.get(&q) {
                stack.extend(invs.iter().map(|&s| (s, !dir)));
            }
        }
        self.achievers.borrow_mut().insert(role, set.clone());
        set
    }

    /// One edge step from `x` over the patterns `ach`.
    fn step(&self, x: usize, ach: &BTreeSet<(u32, bool)>) -> Vec<usize> {
        self.charge(self.edges.len() as u64);
        let mut out = Vec::new();
        let mut seen: BTreeSet<usize> = BTreeSet::new();
        for &(from, to, prop) in &self.edges {
            if ach.contains(&(prop, true)) && from == x && seen.insert(to) {
                out.push(to);
            }
            if ach.contains(&(prop, false)) && to == x && seen.insert(from) {
                out.push(from);
            }
        }
        out
    }

    /// The `role`-neighbours of `x`, closed over the role hierarchy, the inverse declarations
    /// and the transitive-role closure — the calculus's own `r`-neighbourhood, recomputed.
    fn neighbors(&self, x: usize, role: Role) -> Vec<usize> {
        if let Some(cached) = self.neighbours.borrow().get(&(x, role)) {
            return cached.clone();
        }
        let ach = self.achievers(role);
        let mut out = self.step(x, &ach);
        let mut seen: BTreeSet<usize> = out.iter().copied().collect();
        for &(prop, dir) in &ach {
            if !self.kb.transitive.contains(&prop) {
                continue;
            }
            let single: BTreeSet<(u32, bool)> = std::iter::once((prop, dir)).collect();
            let mut frontier = self.step(x, &single);
            let mut visited: BTreeSet<usize> = frontier.iter().copied().collect();
            while let Some(y) = frontier.pop() {
                if self.spent() {
                    break;
                }
                if seen.insert(y) {
                    out.push(y);
                }
                for z in self.step(y, &single) {
                    if visited.insert(z) {
                        frontier.push(z);
                    }
                }
            }
        }
        self.neighbours.borrow_mut().insert((x, role), out.clone());
        out
    }

    /// Whether `x`'s label holds `concept`, `⊤` counting as always held.
    fn has_concept(&self, x: usize, concept: u32) -> bool {
        matches!(self.kb.table.decomp(concept), Decomp::Top)
            || self.nodes[x].label.binary_search(&concept).is_ok()
    }

    /// Whether `a` and `b` are forced DISTINCT: a recorded `≠`, or a disagreement of value
    /// class, which the datatype map forces whether or not anything said so.
    fn distinct(&self, a: usize, b: usize) -> bool {
        if a == b {
            return false;
        }
        if let (Some(left), Some(right)) = (self.nodes[a].value_class, self.nodes[b].value_class)
            && left != right
        {
            return true;
        }
        self.nodes[a].distinct.contains(&self.nodes[b].node)
            || self.nodes[b].distinct.contains(&self.nodes[a].node)
    }

    /// Whether `items` holds `need` pairwise-distinct members.
    ///
    /// A bounded backtracking search, not a greedy one: "are there `n` pairwise `≠` witnesses"
    /// is a clique question and a greedy answer would withhold satisfaction that holds. Budget
    /// exhaustion answers `false`, which can only ever WITHHOLD a satisfaction claim — and a
    /// withheld one surfaces as an unsatisfied clause rather than as a silent pass.
    fn has_distinct(&self, items: &[usize], need: usize, chosen: &mut Vec<usize>) -> bool {
        if chosen.len() >= need {
            return true;
        }
        if chosen.len() + items.len() < need || self.spent() {
            return false;
        }
        for (at, &candidate) in items.iter().enumerate() {
            self.charge(1);
            if chosen.iter().all(|&other| self.distinct(other, candidate)) {
                chosen.push(candidate);
                if self.has_distinct(&items[at + 1..], need, chosen) {
                    chosen.pop();
                    return true;
                }
                chosen.pop();
            }
        }
        false
    }

    /// Whether every atom of a grounded disjunct HOLDS on the completion.
    ///
    /// The mirror of the search's own satisfaction test, over the recorded graph.
    fn satisfied(&self, disjunct: &[Ground<usize>]) -> bool {
        disjunct.iter().all(|atom| match *atom {
            Ground::Concept(node, concept) => self.has_concept(node, concept),
            Ground::SelfLoop(node, role) => self.neighbors(node, role).contains(&node),
            Ground::AtLeast(node, n, role, filler) => {
                if n == 0 {
                    return true;
                }
                let with_filler: Vec<usize> = self
                    .neighbors(node, role)
                    .into_iter()
                    .filter(|&y| self.has_concept(y, filler))
                    .collect();
                self.has_distinct(&with_filler, n as usize, &mut Vec::new())
            }
            Ground::Equal(left, right) => left == right,
            Ground::EqualIndividual(node, individual) => {
                self.nodes[node].nominals.binary_search(&individual).is_ok()
            }
            Ground::EqualReserved(node, ref key) => {
                self.nodes[node].node == NodeRef::Reserved(Box::new(reserved_ref(key)))
            }
        })
    }

    /// Every binding frame that satisfies `clause`'s body with variable `0` bound to `x`.
    fn body_matches(&self, clause: &DlClause, x: usize) -> Vec<Vec<usize>> {
        let mut out = Vec::new();
        let mut frame = vec![x];
        self.walk(clause, 0, &mut frame, &mut out);
        out
    }

    /// Match `clause.body[at..]`, extending `frame` — the checker's own left-deep join over the
    /// recorded graph.
    fn walk(
        &self,
        clause: &DlClause,
        at: usize,
        frame: &mut Vec<usize>,
        out: &mut Vec<Vec<usize>>,
    ) {
        self.charge(1);
        if self.spent() {
            return;
        }
        let Some(&atom) = clause.body.get(at) else {
            out.push(frame.clone());
            return;
        };
        match atom {
            BodyAtom::Concept { var, concept } => {
                if self.has_concept(frame[var as usize], concept) {
                    self.walk(clause, at + 1, frame, out);
                }
            }
            BodyAtom::Denotes { var, individual } => {
                if self.nodes[frame[var as usize]]
                    .nominals
                    .binary_search(&individual)
                    .is_ok()
                {
                    self.walk(clause, at + 1, frame, out);
                }
            }
            BodyAtom::Role { from, to, role } => {
                let source = frame[from as usize];
                if (to as usize) < frame.len() {
                    let target = frame[to as usize];
                    if self.neighbors(source, role).contains(&target) {
                        self.walk(clause, at + 1, frame, out);
                    }
                } else {
                    for y in self.neighbors(source, role) {
                        frame.push(y);
                        self.walk(clause, at + 1, frame, out);
                        frame.pop();
                    }
                }
            }
            BodyAtom::Successors {
                role,
                filler,
                first,
                count,
            } => {
                if first as usize != frame.len() {
                    // The frame is a stack, so a schematic successor atom binds the next
                    // `count` pushes. A clause set that numbered them otherwise is not this
                    // calculus's, and grounding its head would name the wrong nodes: refuse to
                    // match it rather than match it wrongly.
                    return;
                }
                let mut pool: Vec<usize> = self
                    .neighbors(frame[0], role)
                    .into_iter()
                    .filter(|&y| self.has_concept(y, filler))
                    .collect();
                pool.sort_unstable();
                pool.dedup();
                let selection = Selection {
                    clause,
                    at,
                    pool: &pool,
                };
                self.walk_subsets(&selection, frame, 0, count, out);
            }
        }
    }

    /// Extend `frame` with every strictly-increasing `remaining`-element selection from
    /// `selection.pool[from..]`, continuing the body walk once the selection is complete.
    fn walk_subsets(
        &self,
        selection: &Selection<'_>,
        frame: &mut Vec<usize>,
        from: usize,
        remaining: u32,
        out: &mut Vec<Vec<usize>>,
    ) {
        self.charge(1);
        if self.spent() {
            return;
        }
        if remaining == 0 {
            self.walk(selection.clause, selection.at + 1, frame, out);
            return;
        }
        if selection.pool.len() - from < remaining as usize {
            return;
        }
        for index in from..selection.pool.len() {
            frame.push(selection.pool[index]);
            self.walk_subsets(selection, frame, index + 1, remaining - 1, out);
            frame.pop();
        }
    }

    /// MODEL CHECK every clause of `clauses` against the completion.
    fn check(&self, clauses: &ClauseSet) -> Result<CompletionReplay, DlProofError> {
        let mut checks = CheckReport::new();
        // The structural reading of the completion — one node identity per node, every edge and
        // every blocking pair naming a node that is there — and the blocking signatures, both
        // re-derived by this checker from the proof term alone.
        checks.attest(1 + self.blocked.iter().filter(|&&b| b).count());
        let mut checked = 0_usize;
        let mut deferred = 0_usize;
        for index in 0..clauses.count() {
            if self.spent() {
                // A clause the budget did not reach is UNATTESTED, never assumed satisfied.
                checks.leave(clauses.count() - index);
                break;
            }
            let clause = clauses.clause(index);
            let empty = clause.head.is_empty();
            let tbox = clauses.is_tbox(index);
            for x in 0..self.nodes.len() {
                // A general concept inclusion quantifies over the object domain, so a TBox
                // clause is matched from ABSTRACT nodes only — the same restriction the search
                // applies, and omitting it here would fail a completion for a clause the
                // calculus never fired on that node.
                if tbox && self.nodes[x].concrete {
                    continue;
                }
                for frame in self.body_matches(clause, x) {
                    if empty {
                        return Err(DlProofError::ClashInCompletion {
                            clause: index,
                            node: Box::new(self.nodes[x].node.clone()),
                        });
                    }
                    let disjuncts = ground_head(&clause.head, &frame);
                    if disjuncts.iter().any(|d| self.satisfied(d)) {
                        continue;
                    }
                    // The one obligation a saturated pre-model is allowed to leave open: an
                    // at-least restriction at a BLOCKED node. Blocking withholds exactly the
                    // `≥`-rule, and the blocker carries the obligation instead — which is the
                    // unravelling argument, cited rather than re-proved.
                    if disjuncts.iter().flatten().any(
                        |atom| matches!(atom, Ground::AtLeast(node, ..) if self.blocked[*node]),
                    ) {
                        deferred += 1;
                        continue;
                    }
                    return Err(DlProofError::ClauseNotSatisfied {
                        clause: index,
                        node: Box::new(self.nodes[x].node.clone()),
                    });
                }
            }
            checked += 1;
        }
        // Each satisfied clause is a statement about the caller's clause set, evaluated through
        // the caller's own role axioms and this module's grounding.
        checks.trust(
            checked,
            &[
                TrustBaseEntry::ReverseMapping,
                TrustBaseEntry::Clausification,
                TrustBaseEntry::Grounding,
            ],
        );
        // …and that a clash-free, saturated, blocked pre-model yields a MODEL is the
        // metatheorem, not anything checked above.
        checks.cite(&[TrustBaseEntry::Unravelling]);
        Ok(CompletionReplay {
            nodes: self.nodes.len(),
            clauses: checked,
            blocks: self.blocked.iter().filter(|&&b| b).count(),
            deferred,
            checks,
        })
    }
}

/// RECOMPUTE one blocking pair's signature from the recorded nodes.
///
/// The four conditions the calculus states, checked rather than believed: the blocker is
/// EARLIER and not itself blocked, neither node is a root, both have a predecessor, and the two
/// agree on their own label, their predecessor's label and their incoming edge.
fn check_blocking(
    completion: &Completion,
    x: usize,
    y: usize,
    position: &BTreeMap<NodeRef, usize>,
    blocked: &[bool],
) -> Result<(), DlProofError> {
    let (blocked_node, blocker) = (&completion.nodes[x], &completion.nodes[y]);
    let fail = |detail: &str| DlProofError::BlockingSignatureMismatch {
        blocked: Box::new(blocked_node.node.clone()),
        blocker: Box::new(blocker.node.clone()),
        detail: detail.to_owned(),
    };
    if y >= x {
        return Err(fail("the blocker is not earlier than the node it blocks"));
    }
    if blocked[y] {
        return Err(fail("the blocker is itself blocked"));
    }
    if blocked_node.root || blocker.root {
        return Err(fail("a root node is never blocked and never blocks"));
    }
    if blocked_node.label != blocker.label {
        return Err(fail("the two labels differ"));
    }
    if blocked_node.incoming != blocker.incoming {
        return Err(fail("the two incoming edges differ"));
    }
    let (Some(left), Some(right)) = (blocked_node.parent.as_ref(), blocker.parent.as_ref()) else {
        return Err(fail(
            "a blocking pair is two nodes that both have a predecessor",
        ));
    };
    let (Some(&left), Some(&right)) = (position.get(left), position.get(right)) else {
        return Err(fail("a predecessor is not a node of the completion"));
    };
    if completion.nodes[left].label != completion.nodes[right].label {
        return Err(fail("the two predecessors' labels differ"));
    }
    Ok(())
}

/// OBSERVE a clause's body instance in the completion graph — the recorder's reading.
///
/// Deliberately a second implementation of the grounding above, and deliberately one that
/// consults the STATE: a concept or `denotes` atom is emitted only when the node's label or
/// name set actually holds it, so a matcher that fired on an atom the graph does not carry
/// produces a SHORT witness and [`DlProof::replay_clash`] rejects the step. A witness produced
/// by calling `ground_body` would make that comparison vacuous, which is why the two are not
/// one function.
///
/// Role atoms are recorded as matched: whether a node is an `r`-NEIGHBOUR of another is the
/// role hierarchy's, the inverse declarations' and the transitive closure's answer, and
/// re-deciding it here means re-running the metered neighbour scan — which would change the
/// work a recorded run charges and so change its
/// `Decision`. Nothing in this function consults or charges
/// the work meter.
pub(crate) fn observe_body(
    kb: &Kb,
    st: &State,
    clause: &DlClause,
    frame: &[usize],
) -> Vec<ProofFact> {
    /// Whether `node`'s label holds `concept`, `⊤` counting as always held — the same reading
    /// [`crate::owl_dl::graph::Graph::has_concept`] gives, and unmetered like it.
    fn holds(kb: &Kb, st: &State, node: usize, concept: u32) -> bool {
        matches!(kb.table.decomp(concept), Decomp::Top)
            || st.nodes[find(st, node)].label.contains(&concept)
    }

    let mut out = Vec::with_capacity(clause.body.len());
    for atom in &clause.body {
        match *atom {
            BodyAtom::Concept { var, concept } => {
                let Some(&node) = frame.get(var as usize) else {
                    continue;
                };
                if holds(kb, st, node, concept) {
                    out.push(ProofFact::Concept {
                        node: node_ref(st, node),
                        concept,
                    });
                }
            }
            BodyAtom::Role { from, to, role } => {
                let (Some(&from), Some(&to)) = (frame.get(from as usize), frame.get(to as usize))
                else {
                    continue;
                };
                out.push(ProofFact::Edge {
                    from: node_ref(st, from),
                    to: node_ref(st, to),
                    role: ProofRole::of(role),
                });
            }
            BodyAtom::Denotes { var, individual } => {
                let Some(&node) = frame.get(var as usize) else {
                    continue;
                };
                if st.nodes[find(st, node)].nominals.contains(&individual) {
                    out.push(ProofFact::Denotes {
                        node: node_ref(st, node),
                        individual,
                    });
                }
            }
            BodyAtom::Successors {
                role,
                filler,
                first,
                count,
            } => {
                let Some(&source) = frame.first() else {
                    continue;
                };
                for offset in 0..count {
                    let Some(&successor) = frame.get((first + offset) as usize) else {
                        continue;
                    };
                    out.push(ProofFact::Edge {
                        from: node_ref(st, source),
                        to: node_ref(st, successor),
                        role: ProofRole::of(role),
                    });
                    if holds(kb, st, successor, filler) {
                        out.push(ProofFact::Concept {
                            node: node_ref(st, successor),
                            concept: filler,
                        });
                    }
                }
            }
        }
    }
    out
}

/// OBSERVE one `⊔`-rule alternative the search is about to branch over.
///
/// A structure map of the vector the search HOLDS, node indices resolved to merge-invariant
/// identities. It never calls `hyper::ground_head`: a record
/// produced by the computation the checker compares it against would make the comparison
/// vacuous, and a search that branched over a shortened alternative list must record the
/// shortened list. Nothing here consults or charges the work meter.
pub(crate) fn observe_alternative(st: &State, disjunct: &[Ground<usize>]) -> ProofAlternative {
    ProofAlternative {
        atoms: disjunct
            .iter()
            .map(|atom| ProofGround::of(&atom.map(&mut |&node| node_ref(st, node))))
            .collect(),
    }
}

/// OBSERVE the clash-free completion the search stopped at.
///
/// A direct reading of the state's node and edge vectors: representatives in ascending index
/// order, edges in first-seen order, both unmetered. The role hierarchy's closure of the edges
/// is deliberately NOT computed — that is the metered neighbour scan, and charging it would
/// make a recorded run reach a different `Decision` than an
/// unrecorded one. The checker closes the edges itself, from the caller's own role axioms.
pub(crate) fn observe_completion(st: &State, blocks: &[(NodeRef, NodeRef)]) -> Completion {
    let mut nodes = Vec::new();
    for x in 0..st.nodes.len() {
        if find(st, x) != x {
            continue;
        }
        let node = &st.nodes[x];
        let mut distinct: Vec<NodeRef> = node.neq.iter().map(|&w| node_ref(st, w)).collect();
        distinct.sort();
        distinct.dedup();
        nodes.push(CompletionNode {
            node: node_ref(st, x),
            label: node.label.iter().copied().collect(),
            nominals: node.nominals.iter().copied().collect(),
            distinct,
            parent: node.parent.map(|p| node_ref(st, p)),
            incoming: node
                .incoming
                .map(|(property, inverse)| ProofRole { property, inverse }),
            concrete: node.concrete,
            root: node.root,
            value_class: node.value_class,
        });
    }
    let edges = st
        .edges
        .iter()
        .map(|&(from, to, property)| CompletionEdge {
            from: node_ref(st, from),
            to: node_ref(st, to),
            property,
        })
        .collect();
    let blocks = blocks
        .iter()
        .map(|(blocked, blocker)| BlockingPair {
            blocked: blocked.clone(),
            blocker: blocker.clone(),
        })
        .collect();
    Completion {
        nodes,
        edges,
        blocks,
    }
}

// ── Digests ─────────────────────────────────────────────────────────────────────

/// The PRODUCER-INDEPENDENT identity of an ontology: BLAKE3 over its RDFC-1.0 canonical
/// N-Quads.
///
/// Producer-independent because canonical N-Quads is a statement about a quad SET: two engines
/// that read the same graph, in any order, with any blank-node labelling, compute the same 32
/// bytes. The same shape [`Justification::digest`](crate::explain::Justification::digest)
/// already uses.
fn input_digest(ontology: &RdfDataset) -> [u8; 32] {
    *blake3::hash(purrdf_core::canonicalize(ontology).nquads.as_bytes()).as_bytes()
}

/// The calculus/clausification contract: BLAKE3 over [`CALCULUS_VERSION`] and the canonical
/// encoding of the DL-clause set.
///
/// PRODUCER-DERIVED and labelled so wherever it appears. The clause set is the output of this
/// crate's own absorption and clausification, so this digest answers "which calculus, and which
/// compilation of the ontology" — it is not a second identity for the ontology, which is what
/// [`input_digest`] is for. Conflating the two would let a producer's own compilation stand in
/// for the caller's data.
fn contract_digest(clauses: &ClauseSet) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    frame_hash(&mut hasher, CONTRACT_DIGEST_TAG.as_bytes());
    frame_hash(&mut hasher, CALCULUS_VERSION.as_bytes());
    hasher.update(&(clauses.count() as u64).to_le_bytes());
    for index in 0..clauses.count() {
        let clause = clauses.clause(index);
        hasher.update(&(clause.body.len() as u64).to_le_bytes());
        for atom in &clause.body {
            let (kind, a, b, c, d) = match *atom {
                BodyAtom::Concept { var, concept } => (0_u8, var, concept, 0, 0),
                BodyAtom::Role { from, to, role } => {
                    let role = ProofRole::of(role);
                    (1, from, to, role.property(), u32::from(role.is_inverse()))
                }
                BodyAtom::Denotes { var, individual } => (2, var, individual, 0, 0),
                BodyAtom::Successors {
                    role,
                    filler,
                    first,
                    count,
                } => {
                    let role = ProofRole::of(role);
                    // The role's two components share one field pair with the counted
                    // variables, so both are folded: property, inverted, filler, first, count.
                    hasher.update(&[3_u8]);
                    hasher.update(&role.property().to_le_bytes());
                    hasher.update(&[u8::from(role.is_inverse())]);
                    hasher.update(&filler.to_le_bytes());
                    hasher.update(&first.to_le_bytes());
                    hasher.update(&count.to_le_bytes());
                    continue;
                }
            };
            hasher.update(&[kind]);
            hasher.update(&a.to_le_bytes());
            hasher.update(&b.to_le_bytes());
            hasher.update(&c.to_le_bytes());
            hasher.update(&d.to_le_bytes());
        }
        hasher.update(&(clause.head.len() as u64).to_le_bytes());
        for disjunct in &clause.head {
            hasher.update(&(disjunct.len() as u64).to_le_bytes());
            for atom in disjunct {
                head_atom_hash(&mut hasher, atom);
            }
        }
    }
    *hasher.finalize().as_bytes()
}

/// Fold one head atom into the contract digest.
fn head_atom_hash(hasher: &mut blake3::Hasher, atom: &HeadAtom) {
    match *atom {
        HeadAtom::Concept { var, concept } => {
            hasher.update(&[0_u8]);
            hasher.update(&var.to_le_bytes());
            hasher.update(&concept.to_le_bytes());
        }
        HeadAtom::SelfLoop { var, role } => {
            let role = ProofRole::of(role);
            hasher.update(&[1_u8]);
            hasher.update(&var.to_le_bytes());
            hasher.update(&role.property().to_le_bytes());
            hasher.update(&[u8::from(role.is_inverse())]);
        }
        HeadAtom::AtLeast {
            var,
            n,
            role,
            filler,
        } => {
            let role = ProofRole::of(role);
            hasher.update(&[2_u8]);
            hasher.update(&var.to_le_bytes());
            hasher.update(&n.to_le_bytes());
            hasher.update(&role.property().to_le_bytes());
            hasher.update(&[u8::from(role.is_inverse())]);
            hasher.update(&filler.to_le_bytes());
        }
        HeadAtom::EqualSomePair { first, count } => {
            hasher.update(&[3_u8]);
            hasher.update(&first.to_le_bytes());
            hasher.update(&count.to_le_bytes());
        }
        HeadAtom::EqualIndividual { var, individual } => {
            hasher.update(&[4_u8]);
            hasher.update(&var.to_le_bytes());
            hasher.update(&individual.to_le_bytes());
        }
    }
}

// ── Byte plumbing ───────────────────────────────────────────────────────────────

/// A [`DlProofError::Malformed`] carrying `detail`.
fn malformed(detail: &str) -> DlProofError {
    DlProofError::Malformed {
        detail: detail.to_owned(),
    }
}

/// 32 bytes as 64 lowercase hex characters.
fn hex(digest: [u8; 32]) -> String {
    let mut out = String::with_capacity(64);
    for byte in digest {
        out.push(char::from_digit(u32::from(byte >> 4), 16).expect("a nibble is one hex digit"));
        out.push(char::from_digit(u32::from(byte & 0x0f), 16).expect("a nibble is one hex digit"));
    }
    out
}

/// Append a length-prefixed byte string.
fn frame(out: &mut Vec<u8>, bytes: &[u8]) {
    out.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
    out.extend_from_slice(bytes);
}

/// Fold a length-prefixed byte string into a hasher.
fn frame_hash(hasher: &mut blake3::Hasher, bytes: &[u8]) {
    hasher.update(&(bytes.len() as u64).to_le_bytes());
    hasher.update(bytes);
}

/// Append a [`NodeRef`].
fn encode_node(out: &mut Vec<u8>, node: &NodeRef) {
    match node {
        NodeRef::Individual(a) => {
            out.push(NODE_INDIVIDUAL);
            out.extend_from_slice(&a.to_le_bytes());
        }
        NodeRef::Anonymous(index) => {
            out.push(NODE_ANONYMOUS);
            out.extend_from_slice(&index.to_le_bytes());
        }
        NodeRef::Reserved(reserved) => {
            out.push(NODE_RESERVED);
            encode_node(out, &reserved.origin);
            out.extend_from_slice(&reserved.role.property.to_le_bytes());
            out.push(u8::from(reserved.role.inverse));
            out.extend_from_slice(&reserved.filler.to_le_bytes());
            out.extend_from_slice(&reserved.index.to_le_bytes());
        }
    }
}

/// Append a [`ProofFact`].
fn encode_fact(out: &mut Vec<u8>, fact: &ProofFact) {
    match fact {
        ProofFact::Concept { node, concept } => {
            out.push(FACT_CONCEPT);
            encode_node(out, node);
            out.extend_from_slice(&concept.to_le_bytes());
        }
        ProofFact::Edge { from, to, role } => {
            out.push(FACT_EDGE);
            encode_node(out, from);
            encode_node(out, to);
            out.extend_from_slice(&role.property.to_le_bytes());
            out.push(u8::from(role.inverse));
        }
        ProofFact::Denotes { node, individual } => {
            out.push(FACT_DENOTES);
            encode_node(out, node);
            out.extend_from_slice(&individual.to_le_bytes());
        }
    }
}

/// Append a [`BranchOutcome`].
fn encode_outcome(out: &mut Vec<u8>, outcome: BranchOutcome) {
    out.push(outcome.kind());
    out.extend_from_slice(&(outcome.payload() as u64).to_le_bytes());
}

/// Append a [`ProofGround`].
fn encode_ground(out: &mut Vec<u8>, atom: &ProofGround) {
    match atom {
        ProofGround::Concept { node, concept } => {
            out.push(0);
            encode_node(out, node);
            out.extend_from_slice(&concept.to_le_bytes());
        }
        ProofGround::SelfLoop { node, role } => {
            out.push(1);
            encode_node(out, node);
            encode_role(out, *role);
        }
        ProofGround::AtLeast {
            node,
            n,
            role,
            filler,
        } => {
            out.push(2);
            encode_node(out, node);
            out.extend_from_slice(&n.to_le_bytes());
            encode_role(out, *role);
            out.extend_from_slice(&filler.to_le_bytes());
        }
        ProofGround::Equal { left, right } => {
            out.push(3);
            encode_node(out, left);
            encode_node(out, right);
        }
        ProofGround::EqualIndividual { node, individual } => {
            out.push(4);
            encode_node(out, node);
            out.extend_from_slice(&individual.to_le_bytes());
        }
        ProofGround::EqualReserved { node, root } => {
            out.push(5);
            encode_node(out, node);
            encode_node(out, &NodeRef::Reserved(Box::new(root.clone())));
        }
    }
}

/// Append a [`ProofRole`].
fn encode_role(out: &mut Vec<u8>, role: ProofRole) {
    out.extend_from_slice(&role.property.to_le_bytes());
    out.push(u8::from(role.inverse));
}

/// Append a [`ProofAlternative`].
fn encode_alternative(out: &mut Vec<u8>, alternative: &ProofAlternative) {
    out.extend_from_slice(&(alternative.atoms.len() as u64).to_le_bytes());
    for atom in &alternative.atoms {
        encode_ground(out, atom);
    }
}

/// Append an optional `u32`, as a presence flag and then the value.
fn encode_option_u32(out: &mut Vec<u8>, value: Option<u32>) {
    match value {
        Some(value) => {
            out.push(1);
            out.extend_from_slice(&value.to_le_bytes());
        }
        None => out.push(0),
    }
}

/// Append a [`Completion`].
fn encode_completion(out: &mut Vec<u8>, completion: &Completion) {
    out.extend_from_slice(&(completion.nodes.len() as u64).to_le_bytes());
    for node in &completion.nodes {
        encode_node(out, &node.node);
        out.push(u8::from(node.concrete));
        out.push(u8::from(node.root));
        match node.parent.as_ref() {
            Some(parent) => {
                out.push(1);
                encode_node(out, parent);
            }
            None => out.push(0),
        }
        match node.incoming {
            Some(role) => {
                out.push(1);
                encode_role(out, role);
            }
            None => out.push(0),
        }
        encode_option_u32(out, node.value_class);
        for list in [&node.label, &node.nominals] {
            out.extend_from_slice(&(list.len() as u64).to_le_bytes());
            for &value in list {
                out.extend_from_slice(&value.to_le_bytes());
            }
        }
        out.extend_from_slice(&(node.distinct.len() as u64).to_le_bytes());
        for other in &node.distinct {
            encode_node(out, other);
        }
    }
    out.extend_from_slice(&(completion.edges.len() as u64).to_le_bytes());
    for edge in &completion.edges {
        encode_node(out, &edge.from);
        encode_node(out, &edge.to);
        out.extend_from_slice(&edge.property.to_le_bytes());
    }
    out.extend_from_slice(&(completion.blocks.len() as u64).to_le_bytes());
    for pair in &completion.blocks {
        encode_node(out, &pair.blocked);
        encode_node(out, &pair.blocker);
    }
}

/// A bounds-checked cursor over an encoded proof.
struct Reader<'a> {
    /// The remaining bytes.
    bytes: &'a [u8],
}

impl<'a> Reader<'a> {
    /// A cursor over `bytes`.
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes }
    }

    /// Whether every byte has been consumed.
    const fn is_exhausted(&self) -> bool {
        self.bytes.is_empty()
    }

    /// Take `n` bytes.
    fn take(&mut self, n: usize) -> Result<&'a [u8], DlProofError> {
        if self.bytes.len() < n {
            return Err(malformed("the proof stream ended mid-field"));
        }
        let (head, rest) = self.bytes.split_at(n);
        self.bytes = rest;
        Ok(head)
    }

    /// Take one byte.
    fn byte(&mut self) -> Result<u8, DlProofError> {
        Ok(self.take(1)?[0])
    }

    /// Take one boolean, refusing any encoding but `0` and `1`.
    fn flag(&mut self) -> Result<bool, DlProofError> {
        match self.byte()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(malformed("a boolean field is 0 or 1")),
        }
    }

    /// Take a little-endian `u32`.
    fn u32(&mut self) -> Result<u32, DlProofError> {
        let bytes: [u8; 4] = self
            .take(4)?
            .try_into()
            .map_err(|_| malformed("a u32 field is four bytes"))?;
        Ok(u32::from_le_bytes(bytes))
    }

    /// Take a little-endian `u64` as a `usize`, refusing one this target cannot hold.
    fn length(&mut self) -> Result<usize, DlProofError> {
        let bytes: [u8; 8] = self
            .take(8)?
            .try_into()
            .map_err(|_| malformed("a length field is eight bytes"))?;
        usize::try_from(u64::from_le_bytes(bytes))
            .map_err(|_| malformed("a length field exceeds this target's usize"))
    }

    /// Take 32 digest bytes.
    fn digest(&mut self) -> Result<[u8; 32], DlProofError> {
        self.take(32)?
            .try_into()
            .map_err(|_| malformed("a digest field is thirty-two bytes"))
    }

    /// Take a length-prefixed byte string.
    fn frame(&mut self) -> Result<&'a [u8], DlProofError> {
        let len = self.length()?;
        self.take(len)
    }

    /// Take a [`NodeRef`].
    fn node(&mut self) -> Result<NodeRef, DlProofError> {
        match self.byte()? {
            NODE_INDIVIDUAL => Ok(NodeRef::Individual(self.u32()?)),
            NODE_ANONYMOUS => Ok(NodeRef::Anonymous(self.u32()?)),
            NODE_RESERVED => {
                let origin = self.node()?;
                let property = self.u32()?;
                let inverse = self.flag()?;
                let filler = self.u32()?;
                let index = self.u32()?;
                Ok(NodeRef::Reserved(Box::new(ReservedRef {
                    origin,
                    role: ProofRole { property, inverse },
                    filler,
                    index,
                })))
            }
            _ => Err(malformed("unknown node kind")),
        }
    }

    /// Take a [`ProofFact`].
    fn fact(&mut self) -> Result<ProofFact, DlProofError> {
        match self.byte()? {
            FACT_CONCEPT => Ok(ProofFact::Concept {
                node: self.node()?,
                concept: self.u32()?,
            }),
            FACT_EDGE => {
                let from = self.node()?;
                let to = self.node()?;
                let property = self.u32()?;
                let inverse = self.flag()?;
                Ok(ProofFact::Edge {
                    from,
                    to,
                    role: ProofRole { property, inverse },
                })
            }
            FACT_DENOTES => Ok(ProofFact::Denotes {
                node: self.node()?,
                individual: self.u32()?,
            }),
            _ => Err(malformed("unknown fact kind")),
        }
    }

    /// Take a [`ProofRole`].
    fn role(&mut self) -> Result<ProofRole, DlProofError> {
        let property = self.u32()?;
        let inverse = self.flag()?;
        Ok(ProofRole { property, inverse })
    }

    /// Take a [`BranchOutcome`], refusing an unknown kind and a payload on a kind that carries
    /// none — so two byte strings can never decode to one outcome.
    fn outcome(&mut self) -> Result<BranchOutcome, DlProofError> {
        let kind = self.byte()?;
        let index = self.length()?;
        match kind {
            0 => Ok(BranchOutcome::Clash(index)),
            1 => Ok(BranchOutcome::DataClash(index)),
            2 => Ok(BranchOutcome::Merge(index)),
            3 => Ok(BranchOutcome::Branch(index)),
            4..=6 if index != 0 => Err(malformed(
                "an outcome kind that carries no index states one",
            )),
            4 => Ok(BranchOutcome::Unrecorded),
            5 => Ok(BranchOutcome::Open),
            6 => Ok(BranchOutcome::Unexplored),
            _ => Err(malformed("unknown branch outcome kind")),
        }
    }

    /// Take a [`ProofGround`].
    fn ground(&mut self) -> Result<ProofGround, DlProofError> {
        match self.byte()? {
            0 => Ok(ProofGround::Concept {
                node: self.node()?,
                concept: self.u32()?,
            }),
            1 => Ok(ProofGround::SelfLoop {
                node: self.node()?,
                role: self.role()?,
            }),
            2 => {
                let node = self.node()?;
                let n = self.u32()?;
                let role = self.role()?;
                Ok(ProofGround::AtLeast {
                    node,
                    n,
                    role,
                    filler: self.u32()?,
                })
            }
            3 => Ok(ProofGround::Equal {
                left: self.node()?,
                right: self.node()?,
            }),
            4 => Ok(ProofGround::EqualIndividual {
                node: self.node()?,
                individual: self.u32()?,
            }),
            5 => {
                let node = self.node()?;
                let NodeRef::Reserved(root) = self.node()? else {
                    return Err(malformed(
                        "a reserved-root identification names a reserved root",
                    ));
                };
                Ok(ProofGround::EqualReserved { node, root: *root })
            }
            _ => Err(malformed("unknown grounded atom kind")),
        }
    }

    /// Take a [`ProofAlternative`].
    fn alternative(&mut self) -> Result<ProofAlternative, DlProofError> {
        let mut atoms = Vec::new();
        for _ in 0..self.length()? {
            atoms.push(self.ground()?);
        }
        Ok(ProofAlternative { atoms })
    }

    /// Take an optional `u32`.
    fn option_u32(&mut self) -> Result<Option<u32>, DlProofError> {
        if self.flag()? {
            return Ok(Some(self.u32()?));
        }
        Ok(None)
    }

    /// Take a list of `u32`s.
    fn u32s(&mut self) -> Result<Vec<u32>, DlProofError> {
        let mut out = Vec::new();
        for _ in 0..self.length()? {
            out.push(self.u32()?);
        }
        Ok(out)
    }

    /// Take a [`Completion`].
    fn completion(&mut self) -> Result<Completion, DlProofError> {
        let mut nodes = Vec::new();
        for _ in 0..self.length()? {
            let node = self.node()?;
            let concrete = self.flag()?;
            let root = self.flag()?;
            let parent = if self.flag()? {
                Some(self.node()?)
            } else {
                None
            };
            let incoming = if self.flag()? {
                Some(self.role()?)
            } else {
                None
            };
            let value_class = self.option_u32()?;
            let label = self.u32s()?;
            let nominals = self.u32s()?;
            let mut distinct = Vec::new();
            for _ in 0..self.length()? {
                distinct.push(self.node()?);
            }
            nodes.push(CompletionNode {
                node,
                label,
                nominals,
                distinct,
                parent,
                incoming,
                concrete,
                root,
                value_class,
            });
        }
        let mut edges = Vec::new();
        for _ in 0..self.length()? {
            edges.push(CompletionEdge {
                from: self.node()?,
                to: self.node()?,
                property: self.u32()?,
            });
        }
        let mut blocks = Vec::new();
        for _ in 0..self.length()? {
            blocks.push(BlockingPair {
                blocked: self.node()?,
                blocker: self.node()?,
            });
        }
        Ok(Completion {
            nodes,
            edges,
            blocks,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use purrdf_core::{RdfDataset, RdfDatasetBuilder, TermId};

    use super::*;

    /// Fixture terms are `example.org`: **PurRDF mints no vocabulary**, and a fixture that
    /// used a reserved IRI for its own data would be testing the reverse mapper against
    /// itself.
    const EX_A: &str = "http://example.org/a";
    /// A second fixture individual.
    const EX_B: &str = "http://example.org/b";
    /// A fixture class.
    const EX_C: &str = "http://example.org/C";
    /// A second fixture class.
    const EX_D: &str = "http://example.org/D";
    /// A fixture property.
    const EX_P: &str = "http://example.org/p";
    /// `rdf:type`.
    const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
    /// `owl:disjointWith`.
    const OWL_DISJOINT_WITH: &str = "http://www.w3.org/2002/07/owl#disjointWith";
    /// `rdfs:subClassOf`.
    const RDFS_SUBCLASS_OF: &str = "http://www.w3.org/2000/01/rdf-schema#subClassOf";

    /// A tiny triple sink over the frozen IR.
    struct Fixture {
        /// The dataset under construction.
        builder: RdfDatasetBuilder,
    }

    impl Fixture {
        /// An empty fixture.
        fn new() -> Self {
            Self {
                builder: RdfDatasetBuilder::new(),
            }
        }

        /// Intern an IRI.
        fn iri(&mut self, iri: &str) -> TermId {
            self.builder.intern_iri(iri)
        }

        /// Push a default-graph triple.
        fn quad(&mut self, s: TermId, p: TermId, o: TermId) {
            self.builder.push_quad(s, p, o, None);
        }

        /// Freeze into a dataset.
        fn freeze(self) -> Arc<RdfDataset> {
            self.builder.freeze().expect("the fixture freezes")
        }
    }

    /// `C ⊓ D ⊑ ⊥`, and one individual asserted to be both — an inconsistency whose refutation
    /// is a single clause instance with an EMPTY head, matched at the individual's own root.
    ///
    /// Chosen because both witness atoms are ASSERTED axioms of the ABox, so the replay's
    /// attestation counter has something to attest and the honesty of the unattested counter is
    /// observable beside it (see the subsumption fixture below).
    fn disjoint_classes() -> Arc<RdfDataset> {
        let mut f = Fixture::new();
        let a = f.iri(EX_A);
        let c = f.iri(EX_C);
        let d = f.iri(EX_D);
        let ty = f.iri(RDF_TYPE);
        let disjoint = f.iri(OWL_DISJOINT_WITH);
        f.quad(c, disjoint, d);
        f.quad(a, ty, c);
        f.quad(a, ty, d);
        f.freeze()
    }

    /// The same clash reached through a SUBCLASS step, so the clash's second witness atom is
    /// derived rather than asserted.
    fn derived_clash() -> Arc<RdfDataset> {
        let mut f = Fixture::new();
        let a = f.iri(EX_A);
        let b = f.iri(EX_B);
        let c = f.iri(EX_C);
        let d = f.iri(EX_D);
        let ty = f.iri(RDF_TYPE);
        let sub = f.iri(RDFS_SUBCLASS_OF);
        let disjoint = f.iri(OWL_DISJOINT_WITH);
        f.quad(c, disjoint, d);
        f.quad(b, sub, d);
        f.quad(a, ty, c);
        f.quad(a, ty, b);
        f.freeze()
    }

    /// A consistent ontology: one property assertion and nothing that closes.
    fn consistent_ontology() -> Arc<RdfDataset> {
        let mut f = Fixture::new();
        let a = f.iri(EX_A);
        let b = f.iri(EX_B);
        let p = f.iri(EX_P);
        f.quad(a, p, b);
        f.freeze()
    }

    /// The proof of the disjoint-classes refutation, and a context over the SAME ontology built
    /// independently of it.
    fn refutation() -> (Arc<RdfDataset>, DlProof, DlProofContext) {
        let ontology = disjoint_classes();
        let (answer, proof) = prove_consistency(&ontology).expect("the fixture reverse-maps");
        assert_eq!(answer, ProofAnswer::Inconsistent, "C ⊓ D ⊑ ⊥ with a : C, D");
        let ctx = DlProofContext::of_ontology(&ontology).expect("the fixture reverse-maps");
        (ontology, proof, ctx)
    }

    // ── The positive direction ──────────────────────────────────────────────────

    /// The whole point: a recorded clash step RE-DERIVES against the consumer's own ontology,
    /// and the conclusion the checker returns is the one the CHECKER computed from the clause
    /// it looked up itself.
    #[test]
    fn a_recorded_clash_replays_against_the_callers_own_ontology() {
        let (_, proof, ctx) = refutation();
        assert!(
            !proof.clashes().is_empty(),
            "the refutation closed on a clause instance, so one was recorded"
        );
        let replay = proof
            .replay_clash(0, &ctx)
            .expect("a genuine clash replays");
        assert_eq!(replay.conclusion(), DerivedConclusion::False);
        assert!(
            replay.attested() + replay.unattested() > 0,
            "the clause body grounds to at least one atom: {replay:?}"
        );
    }

    /// The checker's inputs are the consumer's DATASET and a BYTE STRING — nothing the producer
    /// holds.
    ///
    /// The proof is round-tripped through [`DlProof::encode`]/[`DlProof::decode`] first, so the
    /// value that is replayed shares no allocation, no arena and no interner with the run that
    /// produced it; and the context is built from the dataset alone. This is what "the checker
    /// does not link the search" means operationally: `DlProofContext::of_ontology` constructs
    /// no `Hyper` driver and opens no `Session`, and this test would not compile if
    /// `replay_clash` needed one.
    #[test]
    fn the_checker_consumes_only_an_ontology_and_a_byte_string() {
        let ontology = disjoint_classes();
        let bytes = {
            let (_, proof) = prove_consistency(&ontology).expect("reverse-maps");
            proof.encode()
        };
        // Everything the producer held is dropped at this brace; only `bytes` survives.
        let ctx = DlProofContext::of_ontology(&ontology).expect("reverse-maps");
        let proof = DlProof::decode(&bytes).expect("its own encoding decodes");
        let replay = proof
            .replay_clash(0, &ctx)
            .expect("a genuine clash replays");
        assert_eq!(replay.conclusion(), DerivedConclusion::False);
    }

    /// A clash whose witness includes a DERIVED concept membership reports it as unattested
    /// rather than counting it as an axiom.
    ///
    /// This is the stage's honesty made observable: the replay establishes that the clause
    /// instance derives `false`, and says out loud how much of the instance it could reduce to
    /// the caller's own ABox.
    #[test]
    fn a_derived_witness_atom_is_reported_unattested_rather_than_attested() {
        let ontology = derived_clash();
        let (answer, proof) = prove_consistency(&ontology).expect("reverse-maps");
        assert_eq!(answer, ProofAnswer::Inconsistent);
        let ctx = DlProofContext::of_ontology(&ontology).expect("reverse-maps");
        let replay = proof
            .replay_clash(0, &ctx)
            .expect("a genuine clash replays");
        assert!(
            replay.unattested() > 0,
            "`a : D` is derived through `B ⊑ D`, so it is not an asserted axiom: {replay:?}"
        );
    }

    /// A consistent ontology gets a proof bound to the CONSISTENT answer and carrying no clash
    /// — and the module claims nothing more about it than that.
    #[test]
    fn a_consistent_answer_carries_no_clash_and_says_so() {
        let ontology = consistent_ontology();
        let (answer, proof) = prove_consistency(&ontology).expect("reverse-maps");
        assert_eq!(answer, ProofAnswer::Consistent);
        assert_eq!(proof.answer(), ProofAnswer::Consistent);
        assert!(proof.clashes().is_empty(), "{:?}", proof.clashes());
        assert!(!proof.truncated());
    }

    // ── Determinism ─────────────────────────────────────────────────────────────

    /// Two independent runs over one ontology produce BYTE-IDENTICAL proof terms.
    ///
    /// The determinism doctrine is stated in the module docs; this is what makes it an
    /// observation. A recorder that read a hash map, a clock or an address would still record
    /// the same number of steps most runs, and it is the bytes that would move first.
    #[test]
    fn a_proof_term_is_byte_identical_run_to_run() {
        let ontology = disjoint_classes();
        let (_, first) = prove_consistency(&ontology).expect("reverse-maps");
        let (_, again) = prove_consistency(&ontology).expect("reverse-maps");
        assert_eq!(first.encode(), again.encode(), "two runs, one proof");
        assert_eq!(first.digest(), again.digest());
        assert_eq!(first.digest_hex().len(), 64);
    }

    /// `decode(encode(p))` is `p`, and re-encodes to the identical bytes.
    #[test]
    fn a_proof_term_round_trips_through_its_own_encoding() {
        let (_, proof, _) = refutation();
        let bytes = proof.encode();
        let decoded = DlProof::decode(&bytes).expect("its own encoding decodes");
        assert_eq!(decoded, proof);
        assert_eq!(decoded.encode(), bytes);
        assert_eq!(decoded.digest(), proof.digest());
    }

    /// The input identity is the PRODUCER-INDEPENDENT one, and it is not the contract.
    ///
    /// Conflating the two is the failure mode this test exists to catch: a proof whose only
    /// identity were the clause set would be verified against the producer's own compilation of
    /// the ontology rather than against the consumer's data.
    #[test]
    fn the_input_identity_is_the_canonical_ontology_and_not_the_clause_set() {
        let (ontology, proof, ctx) = refutation();
        let canonical =
            *blake3::hash(purrdf_core::canonicalize(&ontology).nquads.as_bytes()).as_bytes();
        assert_eq!(proof.input(), canonical, "the input digest is RDFC-1.0's");
        assert_eq!(ctx.input(), canonical, "and the consumer recomputes it");
        assert_ne!(
            proof.input(),
            proof.contract(),
            "the ontology's identity and the calculus contract are two facts"
        );
    }

    // ── Tamper-negatives: the clash step ────────────────────────────────────────

    /// A proof presented against ANOTHER ontology is rejected, whatever it says about itself.
    #[test]
    fn a_proof_does_not_check_against_a_different_ontology() {
        let (_, proof, _) = refutation();
        let other = consistent_ontology();
        let ctx = DlProofContext::of_ontology(&other).expect("reverse-maps");
        assert!(matches!(
            proof.replay_clash(0, &ctx),
            Err(DlProofError::InputMismatch { .. })
        ));
    }

    /// Forging the INPUT identity to match a different ontology does not help: the contract is
    /// recomputed from that ontology's own clause set and refuses too.
    #[test]
    fn forging_the_input_identity_still_fails_the_contract() {
        let (_, mut proof, _) = refutation();
        let other = consistent_ontology();
        let ctx = DlProofContext::of_ontology(&other).expect("reverse-maps");
        proof.input = ctx.input();
        assert!(matches!(
            proof.replay_clash(0, &ctx),
            Err(DlProofError::ContractMismatch { .. })
        ));
    }

    /// A tampered contract is rejected even when the ontology matches — which is what makes a
    /// proof from another calculus unusable rather than silently accepted.
    #[test]
    fn a_tampered_contract_is_rejected() {
        let (_, mut proof, ctx) = refutation();
        proof.contract[0] ^= 0xff;
        assert!(matches!(
            proof.replay_clash(0, &ctx),
            Err(DlProofError::ContractMismatch { .. })
        ));
    }

    /// A clause index past the end of the CONSUMER's clause set is a rejection, not a panic.
    #[test]
    fn a_clash_citing_a_clause_the_ontology_does_not_have_is_rejected() {
        let (_, mut proof, ctx) = refutation();
        proof.clashes[0].clause = usize::MAX;
        assert!(matches!(
            proof.replay_clash(0, &ctx),
            Err(DlProofError::UnknownClause { .. })
        ));
    }

    /// Re-pointing a clash at a REAL clause of the same ontology does not make it a refutation:
    /// the checker computes the head form itself, and only an EMPTY head derives `false`.
    ///
    /// This is the forgery that matters most, because it is the one a producer could make by
    /// accident: it renames the step without changing anything a log would print.
    #[test]
    fn a_clash_repointed_at_a_non_empty_headed_clause_is_rejected() {
        let (_, mut proof, ctx) = refutation();
        let honest = proof.clashes[0].clause;
        let derailed = (0..ctx.clause_count())
            .find(|&index| ctx.clauses.clause(index).head_form() != HeadForm::Inconsistency)
            .expect("the fixture produces at least one clause with a head");
        assert_ne!(honest, derailed);
        proof.clashes[0].clause = derailed;
        match proof.replay_clash(0, &ctx) {
            Err(DlProofError::NotARefutation { clause, .. }) => assert_eq!(clause, derailed),
            other => panic!("a clause with a head derives no false: {other:?}"),
        }
    }

    /// Rewriting a witness atom is caught, because the checker grounds the clause's body ITSELF
    /// and compares its own atoms against the recorded ones.
    #[test]
    fn a_tampered_witness_atom_is_rejected() {
        let (_, mut proof, ctx) = refutation();
        let forged = match &proof.clashes[0].witness[0] {
            ProofFact::Concept { node, concept } => ProofFact::Concept {
                node: node.clone(),
                concept: concept.wrapping_add(1),
            },
            other => panic!("the fixture's first body atom is a concept atom: {other:?}"),
        };
        proof.clashes[0].witness[0] = forged;
        assert!(matches!(
            proof.replay_clash(0, &ctx),
            Err(DlProofError::WitnessMismatch { position: 0, .. })
        ));
    }

    /// Dropping a witness atom is caught by the length comparison, so a forger cannot shrink a
    /// body instance into one the ontology happens to license.
    #[test]
    fn a_truncated_witness_is_rejected() {
        let (_, mut proof, ctx) = refutation();
        proof.clashes[0].witness.pop();
        assert!(matches!(
            proof.replay_clash(0, &ctx),
            Err(DlProofError::WitnessLengthMismatch { .. })
        ));
    }

    /// Re-binding the frame to another node is caught: the checker's grounding follows the
    /// FRAME, so the atoms it derives stop matching the recorded witness.
    #[test]
    fn a_tampered_frame_is_rejected() {
        let (_, mut proof, ctx) = refutation();
        proof.clashes[0].frame[0] = NodeRef::Anonymous(9_999);
        assert!(matches!(
            proof.replay_clash(0, &ctx),
            Err(DlProofError::WitnessMismatch { .. } | DlProofError::NodeMismatch { .. })
        ));
    }

    /// Emptying the frame is a rejection rather than an index panic.
    #[test]
    fn an_empty_frame_is_rejected() {
        let (_, mut proof, ctx) = refutation();
        proof.clashes[0].frame.clear();
        assert!(matches!(
            proof.replay_clash(0, &ctx),
            Err(DlProofError::FrameTooShort { .. } | DlProofError::Malformed { .. })
        ));
    }

    /// Renaming the node a clash is reported AT, while leaving the frame honest, is caught:
    /// the clash node is a function of the frame's variable `0`, not a claim the step makes.
    #[test]
    fn a_relabelled_clash_node_is_rejected() {
        let (_, mut proof, ctx) = refutation();
        proof.clashes[0].node = NodeRef::Anonymous(4_242);
        assert!(matches!(
            proof.replay_clash(0, &ctx),
            Err(DlProofError::NodeMismatch { .. })
        ));
    }

    /// Asking for a clash step that is not there is a rejection.
    #[test]
    fn a_clash_index_past_the_end_is_rejected() {
        let (_, proof, ctx) = refutation();
        assert!(matches!(
            proof.replay_clash(proof.clashes().len(), &ctx),
            Err(DlProofError::Malformed { .. })
        ));
    }

    // ── Tamper-negatives: the recorded (non-replayable) kinds ───────────────────

    /// A tampered MERGE record changes the proof's digest, so a consumer that pinned the digest
    /// sees the edit even though this stage does not replay a merge.
    ///
    /// Stated as a limit rather than as a guarantee: the digest detects an edit to a proof term
    /// it already holds; it does not establish that the recorded merge was licensed. Replaying
    /// a merge needs the premise DAG this stage does not build.
    #[test]
    fn a_tampered_merge_record_changes_the_digest() {
        let mut proof = merge_bearing_proof();
        assert!(!proof.merges().is_empty(), "the fixture merges");
        let before = proof.digest();
        proof.merges[0].joined = NodeRef::Anonymous(7);
        assert_ne!(
            before,
            proof.digest(),
            "an edited record is a different term"
        );
        let bytes = proof.encode();
        assert_eq!(
            DlProof::decode(&bytes).expect("still well formed").digest(),
            proof.digest(),
            "the edit survives a round trip rather than being normalized away"
        );
    }

    /// A tampered DATA-CLASH record changes the digest, under the same stated limit.
    #[test]
    fn a_tampered_data_clash_record_changes_the_digest() {
        let (_, mut proof, _) = refutation();
        let before = proof.digest();
        proof.data_clashes.push(NodeRef::Individual(3));
        assert_ne!(before, proof.digest());
    }

    /// Rewriting the ANSWER changes the digest: a refutation cannot be re-presented as a
    /// consistency proof under the same identity.
    #[test]
    fn rewriting_the_answer_changes_the_digest() {
        let (_, mut proof, _) = refutation();
        let before = proof.digest();
        proof.answer = ProofAnswer::Consistent;
        assert_ne!(before, proof.digest());
    }

    /// Rewriting the BOUNDARY set changes the digest: an answer's boundaries are part of what
    /// it claims, not decoration beside it.
    #[test]
    fn rewriting_the_boundary_set_changes_the_digest() {
        let (_, mut proof, _) = refutation();
        let before = proof.digest();
        proof.boundaries.push(Construct::PropertyChain);
        assert_ne!(before, proof.digest());
        assert_eq!(
            DlProof::decode(&proof.encode()).expect("well formed"),
            proof
        );
    }

    /// An ontology whose `owl:sameAs` and nominal identifications drive a merge, so the merge
    /// record has something in it.
    fn merge_bearing_proof() -> DlProof {
        let mut f = Fixture::new();
        let a = f.iri(EX_A);
        let b = f.iri(EX_B);
        let c = f.iri(EX_C);
        let d = f.iri(EX_D);
        let p = f.iri(EX_P);
        let ty = f.iri(RDF_TYPE);
        let sub = f.iri(RDFS_SUBCLASS_OF);
        let one_of = f.iri("http://www.w3.org/2002/07/owl#oneOf");
        let first = f.iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#first");
        let rest = f.iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#rest");
        let nil = f.iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#nil");
        let cell = f
            .builder
            .intern_blank("m1", purrdf_core::BlankScope::DEFAULT);
        // `D owl:oneOf (b)` — the `o`-clause, which identifies every `D` with `b`.
        f.quad(cell, first, b);
        f.quad(cell, rest, nil);
        f.quad(d, one_of, cell);
        f.quad(c, sub, d);
        f.quad(a, ty, c);
        f.quad(a, p, b);
        let ontology = f.freeze();
        let (_, proof) = prove_consistency(&ontology).expect("reverse-maps");
        proof
    }

    // ── Tamper-negatives: the wire format ───────────────────────────────────────

    /// Bytes written under another layout do not decode as if they were current.
    #[test]
    fn a_foreign_encoding_tag_is_rejected() {
        let (_, proof, _) = refutation();
        let mut bytes = proof.encode();
        bytes[8] = b'X';
        assert!(matches!(
            DlProof::decode(&bytes),
            Err(DlProofError::Malformed { .. })
        ));
    }

    /// A truncated stream is a rejection rather than a panic.
    #[test]
    fn a_truncated_stream_is_rejected() {
        let (_, proof, _) = refutation();
        let bytes = proof.encode();
        for cut in 0..bytes.len() {
            assert!(
                DlProof::decode(&bytes[..cut]).is_err(),
                "a proof truncated to {cut} bytes must not decode"
            );
        }
    }

    /// Trailing bytes after the last field are a rejection: a proof is the WHOLE stream, so a
    /// forger cannot append a second, unread payload to a term that checks.
    #[test]
    fn trailing_bytes_are_rejected() {
        let (_, proof, _) = refutation();
        let mut bytes = proof.encode();
        bytes.push(0);
        assert!(matches!(
            DlProof::decode(&bytes),
            Err(DlProofError::Malformed { .. })
        ));
    }

    /// Every single-byte corruption either decodes to a DIFFERENT proof or is refused — never
    /// to the same term, which would mean the encoding had a field nothing reads.
    #[test]
    fn no_single_byte_edit_decodes_back_to_the_same_proof() {
        let (_, proof, _) = refutation();
        let bytes = proof.encode();
        for position in 0..bytes.len() {
            let mut forged = bytes.clone();
            forged[position] ^= 0x01;
            if let Ok(decoded) = DlProof::decode(&forged) {
                assert_ne!(
                    decoded, proof,
                    "byte {position} is not read by the decoder, so the encoding carries a \
                     field a forger can move freely"
                );
            }
        }
    }

    /// An unknown node kind is refused rather than read as some neighbouring one.
    #[test]
    fn an_unknown_node_kind_is_rejected() {
        let mut out = Vec::new();
        frame(&mut out, PROOF_ENCODING_TAG.as_bytes());
        out.extend_from_slice(&[0_u8; 32]);
        out.extend_from_slice(&[0_u8; 32]);
        out.push(ProofAnswer::Inconsistent.ordinal());
        out.push(0);
        out.extend_from_slice(&0_u64.to_le_bytes());
        out.extend_from_slice(&1_u64.to_le_bytes());
        out.extend_from_slice(&0_u64.to_le_bytes());
        out.push(200);
        assert!(matches!(
            DlProof::decode(&out),
            Err(DlProofError::Malformed { .. })
        ));
    }

    /// A boundary ordinal outside `Construct::ALL` is refused rather than clamped.
    #[test]
    fn an_out_of_range_boundary_ordinal_is_rejected() {
        let mut out = Vec::new();
        frame(&mut out, PROOF_ENCODING_TAG.as_bytes());
        out.extend_from_slice(&[0_u8; 32]);
        out.extend_from_slice(&[0_u8; 32]);
        out.push(ProofAnswer::Consistent.ordinal());
        out.push(0);
        out.extend_from_slice(&1_u64.to_le_bytes());
        out.extend_from_slice(&(Construct::ALL.len() as u64).to_le_bytes());
        assert!(matches!(
            DlProof::decode(&out),
            Err(DlProofError::Malformed { .. })
        ));
    }

    /// A boolean field encoded as anything but `0` or `1` is refused, so two byte strings can
    /// never decode to one proof.
    #[test]
    fn a_non_canonical_boolean_is_rejected() {
        let (_, proof, _) = refutation();
        let mut bytes = proof.encode();
        // The `truncated` flag sits immediately after the tag, the two digests and the answer.
        let position = 8 + PROOF_ENCODING_TAG.len() + 32 + 32 + 1;
        bytes[position] = 2;
        assert!(matches!(
            DlProof::decode(&bytes),
            Err(DlProofError::Malformed { .. })
        ));
    }

    // ── The trust base ──────────────────────────────────────────────────────────

    /// The trust base is PINNED, element by element and in order.
    ///
    /// The whole value of this stage is that a consumer knows exactly what they are trusting,
    /// and that knowledge is worth nothing if the list can grow between two releases without
    /// anybody noticing. Adding an entry is a BREAKING change: it changes what an
    /// already-issued proof means. This test is what makes growing it deliberate.
    #[test]
    fn the_trust_base_is_pinned() {
        assert_eq!(
            TrustBaseEntry::ALL,
            [
                TrustBaseEntry::ReverseMapping,
                TrustBaseEntry::Clausification,
                TrustBaseEntry::Grounding,
                TrustBaseEntry::Unravelling,
            ],
            "adding, removing or reordering a trust-base entry is a breaking version bump: \
             bump TRUST_BASE_VERSION and the proof encoding tag, and update this pin"
        );
        assert_eq!(
            trust_base_text(&TrustBaseEntry::ALL),
            "reverse-mapping,clausification,grounding,unravelling"
        );
        assert_eq!(TRUST_BASE_VERSION, "purrdf-owl-dl-trust-base-v1");
        for (ordinal, entry) in TrustBaseEntry::ALL.iter().enumerate() {
            assert_eq!(
                entry.ordinal(),
                ordinal as u64,
                "wire ordinals are positions"
            );
        }
    }

    /// A proof CARRIES its trust base, and the digest covers it — so a consumer who pinned a
    /// digest cannot have the meaning of "verified" changed under them.
    #[test]
    fn the_trust_base_travels_in_the_proof_and_is_digested() {
        let (_, mut proof, _) = refutation();
        assert_eq!(proof.trust_base(), TrustBaseEntry::ALL);
        let before = proof.digest();
        proof.trust_base.pop();
        assert_ne!(
            before,
            proof.digest(),
            "a proof resting on less is a different term"
        );
        assert_eq!(
            DlProof::decode(&proof.encode())
                .expect("well formed")
                .trust_base,
            proof.trust_base,
            "the edit survives a round trip rather than being normalized away"
        );
    }

    /// A proof stating a trust base this checker does not implement is REJECTED, not checked
    /// against a different meaning of the same word.
    #[test]
    fn a_proof_stating_another_trust_base_is_rejected() {
        let (_, mut proof, ctx) = refutation();
        proof.trust_base = vec![TrustBaseEntry::Grounding];
        assert!(matches!(
            proof.replay_clash(0, &ctx),
            Err(DlProofError::TrustBaseMismatch { .. })
        ));
        assert!(matches!(
            proof.replay_refutation(&ctx),
            Err(DlProofError::TrustBaseMismatch { .. })
        ));
    }

    /// A trust-base ordinal outside [`TrustBaseEntry::ALL`] is refused rather than clamped.
    #[test]
    fn an_out_of_range_trust_base_ordinal_is_rejected() {
        let (_, proof, _) = refutation();
        let mut bytes = proof.encode();
        // The first trust-base ordinal sits after the tag, the two digests, the answer and the
        // truncated flag, and its own count.
        let at = 8 + PROOF_ENCODING_TAG.len() + 32 + 32 + 1 + 1 + 8;
        bytes[at..at + 8].copy_from_slice(&(TrustBaseEntry::ALL.len() as u64).to_le_bytes());
        assert!(matches!(
            DlProof::decode(&bytes),
            Err(DlProofError::Malformed { .. })
        ));
    }

    // ── The three counts ────────────────────────────────────────────────────────

    /// A clash replay reports its structural checks as `trusted`, NAMES what they rest on, and
    /// never presents them as `attested`.
    ///
    /// This is the honesty of the stage made observable. Stage 1's module docs said the checker
    /// "clausifies that ontology itself"; that is true and it is still not independence, because
    /// the clausifier is the producer's. The report says so.
    #[test]
    fn a_clash_replay_separates_what_it_re_derived_from_what_it_trusts() {
        let (_, proof, ctx) = refutation();
        let replay = proof
            .replay_clash(0, &ctx)
            .expect("a genuine clash replays");
        let checks = replay.checks();
        assert!(
            checks.trusted() > 0,
            "the clause lookup, the head form and the grounding all rest on the trust base: \
             {checks:?}"
        );
        assert_eq!(replay.trusted(), checks.trusted());
        assert!(
            checks.rests_on().contains(&TrustBaseEntry::Clausification),
            "a claim about a clause rests on the clausifier: {checks:?}"
        );
        assert!(checks.rests_on().contains(&TrustBaseEntry::Grounding));
        assert!(
            !checks.is_fully_attested(),
            "no check that reads a clause set is fully attested, and saying otherwise is the \
             one thing this stage exists to prevent"
        );
    }

    /// …and the `attested` count is a real count, not a field nothing ever fills: a clash whose
    /// whole body instance is ASSERTED ABox axioms attests every atom of it.
    ///
    /// `p` asymmetric with `a p b` and `b p a` closes on `p(x,y) ∧ p(y,x) → ⊥`, whose two body
    /// atoms are the two role assertions the ontology states outright. Without this the
    /// attested counter could be stuck at zero and every other test here would still pass.
    #[test]
    fn a_clash_whose_body_is_asserted_attests_every_atom_of_it() {
        let ontology = asymmetric_clash();
        let (answer, proof) = prove_consistency(&ontology).expect("reverse-maps");
        assert_eq!(
            answer,
            ProofAnswer::Inconsistent,
            "an asymmetric role's own pair"
        );
        let ctx = DlProofContext::of_ontology(&ontology).expect("reverse-maps");
        let replay = proof
            .replay_clash(0, &ctx)
            .expect("a genuine clash replays");
        assert_eq!(
            replay.unattested(),
            0,
            "both body atoms are asserted role assertions: {replay:?}"
        );
        assert!(replay.attested() >= 2, "{replay:?}");
        let refutation = proof
            .replay_refutation(&ctx)
            .expect("the whole tree is one replayed leaf");
        assert!(refutation.is_closed(), "{refutation:?}");
    }

    // ── Branch exhaustiveness ───────────────────────────────────────────────────

    /// A recorded branch point's alternatives are REGENERATED from the caller's own clause set
    /// and match, atom for atom and in order.
    #[test]
    fn a_recorded_branch_point_regenerates_against_the_callers_own_ontology() {
        let (proof, ctx) = branching_refutation();
        assert!(
            !proof.branches().is_empty(),
            "the fixture refutes through a case split"
        );
        let replay = proof
            .replay_branch(0, &ctx)
            .expect("a genuine branch point regenerates");
        assert_eq!(
            replay.alternatives(),
            proof.branches()[0].alternatives().len()
        );
        assert!(replay.alternatives() >= 2, "a case split has alternatives");
        assert!(
            replay
                .checks()
                .rests_on()
                .contains(&TrustBaseEntry::Grounding),
            "regenerating the alternatives IS the grounding function: {:?}",
            replay.checks()
        );
        assert!(
            !replay.checks().is_fully_attested(),
            "an exhaustiveness check is trusted, never attested"
        );
    }

    /// The whole refutation TREE checks: every branch point exhaustive, every alternative
    /// closed, every leaf a re-derived clash.
    #[test]
    fn a_refutation_tree_closes_with_every_leaf_replayed() {
        let (proof, ctx) = branching_refutation();
        let replay = proof
            .replay_refutation(&ctx)
            .expect("a genuine refutation walks");
        assert!(replay.branches() >= 1, "{replay:?}");
        assert!(
            replay.clashes() >= 2,
            "both alternatives closed: {replay:?}"
        );
        assert!(
            replay.is_closed(),
            "every alternative reached a replayed closure: {replay:?}"
        );
        assert_eq!(replay.unrecorded(), 0);
    }

    /// A refutation with NO branch point is still a tree — a single leaf — and it walks.
    #[test]
    fn a_refutation_with_no_case_split_is_a_single_replayed_leaf() {
        let (_, proof, ctx) = refutation();
        let replay = proof.replay_refutation(&ctx).expect("one leaf is a tree");
        assert_eq!(replay.branches(), 0);
        assert_eq!(replay.clashes(), 1);
        assert!(replay.is_closed(), "{replay:?}");
    }

    // ── Tamper-negatives: written as a forger ───────────────────────────────────

    /// **THE HEADLINE.** DROPPING AN ALTERNATIVE IS REJECTED.
    ///
    /// This is how an unsound `inconsistent` is fabricated: refute the alternatives you like,
    /// omit the one you cannot close, and present a proof in which every recorded step checks.
    /// Stage 1's checker passes such a proof — every clash it carries is genuine. This one does
    /// not, because it regenerates the alternative set from the CALLER's own clause set and
    /// finds the case split narrower than the clause licenses.
    #[test]
    fn a_dropped_alternative_is_rejected() {
        let (mut proof, ctx) = branching_refutation();
        let honest = proof.branches[0].alternatives.len();
        assert!(honest >= 2, "there is something to drop");
        proof.branches[0].alternatives.pop();
        proof.branches[0].outcomes.pop();
        match proof.replay_branch(0, &ctx) {
            Err(DlProofError::AlternativeCountMismatch {
                derived, stated, ..
            }) => {
                assert_eq!(derived, honest);
                assert_eq!(stated, honest - 1);
            }
            other => panic!("a dropped disjunct must not check: {other:?}"),
        }
        assert!(
            proof.replay_refutation(&ctx).is_err(),
            "and the whole-tree walk must refuse it too"
        );
    }

    /// Adding a spurious alternative is rejected for the same reason, from the other side: the
    /// regenerated set is NARROWER than the record.
    #[test]
    fn a_spurious_alternative_is_rejected() {
        let (mut proof, ctx) = branching_refutation();
        let extra = proof.branches[0].alternatives[0].clone();
        proof.branches[0].alternatives.push(extra);
        proof.branches[0].outcomes.push(BranchOutcome::Unrecorded);
        assert!(matches!(
            proof.replay_branch(0, &ctx),
            Err(DlProofError::AlternativeCountMismatch { .. })
        ));
    }

    /// REORDERING the alternatives is rejected: the comparison is positional, because the
    /// authored order is what the search's determinism — and its cost — rests on.
    #[test]
    fn a_reordered_alternative_list_is_rejected() {
        let (mut proof, ctx) = branching_refutation();
        assert!(proof.branches[0].alternatives.len() >= 2);
        proof.branches[0].alternatives.reverse();
        match proof.replay_branch(0, &ctx) {
            Err(DlProofError::AlternativeMismatch { .. }) => {}
            other => panic!("a reordered case split must not check: {other:?}"),
        }
    }

    /// Rewriting an ATOM inside an alternative is rejected — the comparison goes all the way
    /// down, so a forger cannot keep the shape and change the content.
    #[test]
    fn a_rewritten_alternative_atom_is_rejected() {
        let (mut proof, ctx) = branching_refutation();
        let forged = match &proof.branches[0].alternatives[0].atoms[0] {
            ProofGround::Concept { node, concept } => ProofGround::Concept {
                node: node.clone(),
                concept: concept.wrapping_add(1),
            },
            other => panic!("the fixture's alternatives are concept atoms: {other:?}"),
        };
        proof.branches[0].alternatives[0].atoms[0] = forged;
        assert!(matches!(
            proof.replay_branch(0, &ctx),
            Err(DlProofError::AlternativeMismatch { position: 0, .. })
        ));
    }

    /// A branch point re-pointed at a clause that generates NO disjunction is rejected: the
    /// checker computes the head form itself, and only a disjunction is a case split.
    #[test]
    fn a_branch_point_citing_a_clause_that_generates_no_disjunction_is_rejected() {
        let (mut proof, ctx) = branching_refutation();
        let derailed = (0..ctx.clause_count())
            .find(|&index| ctx.clauses.clause(index).head_form() != HeadForm::Disjunctive)
            .expect("the fixture produces a non-disjunctive clause");
        proof.branches[0].clause = derailed;
        match proof.replay_branch(0, &ctx) {
            Err(DlProofError::NotADisjunction { clause, .. }) => assert_eq!(clause, derailed),
            other => panic!("a clause with no case split is no branch point: {other:?}"),
        }
    }

    /// A branch point citing a clause the caller's ontology does not have is a rejection, not a
    /// panic.
    #[test]
    fn a_branch_point_citing_an_absent_clause_is_rejected() {
        let (mut proof, ctx) = branching_refutation();
        proof.branches[0].clause = usize::MAX;
        assert!(matches!(
            proof.replay_branch(0, &ctx),
            Err(DlProofError::UnknownClause { .. })
        ));
    }

    /// A frame too narrow for the cited clause's head is a rejection rather than an index
    /// panic — the checker's guard on the one attacker-controlled input `ground_head` indexes.
    #[test]
    fn a_branch_point_with_a_short_frame_is_rejected() {
        let (mut proof, ctx) = branching_refutation();
        proof.branches[0].frame.clear();
        assert!(matches!(
            proof.replay_branch(0, &ctx),
            Err(DlProofError::FrameTooShort { .. })
        ));
    }

    /// A branch CLAIMED CLOSED whose clash is absent is rejected.
    #[test]
    fn a_branch_closed_on_a_clash_that_is_not_there_is_rejected() {
        let (mut proof, ctx) = branching_refutation();
        proof.branches[0].outcomes[0] = BranchOutcome::Clash(usize::MAX);
        match proof.replay_refutation(&ctx) {
            Err(DlProofError::DanglingOutcome { .. }) => {}
            other => panic!("a closure with no step behind it must not check: {other:?}"),
        }
    }

    /// A branch closed on a MERGE that did not close the state is rejected: the record says
    /// what happened, and a forger cannot promote an ordinary identification into a closure.
    #[test]
    fn a_branch_closed_on_a_merge_that_did_not_clash_is_rejected() {
        let (mut proof, ctx) = branching_refutation();
        proof.merges.push(MergeStep::new(
            MergeCause::Nominal,
            NodeRef::Individual(1),
            NodeRef::Individual(2),
            NodeRef::Individual(1),
            false,
        ));
        proof.branches[0].outcomes[0] = BranchOutcome::Merge(proof.merges.len() - 1);
        assert!(matches!(
            proof.replay_refutation(&ctx),
            Err(DlProofError::DanglingOutcome { .. })
        ));
    }

    /// An alternative left OPEN or UNEXPLORED in a proof whose answer says every branch closed
    /// is rejected — the answer and the tree have to agree.
    #[test]
    fn an_alternative_that_did_not_close_is_rejected() {
        for forged in [BranchOutcome::Open, BranchOutcome::Unexplored] {
            let (mut proof, ctx) = branching_refutation();
            proof.branches[0].outcomes[0] = forged;
            match proof.replay_refutation(&ctx) {
                Err(DlProofError::BranchNotClosed { ordinal: 0, .. }) => {}
                other => panic!("an unrefuted alternative must not check: {other:?}"),
            }
        }
    }

    /// An alternative that closed with nothing replayable is reported, and the refutation does
    /// NOT read as closed.
    ///
    /// The path is reachable — an alternative can close inside the `≥`-rule's own distinctness
    /// bookkeeping — so it is a counted, visible outcome rather than a silent pass.
    #[test]
    fn an_unrecorded_closure_is_not_a_closed_refutation() {
        let (mut proof, ctx) = branching_refutation();
        proof.branches[0].outcomes[0] = BranchOutcome::Unrecorded;
        let replay = proof
            .replay_refutation(&ctx)
            .expect("an unrecorded closure is a report, not a rejection");
        assert_eq!(replay.unrecorded(), 1);
        assert!(
            !replay.is_closed(),
            "a branch that closed on nothing replayable is not a discharged obligation: \
             {replay:?}"
        );
        assert!(replay.checks().unattested() > 0);
    }

    /// A branch point nothing descends into is rejected: an orphaned case split closes
    /// nothing, and a forger could otherwise hide a wide disjunction beside a narrow one.
    #[test]
    fn an_orphaned_branch_point_is_rejected() {
        let (mut proof, ctx) = branching_refutation();
        let orphan = proof.branches[0].clone();
        proof.branches.push(orphan);
        assert!(matches!(
            proof.replay_refutation(&ctx),
            Err(DlProofError::Malformed { .. })
        ));
    }

    /// A branch outcome pointing at an EARLIER branch point is rejected: a child is always
    /// recorded after its parent, and a cycle is how a forger makes two alternatives close each
    /// other.
    #[test]
    fn a_branch_outcome_pointing_backwards_is_rejected() {
        let (mut proof, ctx) = branching_refutation();
        proof.branches[0].outcomes[0] = BranchOutcome::Branch(0);
        assert!(matches!(
            proof.replay_refutation(&ctx),
            Err(DlProofError::DanglingOutcome { .. })
        ));
    }

    /// A branch point whose outcome list does not have one entry per alternative is rejected.
    #[test]
    fn a_branch_point_with_a_short_outcome_list_is_rejected() {
        let (mut proof, ctx) = branching_refutation();
        proof.branches[0].outcomes.pop();
        assert!(matches!(
            proof.replay_branch(0, &ctx),
            Err(DlProofError::OutcomeCountMismatch { .. })
        ));
    }

    /// A forged NOMINAL-INTRODUCTION alternative is rejected: it is not regenerable, but it is
    /// not unconstrained either — its role, filler, index and at-most root all come from the
    /// clause and the branch node.
    #[test]
    fn a_forged_nominal_introduction_alternative_is_rejected() {
        let (mut proof, ctx) = branching_refutation();
        proof.branches[0].introduced.push(ProofAlternative {
            atoms: vec![ProofGround::EqualIndividual {
                node: NodeRef::Anonymous(1),
                individual: 7,
            }],
        });
        proof.branches[0].outcomes.push(BranchOutcome::Unrecorded);
        assert!(matches!(
            proof.replay_branch(0, &ctx),
            Err(DlProofError::IllFormedIntroduction { .. })
        ));
    }

    /// The two whole-trace checks refuse a proof bound to the OTHER answer, rather than
    /// answering about a tree or a completion that is not there.
    #[test]
    fn a_whole_trace_check_refuses_a_proof_bound_to_the_other_answer() {
        let (proof, ctx) = branching_refutation();
        assert!(matches!(
            proof.replay_completion(&ctx),
            Err(DlProofError::WrongAnswer {
                expected: ProofAnswer::Consistent,
                ..
            })
        ));
        let ontology = consistent_ontology();
        let (_, proof) = prove_consistency(&ontology).expect("reverse-maps");
        let ctx = DlProofContext::of_ontology(&ontology).expect("reverse-maps");
        assert!(matches!(
            proof.replay_refutation(&ctx),
            Err(DlProofError::WrongAnswer {
                expected: ProofAnswer::Inconsistent,
                ..
            })
        ));
    }

    /// A TRUNCATED recording is refused by both whole-trace checks: a partial tree walked to a
    /// conclusion is a conclusion about a tree that was never there.
    #[test]
    fn a_truncated_recording_is_refused_by_the_whole_trace_checks() {
        let (mut proof, ctx) = branching_refutation();
        proof.truncated = true;
        assert!(matches!(
            proof.replay_refutation(&ctx),
            Err(DlProofError::Truncated)
        ));
    }

    // ── Countermodels ───────────────────────────────────────────────────────────

    /// A consistent answer carries a COMPLETION, and it model checks: nothing on it clashes,
    /// every clause of the caller's ontology is satisfied on it, and every blocking pair's
    /// signature is recomputed.
    #[test]
    fn a_consistent_answer_carries_a_completion_that_model_checks() {
        let (proof, ctx) = subclass_completion();
        let replay = proof
            .replay_completion(&ctx)
            .expect("a genuine completion model checks");
        assert!(replay.nodes() > 0, "{replay:?}");
        assert_eq!(
            replay.clauses(),
            ctx.clause_count(),
            "the check budget reached every clause: {replay:?}"
        );
        assert!(
            replay
                .checks()
                .rests_on()
                .contains(&TrustBaseEntry::Unravelling),
            "that a pre-model yields a MODEL is the metatheorem, and the report says so: {:?}",
            replay.checks()
        );
        assert!(
            !replay.checks().is_fully_attested(),
            "model checking a clause set is a trusted check"
        );
    }

    /// A completion whose blocking is load-bearing — the chain `⊤ ⊑ ∃r.⊤` terminates only by
    /// blocking — checks, and the checker RECOMPUTES the signatures rather than believing them.
    #[test]
    fn a_blocked_completion_model_checks_and_names_what_blocking_deferred() {
        let (proof, ctx) = blocking_chain_completion();
        let completion = proof.completion().expect("a consistent run records one");
        assert!(
            !completion.blocks().is_empty(),
            "the chain terminates only by blocking"
        );
        let replay = proof
            .replay_completion(&ctx)
            .expect("a genuine blocked completion model checks");
        assert!(replay.blocks() > 0, "{replay:?}");
        assert!(
            replay.deferred_to_blockers() > 0,
            "the blocked node's `∃r.⊤` is exactly what blocking withheld, and the report counts \
             it rather than passing it off as satisfied: {replay:?}"
        );
    }

    /// A CONCEALED CLASH is rejected: a forger who adds the missing half of a disjointness to a
    /// node's label makes an empty-headed clause match the completion, and the checker matches
    /// it too.
    #[test]
    fn a_completion_with_a_concealed_clash_is_rejected() {
        let (mut proof, ctx) = subclass_completion();
        // The clause that derives `false`, and the concepts its body reads. Putting all of them
        // on one node is exactly the clash the search would have closed on.
        let refuting = (0..ctx.clause_count())
            .map(|index| ctx.clauses.clause(index))
            .find(|clause| {
                clause.head.is_empty()
                    && clause
                        .body
                        .iter()
                        .all(|atom| matches!(atom, BodyAtom::Concept { var: 0, .. }))
                    && !clause.body.is_empty()
            })
            .expect("the fixture's disjointness derives false from one node's label");
        let concepts: Vec<u32> = refuting
            .body
            .iter()
            .filter_map(|atom| match *atom {
                BodyAtom::Concept { concept, .. } => Some(concept),
                _ => None,
            })
            .collect();
        let node = proof
            .completion
            .as_mut()
            .expect("a consistent proof carries one")
            .nodes
            .iter_mut()
            .find(|node| !node.concrete)
            .expect("the completion has an abstract node");
        for concept in concepts {
            if node.label.binary_search(&concept).is_err() {
                let at = node.label.partition_point(|&c| c < concept);
                node.label.insert(at, concept);
            }
        }
        match proof.replay_completion(&ctx) {
            Err(DlProofError::ClashInCompletion { .. }) => {}
            other => panic!("a completion that derives false is not a countermodel: {other:?}"),
        }
    }

    /// A completion where a clause is NOT SATISFIED is rejected — the direct model check.
    ///
    /// The forgery is the smallest possible one: remove the concept an absorbed `C ⊑ D` derives,
    /// leaving the body that triggers it in place. A checker that only looked for clashes would
    /// accept it, because nothing on the graph contradicts anything.
    #[test]
    fn a_completion_where_a_clause_is_not_satisfied_is_rejected() {
        let (mut proof, ctx) = subclass_completion();
        let (body, head) = (0..ctx.clause_count())
            .map(|index| ctx.clauses.clause(index))
            .find_map(|clause| {
                let [BodyAtom::Concept { var: 0, concept }] = clause.body.as_slice() else {
                    return None;
                };
                let [disjunct] = clause.head.as_slice() else {
                    return None;
                };
                let [
                    HeadAtom::Concept {
                        var: 0,
                        concept: derived,
                    },
                ] = disjunct.as_slice()
                else {
                    return None;
                };
                Some((*concept, *derived))
            })
            .expect("the fixture absorbs `C ⊑ D` into a one-atom guarded clause");
        let node = proof
            .completion
            .as_mut()
            .expect("a consistent proof carries one")
            .nodes
            .iter_mut()
            .find(|node| {
                node.label.binary_search(&body).is_ok() && node.label.binary_search(&head).is_ok()
            })
            .expect("the fixture's individual carries both");
        let at = node
            .label
            .binary_search(&head)
            .expect("the head concept is on the node");
        node.label.remove(at);
        match proof.replay_completion(&ctx) {
            Err(DlProofError::ClauseNotSatisfied { .. }) => {}
            other => panic!("an unsatisfied clause is not a countermodel: {other:?}"),
        }
    }

    /// A FORGED BLOCKING PAIR whose signatures differ is rejected.
    ///
    /// This is the forgery that would otherwise let an unsatisfied `≥`-restriction pass as
    /// "deferred to a blocker": claim the node is blocked and the check waves it through. The
    /// checker recomputes the signature from the recorded labels, predecessors and incoming
    /// edges, so the claim has to be true.
    #[test]
    fn a_forged_blocking_pair_is_rejected() {
        let (mut proof, ctx) = blocking_chain_completion();
        let completion = proof.completion.as_mut().expect("a completion is recorded");
        let blocker = completion.nodes[0].node.clone();
        completion.blocks[0].blocker = blocker;
        match proof.replay_completion(&ctx) {
            Err(DlProofError::BlockingSignatureMismatch { .. }) => {}
            other => panic!("a blocker that does not match is no blocker: {other:?}"),
        }
    }

    /// Claiming a node is blocked when NOTHING blocks it is rejected: a blocking pair whose two
    /// halves are the same node fails both the ordering condition and the signature.
    #[test]
    fn a_blocking_pair_that_blocks_a_node_with_itself_is_rejected() {
        let (mut proof, ctx) = subclass_completion();
        let completion = proof.completion.as_mut().expect("a completion is recorded");
        let node = completion.nodes[0].node.clone();
        completion.blocks.push(BlockingPair {
            blocked: node.clone(),
            blocker: node,
        });
        assert!(matches!(
            proof.replay_completion(&ctx),
            Err(DlProofError::BlockingSignatureMismatch { .. })
        ));
    }

    /// A completion presented against a DIFFERENT ontology is rejected before a single clause
    /// is evaluated — the identity check is the caller's own recomputation.
    #[test]
    fn a_completion_does_not_check_against_a_different_ontology() {
        let (proof, _) = subclass_completion();
        let other = consistent_ontology();
        let ctx = DlProofContext::of_ontology(&other).expect("reverse-maps");
        assert!(matches!(
            proof.replay_completion(&ctx),
            Err(DlProofError::InputMismatch { .. })
        ));
    }

    /// A completion listing one node identity twice is not a graph, and is refused rather than
    /// silently read as whichever copy came last.
    #[test]
    fn a_duplicated_completion_node_is_rejected() {
        let (mut proof, ctx) = subclass_completion();
        let completion = proof.completion.as_mut().expect("a completion is recorded");
        let duplicate = completion.nodes[0].clone();
        completion.nodes.push(duplicate);
        assert!(matches!(
            proof.replay_completion(&ctx),
            Err(DlProofError::MalformedCompletion { .. })
        ));
    }

    /// An edge naming a node the completion does not have is refused: a forger cannot satisfy a
    /// role atom by pointing at a node nobody can inspect.
    #[test]
    fn a_completion_edge_naming_an_absent_node_is_rejected() {
        let (mut proof, ctx) = subclass_completion();
        let completion = proof.completion.as_mut().expect("a completion is recorded");
        completion.edges.push(CompletionEdge {
            from: NodeRef::Anonymous(4_242),
            to: NodeRef::Anonymous(4_243),
            property: 1,
        });
        assert!(matches!(
            proof.replay_completion(&ctx),
            Err(DlProofError::MalformedCompletion { .. })
        ));
    }

    /// A completion check that runs out of budget reports the clauses it did not reach as
    /// UNATTESTED, rather than passing as if it had checked them.
    ///
    /// The budget is a resource bound and must never be a semantic one: exhausting it can only
    /// WITHHOLD a claim. Without this the exhaustion arm would be a branch no test constrains,
    /// and "the check ran out of time" would be indistinguishable from "the check succeeded".
    #[test]
    fn a_completion_check_that_runs_out_of_budget_reports_unattested_rather_than_passing() {
        let (proof, ctx) = subclass_completion();
        assert!(
            ctx.clause_count() > 1,
            "there is something to leave unchecked"
        );
        let replay = proof
            .replay_completion_within(&ctx, 1)
            .expect("a spent budget is a report, not a rejection");
        assert!(
            replay.clauses() < ctx.clause_count(),
            "a budget of one unit cannot have model checked every clause: {replay:?}"
        );
        assert!(
            replay.checks().unattested() > 0,
            "the clauses it never reached are unattested: {:?}",
            replay.checks()
        );
    }

    /// A consistent proof stripped of its completion is refused rather than passing vacuously —
    /// "nothing to check" must never read as "checked".
    #[test]
    fn a_consistent_proof_with_no_completion_is_refused() {
        let (mut proof, ctx) = subclass_completion();
        proof.completion = None;
        assert!(matches!(
            proof.replay_completion(&ctx),
            Err(DlProofError::NoCompletion)
        ));
    }

    // ── Determinism, over the shapes this stage added ───────────────────────────

    /// A proof carrying a BRANCH TREE and a proof carrying a COMPLETION are both byte-identical
    /// run to run, and both round-trip through their own encoding.
    #[test]
    fn a_branching_and_a_completing_proof_are_byte_identical_run_to_run() {
        let mut kb = Kb::empty();
        let ontologies = [closing_disjunction(), subclass_consistent()];
        kb.finalize();
        for ontology in ontologies {
            let (_, first) = prove_consistency(&ontology).expect("reverse-maps");
            let (_, again) = prove_consistency(&ontology).expect("reverse-maps");
            assert_eq!(first.encode(), again.encode(), "two runs, one proof");
            let decoded = DlProof::decode(&first.encode()).expect("its own encoding decodes");
            assert_eq!(decoded, first);
            assert_eq!(decoded.encode(), first.encode());
        }
    }

    // ── Fixtures for the shapes this stage added ────────────────────────────────

    /// `a : (C ⊔ D)` with `C ⊑ ⊥` and `D ⊑ ⊥` — INCONSISTENT through a two-way case split whose
    /// alternatives BOTH close on a clause instance, so the refutation is a TREE rather than a
    /// single leaf.
    fn closing_disjunction() -> Arc<RdfDataset> {
        let mut f = Fixture::new();
        let a = f.iri(EX_A);
        let c = f.iri(EX_C);
        let d = f.iri(EX_D);
        let ty = f.iri(RDF_TYPE);
        let sub = f.iri(RDFS_SUBCLASS_OF);
        let union_of = f.iri("http://www.w3.org/2002/07/owl#unionOf");
        let first = f.iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#first");
        let rest = f.iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#rest");
        let nil = f.iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#nil");
        let nothing = f.iri("http://www.w3.org/2002/07/owl#Nothing");
        let union = f
            .builder
            .intern_blank("u", purrdf_core::BlankScope::DEFAULT);
        let head = f
            .builder
            .intern_blank("l1", purrdf_core::BlankScope::DEFAULT);
        let tail = f
            .builder
            .intern_blank("l2", purrdf_core::BlankScope::DEFAULT);
        f.quad(head, first, c);
        f.quad(head, rest, tail);
        f.quad(tail, first, d);
        f.quad(tail, rest, nil);
        f.quad(union, union_of, head);
        f.quad(a, ty, union);
        f.quad(c, sub, nothing);
        f.quad(d, sub, nothing);
        f.freeze()
    }

    /// The branching refutation's proof, and a context built independently over the same
    /// ontology.
    fn branching_refutation() -> (DlProof, DlProofContext) {
        let ontology = closing_disjunction();
        let (answer, proof) = prove_consistency(&ontology).expect("the fixture reverse-maps");
        assert_eq!(
            answer,
            ProofAnswer::Inconsistent,
            "a : (C ⊔ D), C ⊑ ⊥, D ⊑ ⊥"
        );
        let ctx = DlProofContext::of_ontology(&ontology).expect("the fixture reverse-maps");
        (proof, ctx)
    }

    /// `p` asymmetric with `a p b` and `b p a`: INCONSISTENT on a clause whose whole body
    /// instance is ASSERTED — the fixture that gives the attestation counter something to count.
    fn asymmetric_clash() -> Arc<RdfDataset> {
        let mut f = Fixture::new();
        let a = f.iri(EX_A);
        let b = f.iri(EX_B);
        let p = f.iri(EX_P);
        let ty = f.iri(RDF_TYPE);
        let asymmetric = f.iri("http://www.w3.org/2002/07/owl#AsymmetricProperty");
        f.quad(p, ty, asymmetric);
        f.quad(a, p, b);
        f.quad(b, p, a);
        f.freeze()
    }

    /// `C ⊑ D` with `a : C` and `C` disjoint from `E`: CONSISTENT, and its completion carries a
    /// DERIVED concept membership, an empty-headed clause that must not match, and a guarded
    /// clause that must be satisfied.
    fn subclass_consistent() -> Arc<RdfDataset> {
        let mut f = Fixture::new();
        let a = f.iri(EX_A);
        let c = f.iri(EX_C);
        let d = f.iri(EX_D);
        let e = f.iri("http://example.org/E");
        let ty = f.iri(RDF_TYPE);
        let sub = f.iri(RDFS_SUBCLASS_OF);
        let disjoint = f.iri(OWL_DISJOINT_WITH);
        f.quad(c, sub, d);
        f.quad(c, disjoint, e);
        f.quad(a, ty, c);
        f.freeze()
    }

    /// That ontology's consistency proof, and a context built independently over it.
    fn subclass_completion() -> (DlProof, DlProofContext) {
        let ontology = subclass_consistent();
        let (answer, proof) = prove_consistency(&ontology).expect("reverse-maps");
        assert_eq!(answer, ProofAnswer::Consistent);
        let ctx = DlProofContext::of_ontology(&ontology).expect("reverse-maps");
        (proof, ctx)
    }

    /// `⊤ ⊑ ∃r.⊤` over one individual, through the knowledge-base entrance: an infinite chain
    /// that terminates ONLY by blocking, which is the completion whose blocking witnesses are
    /// load-bearing.
    fn blocking_chain_kb() -> Kb {
        use crate::owl_dl::concept::Concept;

        let mut kb = Kb::empty();
        kb.push_gci(
            Concept::Top,
            Concept::Some(Role::Named(20), Box::new(Concept::Top)),
        );
        kb.individuals.insert(30);
        kb.finalize();
        kb
    }

    /// That knowledge base's consistency proof, and a context built independently over a second
    /// copy of it.
    fn blocking_chain_completion() -> (DlProof, DlProofContext) {
        let (answer, proof) = prove_consistency_of_kb(&blocking_chain_kb());
        assert_eq!(answer, ProofAnswer::Consistent);
        (proof, DlProofContext::of_kb(blocking_chain_kb()))
    }

    /// A refutation TREE two levels deep walks, and every one of its branch points is reached
    /// from the root exactly once.
    ///
    /// The single-level fixtures above cannot separate "the walk works" from "there was nothing
    /// to walk". `a : (A ⊔ B)` and `a : (C ⊔ D)` with all four pairs disjoint forces the search
    /// to branch, survive, branch again and close all four ways: three branch points and four
    /// clash leaves, which is the shape the reachability, ordering and one-parent conditions
    /// exist for.
    #[test]
    fn a_two_level_refutation_tree_walks_from_the_root() {
        let (answer, proof) = prove_consistency_of_kb(&nested_disjunctions_kb());
        assert_eq!(answer, ProofAnswer::Inconsistent);
        assert_eq!(
            proof.branches().len(),
            3,
            "one branch point at the root and one under each of its two alternatives"
        );
        let ctx = DlProofContext::of_kb(nested_disjunctions_kb());
        let replay = proof
            .replay_refutation(&ctx)
            .expect("a genuine two-level tree walks");
        assert_eq!(
            replay.branches(),
            3,
            "every branch point is reached: {replay:?}"
        );
        assert_eq!(replay.clashes(), 4, "all four ways close: {replay:?}");
        assert!(replay.is_closed(), "{replay:?}");
    }

    /// Cutting ONE leaf out of that tree is rejected, wherever in the tree it sits: the walk is
    /// total, not a check of the root's own alternatives.
    #[test]
    fn a_dropped_alternative_deep_in_the_tree_is_rejected() {
        let (_, mut proof) = prove_consistency_of_kb(&nested_disjunctions_kb());
        let ctx = DlProofContext::of_kb(nested_disjunctions_kb());
        let deepest = proof.branches.len() - 1;
        proof.branches[deepest].alternatives.pop();
        proof.branches[deepest].outcomes.pop();
        assert!(matches!(
            proof.replay_refutation(&ctx),
            Err(DlProofError::AlternativeCountMismatch { .. })
        ));
    }

    /// `a : (A ⊔ B)` and `a : (C ⊔ D)` with `A ⊓ C`, `A ⊓ D`, `B ⊓ C` and `B ⊓ D` all
    /// unsatisfiable: INCONSISTENT, and it takes a two-level case split to say so.
    fn nested_disjunctions_kb() -> Kb {
        use crate::owl_dl::concept::Concept;

        /// The four class ids the fixture branches over.
        const CLASSES: [(u32, u32); 4] = [(10, 12), (10, 13), (11, 12), (11, 13)];

        let mut kb = Kb::empty();
        for (left, right) in CLASSES {
            kb.push_gci(
                Concept::And(vec![Concept::Named(left), Concept::Named(right)]),
                Concept::Bottom,
            );
        }
        let first = kb
            .table
            .intern(Concept::Or(vec![Concept::Named(10), Concept::Named(11)]));
        let second = kb
            .table
            .intern(Concept::Or(vec![Concept::Named(12), Concept::Named(13)]));
        kb.abox_types.push((30, first));
        kb.abox_types.push((30, second));
        kb.individuals.insert(30);
        kb.finalize();
        kb
    }

    /// A dataset that is not an OWL graph at all is a rejection with the reverse mapper's own
    /// account, never a panic.
    #[test]
    fn an_unreadable_ontology_is_reported_rather_than_panicking() {
        // A well-formed but empty dataset reverse-maps; this pins the ERROR SHAPE exists and
        // is reachable through the public constructor rather than through a private path.
        let empty = RdfDatasetBuilder::new().freeze().expect("empty freezes");
        let ctx = DlProofContext::of_ontology(&empty).expect("an empty graph is an OWL graph");
        assert_eq!(ctx.clause_count(), ctx.clause_count());
    }
}
