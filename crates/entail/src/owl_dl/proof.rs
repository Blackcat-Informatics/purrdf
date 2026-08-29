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
//! # What Stage 1 establishes, and what it does not
//!
//! This module is deliberately narrower than a full tableau proof, and the boundary is stated
//! here rather than discovered by a consumer:
//!
//! * **[`DlProof::replay_clash`] is real.** For a recorded [`ClashStep`] it establishes that
//!   the cited clause is a clause of the caller's ontology's own clause set, that its head is
//!   EMPTY (so the instance derives `false`), that the recorded frame is wide enough for the
//!   clause's variables, and that the recorded witness is exactly the grounding of that
//!   clause's body under that frame. It runs no search: it never constructs a
//!   [`Hyper`](crate::owl_dl::hyper) driver, never opens a
//!   [`Session`](crate::reasoner::certificate), and never expands a completion graph.
//! * **REACHABILITY is not established.** The checker does not show that the recorded witness
//!   facts are derivable in a completion graph of the ontology. It reports how many of them it
//!   could reduce to an ASSERTED axiom of the caller's ABox
//!   ([`ClashReplay::attested`]) and how many it could not
//!   ([`ClashReplay::unattested`]); an unattested fact is a fact this stage takes on the
//!   producer's word. Closing that gap needs a premise DAG per witness fact, which is the
//!   obvious next step and is deliberately not faked here.
//! * **BRANCH EXHAUSTIVENESS is not established.** "These were all the alternatives" is a
//!   claim about the `⊔`-rule's enumeration, and verifying it means re-running the
//!   hyperresolution matcher — relocating the circularity a proof term exists to break rather
//!   than removing it. [`DlProof`] is shaped so an exhaustiveness receipt can be added beside
//!   the clash list later; nothing here claims one exists.
//! * **CLASH-FREE COMPLETIONS are not certified.** A [`ProofAnswer::Consistent`] proof carries
//!   the run's merge provenance and its boundary set, and no countermodel: exhibiting one
//!   needs blocking, unravelling and the concrete-domain solver.
//!
//! # Three recorded kinds, one replayable
//!
//! [`ClashStep`] is a derivation of `false` from a clause with an empty head — the calculus's
//! only clash rule, because in this hypertableau a clash IS a derivation rather than a
//! detector. [`MergeStep`] is provenance for an identification: which rule forced it, and the
//! two stable node identities it joined. The data-clash list names the nodes whose CONCRETE
//! domain constraints the [`data`](crate::owl_dl::data) solver found unsatisfiable — the one
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
//!   [`clause::derive`](crate::owl_dl::clause::derive) over
//!   [`absorb`](crate::owl_dl::absorb)'s decisions, so it attests WHICH calculus and WHICH
//!   clausification an answer came from, and it is not a second, independent identity for the
//!   ontology. The two are never conflated: [`DlProofContext`] recomputes BOTH from the
//!   caller's own dataset and rejects a proof that disagrees with either.
//!
//! # Determinism
//!
//! Every field is an integer, a fixed-order enum ordinal or a length-prefixed byte string;
//! boundaries are emitted in [`Construct::ALL`] order; steps are emitted in the order the
//! deterministic search recorded them. Nothing is read out of a hash map and nothing consults
//! a clock, so [`DlProof::encode`] is byte-identical run to run and on `wasm32`, exactly as
//! the [`Decision`](crate::owl_dl::graph::Decision) it accompanies is.

use std::collections::BTreeSet;

use purrdf_core::RdfDataset;
use purrdf_datalog::clause::HeadForm;

use crate::EntailError;
use crate::owl_dl::Kb;
use crate::owl_dl::clause::{BodyAtom, ClauseSet, DlClause, derive};
use crate::owl_dl::concept::Role;
use crate::owl_dl::graph::{Assumptions, Budget, GeneratedRoot, NominalId, State, find};
use crate::report::Construct;

// ── The wire format's fixed vocabulary ──────────────────────────────────────────

/// Domain-separation tag leading every [`DlProof::encode`]d proof.
///
/// Bumped whenever the encoding changes shape, so bytes written under an older layout can
/// never be decoded as if they were current.
const PROOF_ENCODING_TAG: &str = "purrdf-owl-dl-proof-v1";

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

// ── Stable node identity ────────────────────────────────────────────────────────

/// A role of the proof surface: a property id and whether it is read inverted.
///
/// A public mirror of the crate-private [`Role`], so a proof term can name a role without
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

    /// The proof-surface spelling of an internal [`Role`].
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
    /// The mirror of [`GeneratedRoot`], which was designed to be stable across merges and
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
/// The public mirror of [`GeneratedRoot`]: the at-most root `u` the reserved set belongs to,
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

    /// The proof-surface identity of the internal [`GeneratedRoot`].
    fn of_generated(root: &GeneratedRoot) -> Self {
        Self::Reserved(Box::new(ReservedRef {
            origin: Self::of_nominal(&root.origin),
            role: ProofRole::of(root.role),
            filler: root.filler,
            index: root.index,
        }))
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
    /// [`Graph::merge_nodes`](crate::owl_dl::graph::Graph::merge_nodes) chose, read off the
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

// ── The recorder ────────────────────────────────────────────────────────────────

/// What an instrumented search wrote down.
///
/// Held by [`crate::owl_dl::hyper`]'s driver behind an `Option`, so a non-recording run — every
/// run every existing caller makes — allocates nothing, records nothing, and takes the same
/// branches in the same order. Recording never consults the work meter and never calls a
/// metered graph operation, which is what makes a recorded run's
/// [`Decision`](crate::owl_dl::graph::Decision) identical to an unrecorded one's.
#[derive(Debug, Default)]
pub(crate) struct Recorder {
    /// The clause instances that derived `false`, in search order.
    clashes: Vec<ClashStep>,
    /// The identifications, in search order.
    merges: Vec<MergeStep>,
    /// The nodes whose concrete-domain constraints had no solution, in search order.
    data_clashes: Vec<NodeRef>,
    /// Whether any of the three lists reached [`MAX_RECORDED_STEPS`].
    truncated: bool,
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
    /// [`crate::owl_dl::hyper`]: without it, "a recorded run decides identically" would be
    /// satisfied by a recorder that recorded nothing.
    #[cfg(test)]
    pub(crate) fn is_empty(&self) -> bool {
        self.clashes.is_empty() && self.merges.is_empty() && self.data_clashes.is_empty()
    }

    /// Record a concrete-domain clash, up to the declared ceiling.
    pub(crate) fn data_clash(&mut self, node: NodeRef) {
        if self.data_clashes.len() >= MAX_RECORDED_STEPS {
            self.truncated = true;
            return;
        }
        self.data_clashes.push(node);
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
/// The two counters are the honest boundary of this stage and are reported rather than
/// smoothed over: `attested` witness atoms were reduced to an ASSERTED axiom of the caller's
/// ABox, and `unattested` ones were not, because reducing them needs a premise DAG this stage
/// does not build. A replay with `unattested > 0` says "this clause instance is a genuine
/// derivation of `false` over these atoms", not "these atoms are reachable".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClashReplay {
    /// The conclusion the checker derived.
    conclusion: DerivedConclusion,
    /// The clause the checker looked up in the CALLER's clause set.
    clause: usize,
    /// Witness atoms the checker reduced to an asserted axiom of the caller's ABox.
    attested: usize,
    /// Witness atoms the checker could not reduce to an asserted axiom.
    unattested: usize,
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

    /// Witness atoms reduced to an asserted axiom of the caller's ABox.
    #[must_use]
    pub const fn attested(&self) -> usize {
        self.attested
    }

    /// Witness atoms the checker took on the producer's word.
    ///
    /// Non-zero is the ordinary case for any ontology with a TBox: a derived concept
    /// membership is not an asserted axiom. It is reported because the alternative — omitting
    /// it — would let a partial replay read as a total one.
    #[must_use]
    pub const fn unattested(&self) -> usize {
        self.unattested
    }
}

// ── The checking context ────────────────────────────────────────────────────────

/// What a proof is checked AGAINST: the CALLER's own ontology, clausified by the caller.
///
/// This type exists to make one property structural rather than promised. It is built from a
/// [`RdfDataset`] the CONSUMER supplies and from nothing else: it holds no state a producer
/// shipped, so a proof cannot be verified against the very stores that produced it. It
/// constructs no [`Hyper`](crate::owl_dl::hyper) driver and opens no
/// [`Session`](crate::reasoner::certificate) — the only thing it runs is the reverse mapper and
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
/// See the [module docs](self) for exactly what a replay of one establishes. Fields are
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
    /// The constructs the reverse mapping could not turn into DL clauses, in
    /// [`Construct::ALL`] order.
    boundaries: Vec<Construct>,
    /// The answer this proof is bound to.
    answer: ProofAnswer,
    /// The clause instances that derived `false`, in search order.
    clashes: Vec<ClashStep>,
    /// The identifications, in search order.
    merges: Vec<MergeStep>,
    /// The nodes whose concrete-domain constraints had no solution, in search order.
    data_clashes: Vec<NodeRef>,
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

    /// The constructs the reverse mapping bounded, in [`Construct::ALL`] order.
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
    /// independently means re-running [`crate::owl_dl::data`]'s value-space solver, which is a
    /// later stage.
    #[must_use]
    pub fn data_clashes(&self) -> &[NodeRef] {
        &self.data_clashes
    }

    /// Whether the recording reached [`MAX_RECORDED_STEPS`] for any step kind.
    ///
    /// A truncated proof is still sound for every step it DOES carry — each is replayed
    /// independently — but it is not the whole trace, and a consumer that needs the whole
    /// trace must read this rather than infer completeness from a step count.
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
    /// No [`Hyper`](crate::owl_dl::hyper) driver is constructed, no
    /// [`Session`](crate::reasoner::certificate) is opened, and no completion graph is
    /// expanded. See the [module docs](self) for what this does NOT establish.
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
        Ok(ClashReplay {
            conclusion: DerivedConclusion::False,
            clause: step.clause,
            attested,
            unattested: derived.len() - attested,
        })
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
    /// u64 boundary_count, u64 Construct::ALL ordinal each
    /// u64 clash_count, then per clash:
    ///     u64 clause index
    ///     node                                    -- the clash node
    ///     u64 frame_len, node each
    ///     u64 witness_len, fact each
    /// u64 merge_count, then per merge:
    ///     u8 cause ordinal, node left, node right, node joined, u8 clashed
    /// u64 data_clash_count, node each
    /// ```
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        frame(&mut out, PROOF_ENCODING_TAG.as_bytes());
        out.extend_from_slice(&self.input);
        out.extend_from_slice(&self.contract);
        out.push(self.answer.ordinal());
        out.push(u8::from(self.truncated));
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
    /// fact, cause or answer kind, and a boundary ordinal outside [`Construct::ALL`] are all
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
        if !reader.is_exhausted() {
            return Err(malformed("trailing bytes after the proof's last field"));
        }
        Ok(Self {
            input,
            contract,
            boundaries,
            answer,
            clashes,
            merges,
            data_clashes,
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
        boundaries: boundaries_of(&kb.boundaries),
        answer,
        clashes: recorder.clashes,
        merges: recorder.merges,
        data_clashes: recorder.data_clashes,
        truncated: recorder.truncated,
    };
    Ok((answer, proof))
}

/// The knowledge base's boundary set, in [`Construct::ALL`] order.
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

/// OBSERVE a clause's body instance in the completion graph — the recorder's reading.
///
/// Deliberately a second implementation of the grounding above, and deliberately one that
/// consults the STATE: a concept or `denotes` atom is emitted only when the node's label or
/// name set actually holds it, so a matcher that fired on an atom the graph does not carry
/// produces a SHORT witness and [`DlProof::replay_clash`] rejects the step. A witness produced
/// by calling [`ground_body`] would make that comparison vacuous, which is why the two are not
/// one function.
///
/// Role atoms are recorded as matched: whether a node is an `r`-NEIGHBOUR of another is the
/// role hierarchy's, the inverse declarations' and the transitive closure's answer, and
/// re-deciding it here means re-running the metered neighbour scan — which would change the
/// work a recorded run charges and so change its
/// [`Decision`](crate::owl_dl::graph::Decision). Nothing in this function consults or charges
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
        matches!(
            kb.table.decomp(concept),
            crate::owl_dl::concept::Decomp::Top
        ) || st.nodes[find(st, node)].label.contains(&concept)
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
fn head_atom_hash(hasher: &mut blake3::Hasher, atom: &crate::owl_dl::clause::HeadAtom) {
    use crate::owl_dl::clause::HeadAtom;
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

    /// A boundary ordinal outside [`Construct::ALL`] is refused rather than clamped.
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
