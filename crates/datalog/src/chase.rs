// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The **restricted existential chase**: a value-inventing fixpoint over the DL-clause IR,
//! with a computed termination certificate and blank-node Skolem witnesses.
//!
//! [`crate::seminaive`] computes the least model of a set of DEFINITE clauses. It refuses an
//! existential head by name, because a semi-naive evaluator has no witness to derive. This
//! module is the consumer that head form was represented for: given
//! `A(x) → ∃y. (r(x, y) ∧ C(y))` it invents a witness for `y`, asserts BOTH head atoms over
//! it, and runs to a fixpoint.
//!
//! # Three properties this module is built around
//!
//! **The chase is RESTRICTED, not oblivious.** Before a clause fires, its head is run as a
//! conjunctive query with the existentials FREE, seeded with the frontier binding. If any
//! solution exists the firing is SKIPPED — the obligation is already witnessed. Without
//! that check the fixpoint does not converge even for a trivially terminating program:
//! `A(a)` would mint a witness, the next round would see `A(a)` again and mint another, and
//! so on forever. The check is what makes "no new fact this round" a real fixpoint rather
//! than an accident of witness naming.
//!
//! **A witness is a BLANK NODE, never a minted IRI.** PurRDF mints no vocabulary, so it
//! does not mint an individual either. An invented null is exactly what RDF already has a
//! term kind for — a blank node — and its surface is the repository's blank-node surface
//! `_:<scope>.<label>` in the default scope `0`. There is no IRI-namespace parameter
//! anywhere in this module's API, because there is no IRI to name.
//!
//! **Termination is COMPUTED, never demanded.** [`certify`] is a pure function of the
//! clause set: it either proves the program weakly acyclic or names the existential edges
//! that lie inside a cycle. [`chase`] runs the certificate first and refuses an uncertified
//! program by name ([`ChaseError::NonTerminating`]) rather than looping. A caller supplies
//! no budget, no depth and no acyclicity assertion — see the crate docs for why a
//! caller-supplied budget is not offered here either.
//!
//! # Why the position graph is refined by constants
//!
//! Weak acyclicity is a reachability property of the *position dependency graph*: nodes are
//! argument positions, a normal edge runs from a frontier variable's body position to its
//! head position, a special edge runs from a frontier body position to an EXISTENTIAL head
//! position, and the program is weakly acyclic when no special edge lies inside a cycle.
//!
//! Taken over bare `(predicate, slot)` nodes that analysis is far too coarse for RDF,
//! because a description-logic concept name is not a predicate here — it is a CONSTANT in
//! the object slot of a `type` atom. Consider the plainly terminating restriction
//! `C ⊑ ∃p.D`:
//!
//! ```text
//! type(?x, <C>)  →  ∃?y. ( p(?x, ?y) ∧ type(?y, <D>) )
//! ```
//!
//! Unrefined, `?x`'s body position and `?y`'s head position are the SAME node
//! `type[subject]`, so the special edge `type[subject] → type[subject]` is a self-loop and
//! the analysis reports a terminating program as unbounded. Every `C ⊑ ∃p.D` axiom in every
//! ontology would be refused.
//!
//! A position is therefore refined by the constants co-occurring in the atom's OTHER slots.
//! `?x` occurs at `[s=?, p=<type>, o=<C>, g=…]` and `?y` at `[s=?, p=<type>, o=<D>, g=…]`,
//! which are different nodes, there is no cycle, and the axiom certifies — which is exactly
//! what the test `a_constant_refined_restriction_is_weakly_acyclic` pins.
//!
//! The refinement is SOUND because it only ever SPLITS a position by a constant that
//! genuinely partitions which body atoms can consume a null: a null typed `<D>` sits at the
//! object-constant-`<D>` refinement and cannot be read by a body atom that demands `<C>`
//! there. Where a slot holds a VARIABLE the refinement is the wildcard `*`, and any two
//! refinements of one slot that are compatible — equal, or differing only where one side is
//! a wildcard — are conservatively connected in BOTH directions, so reachability is
//! over-approximated and never under-approximated. A wildcard PREDICATE is the same rule
//! applied to the predicate slot: a rule that quantifies over the predicate is connected to
//! every constant-predicate refinement of the same slot, because at run time it can consume
//! and produce any of them.
//!
//! # Every atom is an arity-4 quad, and the predicate is data
//!
//! A [`ClauseAtom`] is `triple(?s, ?p, ?o, ?g)` and all four positions may be variables, so
//! a position node is keyed by all four slots rather than by a relation symbol: `slot` says
//! which of the four the variable occupies, and the other three carry the refinement. That
//! makes the predicate position an ordinary analysed slot instead of a special case, which
//! is what an OWL 2 RL meta-rule needs.
//!
//! # What this module REFUSES, by name
//!
//! Each of these is a permanent refusal with a documented reason, never a stub:
//!
//! * [`ChaseError::NonTerminating`] — the analysis found an existential edge in a cycle.
//! * [`ChaseError::DisjunctiveExistentialHead`] — `∃ȳ. (C₁ ∨ … ∨ Cₘ)` with `m > 1`. A
//!   disjunctive head needs a hypertableau case split; there is none here, and PICKING a
//!   disjunct would assert something the program does not entail.
//! * [`ChaseError::DisjunctiveHead`] — `m > 1` with no existential, for the same reason.
//! * [`ChaseError::InconsistencyClause`] — `body → false` derives nothing and asserts its
//!   body is unsatisfiable; a chase has no consistency verdict to return, so firing it is
//!   meaningless and ignoring it would silently answer a different question.
//! * [`ChaseError::NegatedBodyAtom`] — negation as failure has no meaning under a chase
//!   that invents values: a witness minted in a later round can falsify a NAF test an
//!   earlier round already used, so the fixpoint would not be a model of the program. There
//!   is no stratifier here to make the test decidable.
//! * [`ChaseError::UnboundHeadVariable`] — a head variable that is neither existentially
//!   quantified nor bound by the body cannot be grounded without fabricating a term.
//! * [`ChaseError::BudgetExhausted`] — one of the crate's three fixed ceilings was passed.
//!
//! The forms that DO fire are the atomic head (a Datalog rule), the conjunctive head
//! (`m = 1`, several conjuncts, no existential) and the single-disjunct existential head.
//! The first two are simply zero-existential conjunctive heads, so ONE loop handles all
//! three: an atomic clause fires exactly as it would in the semi-naive evaluator, and a
//! conjunctive head asserts its conjuncts atomically. Firing a conjunctive head is sound
//! precisely because it is not a disjunction — `→ p ∧ q` entails both conjuncts.
//!
//! # Determinism
//!
//! Every map and set that reaches an output is a `BTreeMap`/`BTreeSet`. A round's pending
//! facts are accumulated in a `BTreeMap<Fact, clause index>`, so they are committed in total
//! lexical `(subject, predicate, object, graph)` order regardless of which clause produced
//! them or in which order the join enumerated rows, and a fact several clauses derive is
//! credited to the lowest authored index rather than to whichever ran first. A witness
//! surface is a content digest of its address, so it does not depend on mint order. There is
//! no wall clock, no RNG, no filesystem and no thread: the module is
//! `wasm32-unknown-unknown`-clean.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::clause::{ClauseAtom, ClauseTerm, DlClause, HeadDisjunct, HeadForm};
use crate::cursor::LendingIterator;
use crate::id::TermId;
use crate::plan::{
    ATOM_ARITY, POSITION_GRAPH, POSITION_OBJECT, POSITION_PREDICATE, POSITION_SUBJECT,
};
use crate::seminaive::{
    BudgetReport, BudgetResource, MAX_JOIN_STEPS, MAX_STORED_FACTS, MAX_TERM_ARENA_BYTES,
};
use crate::store::{Bound, Fact, RelationStore};

// ── The position dependency graph ───────────────────────────────────────────────

/// The class refinement one slot contributes to a position node.
///
/// A constant slot refines the position by its exact lexical surface — the same bytes the
/// store interns — so two positions differing only in that constant are different nodes. A
/// variable slot contributes the wildcard, which is compatible with every constant.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum ClassKey {
    /// The slot holds this constant lexical surface.
    Const(String),
    /// The slot holds a variable, so it matches any term.
    Wildcard,
}

impl ClassKey {
    /// The refinement `term` contributes.
    fn of(term: &ClauseTerm) -> Self {
        term.surface().map_or(Self::Wildcard, Self::Const)
    }

    /// Whether these two refinements can denote the same term at run time.
    ///
    /// Two constants are compatible only when they are byte-equal; a wildcard is compatible
    /// with everything. This is the relation the conservative subsumption edges are built
    /// from, so it must over-approximate — saying "compatible" too often costs precision,
    /// saying it too rarely would let a non-terminating program certify.
    fn compatible(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Const(left), Self::Const(right)) => left == right,
            _ => true,
        }
    }

    /// This refinement, rendered for a diagnostic.
    fn render(&self) -> String {
        match self {
            // The default graph's surface is the EMPTY surface, which would render as
            // nothing at all; name it instead of printing a blank.
            Self::Const(surface) if surface.is_empty() => "(default graph)".to_owned(),
            Self::Const(surface) => surface.clone(),
            Self::Wildcard => "*".to_owned(),
        }
    }
}

/// Which of an atom's four slots a variable occupies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Slot {
    /// The subject slot.
    Subject,
    /// The predicate slot — an ordinary analysed slot here, because the predicate is data.
    Predicate,
    /// The object slot.
    Object,
    /// The graph slot.
    Graph,
}

impl Slot {
    /// The four slots, in the atom's own `(subject, predicate, object, graph)` order.
    const ALL: [Self; ATOM_ARITY] = [Self::Subject, Self::Predicate, Self::Object, Self::Graph];

    /// This slot's index into [`ClauseAtom::terms`].
    const fn index(self) -> usize {
        match self {
            Self::Subject => POSITION_SUBJECT,
            Self::Predicate => POSITION_PREDICATE,
            Self::Object => POSITION_OBJECT,
            Self::Graph => POSITION_GRAPH,
        }
    }

    /// This slot's one-letter name, for a diagnostic.
    const fn name(self) -> &'static str {
        match self {
            Self::Subject => "s",
            Self::Predicate => "p",
            Self::Object => "o",
            Self::Graph => "g",
        }
    }
}

/// One node of the position dependency graph: a slot of an atom, refined by the constants
/// co-occurring in that atom's other three slots.
///
/// The refinement at the node's own slot is unused (it is the position being described), so
/// it is held as [`ClassKey::Wildcard`] and rendered as `?`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Position {
    /// Which slot the variable occupies.
    slot: Slot,
    /// The refinement contributed by each slot; the entry at `slot` is unused.
    keys: [ClassKey; ATOM_ARITY],
}

impl Position {
    /// The position of `atom`'s `slot`, refined by the atom's other three slots.
    fn of(atom: &ClauseAtom, slot: Slot) -> Self {
        let terms = atom.terms();
        let keys = std::array::from_fn(|index| {
            if index == slot.index() {
                ClassKey::Wildcard
            } else {
                ClassKey::of(terms[index])
            }
        });
        Self { slot, keys }
    }

    /// Whether a term flowing into `self` could also flow into `other`.
    ///
    /// Requires the same slot and pairwise-compatible refinements. Conservative by
    /// construction: a wildcard agrees with every constant, so the subsumption edges this
    /// drives can only ADD reachability.
    fn compatible(&self, other: &Self) -> bool {
        self.slot == other.slot
            && self
                .keys
                .iter()
                .zip(&other.keys)
                .all(|(left, right)| left.compatible(right))
    }

    /// This position, rendered for a diagnostic — for example
    /// `[s=?, p=<https://example.org/type>, o=<https://example.org/C>, g=(default graph)]`.
    fn render(&self) -> String {
        let rendered: Vec<String> = Slot::ALL
            .iter()
            .enumerate()
            .map(|(index, slot)| {
                let key = if index == self.slot.index() {
                    "?".to_owned()
                } else {
                    self.keys[index].render()
                };
                format!("{}={key}", slot.name())
            })
            .collect();
        format!("[{}]", rendered.join(", "))
    }
}

/// Every refined position at which `variable` occurs across `atoms`.
///
/// A variable occurring twice in one atom yields two positions, and a variable occurring in
/// two atoms yields one per atom — the analysis is about slots, not occurrences of a name.
fn refined_positions<'a>(
    atoms: impl IntoIterator<Item = &'a ClauseAtom>,
    variable: &str,
) -> Vec<Position> {
    let mut out = Vec::new();
    for atom in atoms {
        let terms = atom.terms();
        for slot in Slot::ALL {
            if terms[slot.index()].variable() == Some(variable) {
                out.push(Position::of(atom, slot));
            }
        }
    }
    out
}

/// Connect every pair of COMPATIBLE distinct positions, in both directions.
///
/// This is the conservative over-approximation the constant refinement owes: a wildcard
/// refinement can denote any constant at run time, so a null reaching the wildcard node can
/// reach every constant node of the same slot and vice versa. It subsumes both the
/// wildcard-class case (`p(?x, ?y)` versus `p(?x, <C>)`) and the wildcard-PREDICATE case (a
/// rule that quantifies over the predicate is joined to every constant-predicate position
/// of the same slot).
///
/// Distinct compatible positions always differ at some slot where one side is a wildcard —
/// two positions whose refinements are all byte-equal constants ARE the same node — so this
/// never adds a self-edge that the program did not already have.
fn add_subsumption(adjacency: &mut BTreeMap<Position, BTreeSet<Position>>, nodes: &[Position]) {
    for (index, left) in nodes.iter().enumerate() {
        for right in &nodes[index + 1..] {
            if left.compatible(right) {
                adjacency
                    .entry(left.clone())
                    .or_default()
                    .insert(right.clone());
                adjacency
                    .entry(right.clone())
                    .or_default()
                    .insert(left.clone());
            }
        }
    }
}

/// Whether `to` is reachable from `from` over at least ONE edge.
///
/// "At least one" is the point: a special edge `(u, v)` is a violation exactly when `v`
/// reaches `u`, and for a self-loop `u == v` that means the edge `u → u` itself must count.
/// A depth-first walk over `BTreeSet` adjacency, so the answer is a pure function of the
/// graph.
fn reaches(
    adjacency: &BTreeMap<Position, BTreeSet<Position>>,
    from: &Position,
    to: &Position,
) -> bool {
    let mut stack: Vec<&Position> = adjacency.get(from).into_iter().flatten().collect();
    let mut seen: BTreeSet<&Position> = BTreeSet::new();
    while let Some(node) = stack.pop() {
        if node == to {
            return true;
        }
        if !seen.insert(node) {
            continue;
        }
        if let Some(successors) = adjacency.get(node) {
            stack.extend(successors.iter());
        }
    }
    false
}

/// What the termination analysis proved about a clause set.
///
/// Computed by [`certify`] and carried on [`ChaseOutcome`] so a caller can put the
/// certificate in a report next to the facts it justifies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChaseTermination {
    /// No existential edge of the position dependency graph lies in a cycle.
    ///
    /// The chase over such a program terminates: every null is created at a position from
    /// which no path returns to a position that creates nulls, so the number of nulls is
    /// bounded by the program rather than by the data.
    WeaklyAcyclic {
        /// How many distinct refined positions the graph holds — the proof's size.
        positions: usize,
        /// How many distinct existential edges were checked.
        existential_edges: usize,
    },
    /// The analysis found an existential edge inside a cycle.
    ///
    /// This does NOT prove the program non-terminating — weak acyclicity is sufficient, not
    /// necessary — but it is the only honest answer this analysis can give, and [`chase`]
    /// refuses rather than risk a fixpoint that never closes.
    Unbounded {
        /// One diagnostic per offending edge, sorted and deduplicated.
        violations: Vec<String>,
    },
}

impl ChaseTermination {
    /// Whether the analysis certified termination.
    pub fn is_certified(&self) -> bool {
        matches!(self, Self::WeaklyAcyclic { .. })
    }
}

impl fmt::Display for ChaseTermination {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WeaklyAcyclic {
                positions,
                existential_edges,
            } => write!(
                f,
                "weakly acyclic: {positions} refined position(s), {existential_edges} \
                 existential edge(s), none in a cycle"
            ),
            Self::Unbounded { violations } => write!(
                f,
                "not weakly acyclic: {} existential edge(s) lie in a cycle",
                violations.len()
            ),
        }
    }
}

/// Decide whether the restricted chase over `program` is certified to terminate.
///
/// A PURE function of the clause set: no store, no caller input, no budget knob and no
/// tunable. Two callers holding the same program always get the same certificate, and a
/// caller cannot assert termination it has not proved.
///
/// The analysis is constant-refined weak acyclicity over the arity-4 position dependency
/// graph — see the [module docs](self) for the node shape, the worked example that motivates
/// the refinement, and the soundness argument for it. Reachability is over-approximated
/// (compatible refinements of one slot are connected in both directions), so a program that
/// does not terminate can never be certified; the price is that a terminating program may
/// occasionally be reported [`Unbounded`](ChaseTermination::Unbounded).
///
/// A program with no existential head has no special edges at all and is therefore
/// trivially weakly acyclic, however cyclic its Datalog recursion is — transitive closure
/// invents nothing.
pub fn certify(program: &[DlClause]) -> ChaseTermination {
    let mut adjacency: BTreeMap<Position, BTreeSet<Position>> = BTreeMap::new();
    let mut special: BTreeSet<(Position, Position)> = BTreeSet::new();
    let mut nodes: BTreeSet<Position> = BTreeSet::new();

    for clause in program {
        let frontier = clause.frontier_variables();

        // Normal edges: a frontier variable's body positions flow to its head positions.
        // Every body atom counts, negated ones included: a negated atom binds nothing, so
        // the extra edges only over-approximate, which is the safe direction.
        for variable in &frontier {
            let body_positions = refined_positions(clause.body(), variable);
            let head_positions = refined_positions(clause.head_atoms(), variable);
            for from in &body_positions {
                for to in &head_positions {
                    nodes.insert(from.clone());
                    nodes.insert(to.clone());
                    adjacency
                        .entry(from.clone())
                        .or_default()
                        .insert(to.clone());
                }
            }
        }

        // Special edges: every frontier body position flows to every existential head
        // position, because the invented null is a function of the whole frontier binding.
        if clause.existentials().is_empty() {
            continue;
        }
        let mut frontier_positions: Vec<Position> = Vec::new();
        for variable in &frontier {
            frontier_positions.extend(refined_positions(clause.body(), variable));
        }
        for existential in clause.existentials() {
            for to in refined_positions(clause.head_atoms(), existential) {
                for from in &frontier_positions {
                    nodes.insert(from.clone());
                    nodes.insert(to.clone());
                    adjacency
                        .entry(from.clone())
                        .or_default()
                        .insert(to.clone());
                    special.insert((from.clone(), to.clone()));
                }
            }
        }
    }

    let ordered: Vec<Position> = nodes.iter().cloned().collect();
    add_subsumption(&mut adjacency, &ordered);

    // A special edge `(u, v)` violates weak acyclicity exactly when `v` reaches `u`: the
    // null created at `v` can flow back to a position that creates another null.
    let mut violations: Vec<String> = Vec::new();
    for (from, to) in &special {
        if reaches(&adjacency, to, from) {
            violations.push(format!(
                "existential edge {} -> {} lies in a cycle, so the restricted chase may not \
                 terminate",
                from.render(),
                to.render()
            ));
        }
    }
    violations.sort();
    violations.dedup();

    if violations.is_empty() {
        ChaseTermination::WeaklyAcyclic {
            positions: nodes.len(),
            existential_edges: special.len(),
        }
    } else {
        ChaseTermination::Unbounded { violations }
    }
}

// ── Witnesses ───────────────────────────────────────────────────────────────────

/// The blank-node scope a chase witness is minted into: the DEFAULT scope.
///
/// The repository renders a blank node as `_:<scope ordinal>.<label>`, and scope `0` is the
/// default scope. A chase witness belongs to the dataset being closed, so it is minted
/// there rather than into a scope of its own.
const WITNESS_SCOPE: u32 = 0;

/// The first character of every witness label.
///
/// A leading letter keeps the label a well-formed blank-node label whatever the digest
/// happens to start with, and it makes a witness visually identifiable in a fact dump.
const WITNESS_LABEL_PREFIX: char = 'w';

/// How many digest bytes a witness label carries (rendered as twice as many hex digits).
const WITNESS_LABEL_BYTES: usize = 16;

/// The domain-separation tag hashed into every witness address.
///
/// Framed like every other field, so a witness digest can never coincide with some other
/// BLAKE3 digest this crate computes over a different kind of value.
const WITNESS_DIGEST_TAG: &str = "purrdf-datalog restricted chase witness v1";

/// Lowercase hex digits, for rendering a witness label without a formatter.
const HEX_DIGITS: [u8; 16] = *b"0123456789abcdef";

/// The address a Skolem witness is minted against: the SKOLEM FUNCTION APPLICATION that
/// produced it.
///
/// A witness is a function of the clause, the existential's ordinal within that clause's
/// quantifier list, and the bound frontier VALUES — never of the lexical variable names
/// alone, and never of the order firings happened to arrive in. Two different frontier
/// bindings therefore get two different witnesses, and re-deriving the same obligation on
/// the same frontier recovers the SAME witness, which is what lets the fixpoint close.
///
/// The address is decomposable, which an opaque digest is not: [`SkolemRegistry::explain`]
/// hands it back so a caller can say what an invented null stands for. A frontier value may
/// itself be a previously invented witness, so an explanation is recursively decomposable
/// through the same registry.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct WitnessAddress {
    /// The producing clause's index in authored program order.
    clause: usize,
    /// The existential's position in the clause's authored quantifier list.
    ordinal: usize,
    /// The bound frontier values, as lexical surfaces, in the clause's fixed frontier
    /// order (the frontier variables in lexical name order).
    frontier: Vec<String>,
}

impl WitnessAddress {
    /// The producing clause's index in authored program order.
    pub fn clause(&self) -> usize {
        self.clause
    }

    /// The existential's position in the clause's authored quantifier list.
    pub fn ordinal(&self) -> usize {
        self.ordinal
    }

    /// The bound frontier values, as lexical surfaces, in the clause's fixed frontier order.
    pub fn frontier(&self) -> &[String] {
        &self.frontier
    }
}

/// The blank-node surface addressed by `address`.
///
/// The label is a BLAKE3 digest over a length-framed encoding of the address: every field is
/// preceded by its byte length, so no frontier value can forge a field boundary whatever
/// bytes it holds, and the encoding is injective. The surface is therefore a deterministic,
/// collision-resistant function of the address alone — identical on every target, and
/// independent of the order witnesses were minted in.
fn witness_surface(address: &WitnessAddress) -> String {
    /// Append `bytes` as its `u64` little-endian length followed by the bytes themselves.
    fn frame(hasher: &mut blake3::Hasher, bytes: &[u8]) {
        hasher.update(&(bytes.len() as u64).to_le_bytes());
        hasher.update(bytes);
    }

    let mut hasher = blake3::Hasher::new();
    frame(&mut hasher, WITNESS_DIGEST_TAG.as_bytes());
    hasher.update(&(address.clause as u64).to_le_bytes());
    hasher.update(&(address.ordinal as u64).to_le_bytes());
    hasher.update(&(address.frontier.len() as u64).to_le_bytes());
    for value in &address.frontier {
        frame(&mut hasher, value.as_bytes());
    }
    let digest = hasher.finalize();

    let mut surface = format!("_:{WITNESS_SCOPE}.");
    surface.push(WITNESS_LABEL_PREFIX);
    for &byte in &digest.as_bytes()[..WITNESS_LABEL_BYTES] {
        surface.push(char::from(HEX_DIGITS[usize::from(byte >> 4)]));
        surface.push(char::from(HEX_DIGITS[usize::from(byte & 0x0f)]));
    }
    surface
}

/// The witnesses one chase invented, keyed by blank-node surface.
///
/// **Idempotent**: minting the same address twice returns the same surface and retains the
/// same recipe. That is not a convenience — it is the second half of the termination
/// argument. The restricted-satisfaction check skips a firing whose obligation is already
/// witnessed; where a check cannot see a witness yet (two body solutions sharing one
/// frontier inside a single round, say) the registry collapses the repeat firings onto one
/// witness and the round's fact set deduplicates. Without idempotence each re-derivation
/// would invent a fresh null and the fixpoint would never close.
///
/// **Deterministic**: a `BTreeMap`, so every sweep is sorted, and the key is a content
/// digest rather than a counter, so it does not depend on mint order either.
#[derive(Debug, Clone, Default)]
pub struct SkolemRegistry {
    /// Witness surface → the address it was minted against.
    witnesses: BTreeMap<String, WitnessAddress>,
}

impl SkolemRegistry {
    /// A fresh, empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Mint — or recover — the witness for `address`, returning its blank-node surface.
    fn mint(&mut self, address: WitnessAddress) -> String {
        let surface = witness_surface(&address);
        self.witnesses.entry(surface.clone()).or_insert(address);
        surface
    }

    /// The address behind an invented witness, or `None` if this registry never minted it.
    ///
    /// This is the "explain an invented individual" surface: a witness is a blank node whose
    /// label is a digest and therefore says nothing on its own, but the registry retains the
    /// Skolem-function application — clause, ordinal, frontier binding — that produced it.
    pub fn explain(&self, witness: &str) -> Option<&WitnessAddress> {
        self.witnesses.get(witness)
    }

    /// Every invented witness surface, in sorted order.
    pub fn witnesses(&self) -> impl Iterator<Item = &str> {
        self.witnesses.keys().map(String::as_str)
    }

    /// How many distinct witnesses were invented.
    pub fn len(&self) -> usize {
        self.witnesses.len()
    }

    /// Whether no witness was invented.
    pub fn is_empty(&self) -> bool {
        self.witnesses.is_empty()
    }
}

// ── Errors ──────────────────────────────────────────────────────────────────────

/// Why a chase could not run, or could not run to completion.
///
/// Every variant is a hard refusal with a named reason. There is no partial answer and no
/// best-effort mode: a [`ChaseOutcome`] this module returns is a complete universal model of
/// the program it was given.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChaseError {
    /// [`certify`] could not prove the program terminating.
    ///
    /// Running it anyway could invent nulls forever, and truncating the run would return a
    /// structure that is not a model of the program. The payload is the sorted violation
    /// list, so the diagnostic points at the edges to fix.
    NonTerminating {
        /// The offending existential edges, sorted and deduplicated.
        violations: Vec<String>,
    },
    /// An EXISTENTIAL head has more than one disjunct.
    ///
    /// `∃ȳ. (C₁ ∨ … ∨ Cₘ)` with `m > 1` has no single universal model: satisfying it means
    /// branching on which disjunct holds, which is a hypertableau case split. This module
    /// has none. Picking a disjunct — even the first — would assert something the program
    /// does not entail, so the clause is refused instead.
    DisjunctiveExistentialHead {
        /// The clause's index in authored program order.
        clause: usize,
        /// How many disjuncts the head has.
        disjuncts: usize,
    },
    /// A head is disjunctive with no existential, `m > 1` and `ȳ = ∅`.
    ///
    /// Refused for the same reason as [`Self::DisjunctiveExistentialHead`]: a disjunction
    /// has no least model, and the chase computes one model, not a set of them.
    DisjunctiveHead {
        /// The clause's index in authored program order.
        clause: usize,
        /// How many disjuncts the head has.
        disjuncts: usize,
    },
    /// A clause's head is `false` — the inconsistency clause `body → false`.
    ///
    /// It derives nothing and instead asserts that its body is unsatisfiable. A chase
    /// returns a model, not a consistency verdict, so there is no answer to give: firing it
    /// is meaningless and dropping it would silently answer a different question.
    InconsistencyClause {
        /// The clause's index in authored program order.
        clause: usize,
    },
    /// A body atom is negated.
    ///
    /// Negation as failure and value invention do not compose without a stratification
    /// argument: a witness invented in a later round can falsify a NAF test an earlier
    /// round already relied on, so the result would not be a model of the program. There is
    /// no stratifier here, so the clause is refused rather than evaluated under a semantics
    /// this module cannot justify.
    NegatedBodyAtom {
        /// The clause's index in authored program order.
        clause: usize,
        /// The negated atom's index in the clause's authored body.
        body_index: usize,
    },
    /// A head variable is neither existentially quantified nor bound by the body.
    ///
    /// Grounding the head would mean fabricating a term for it. An existential head variable
    /// is the SUPPORTED way to say "some value exists here"; an unquantified one is a typo.
    UnboundHeadVariable {
        /// The clause's index in authored program order.
        clause: usize,
        /// The unbindable variable, as authored.
        variable: String,
    },
    /// A fixed ceiling was passed. The report is accurate at the point the chase stopped.
    ///
    /// The ceilings are [`MAX_JOIN_STEPS`], [`MAX_STORED_FACTS`] and
    /// [`MAX_TERM_ARENA_BYTES`] — this crate's own constants, charged exactly as
    /// [`crate::seminaive::evaluate`] charges them. There is no caller-facing budget
    /// parameter, and there is never a truncated answer presented as complete.
    BudgetExhausted {
        /// Which ceiling.
        resource: BudgetResource,
        /// Consumption of all three ceilings when the chase stopped.
        report: BudgetReport,
    },
}

impl fmt::Display for ChaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonTerminating { violations } => {
                write!(
                    f,
                    "the restricted chase is not certified to terminate on this program: {}",
                    violations.join("; ")
                )
            }
            Self::DisjunctiveExistentialHead { clause, disjuncts } => write!(
                f,
                "clause {clause} has an existential head with {disjuncts} disjuncts: the chase \
                 has no case split, and choosing a disjunct would assert what the program does \
                 not entail"
            ),
            Self::DisjunctiveHead { clause, disjuncts } => write!(
                f,
                "clause {clause} has a disjunctive head with {disjuncts} disjuncts: the chase \
                 computes one model, and a disjunction has no least one"
            ),
            Self::InconsistencyClause { clause } => write!(
                f,
                "clause {clause} has an empty (false) head: the chase returns a model, not a \
                 consistency verdict"
            ),
            Self::NegatedBodyAtom { clause, body_index } => write!(
                f,
                "clause {clause} body atom {body_index} is negated: negation as failure has no \
                 semantics under a chase that invents values, and there is no stratifier here"
            ),
            Self::UnboundHeadVariable { clause, variable } => write!(
                f,
                "clause {clause} head variable {variable} is neither existentially quantified \
                 nor bound by the body, so it cannot be grounded"
            ),
            Self::BudgetExhausted { resource, report } => write!(
                f,
                "the chase exceeded a fixed ceiling ({resource:?}): {} join step(s), {} stored \
                 fact(s), {} term arena byte(s)",
                report.join_steps(),
                report.stored_facts(),
                report.term_arena_bytes()
            ),
        }
    }
}

impl std::error::Error for ChaseError {}

// ── The chase ───────────────────────────────────────────────────────────────────

/// A variable binding: variable name → the bound term's lexical surface.
///
/// A `BTreeMap` because a binding is walked when a witness address is assembled, and that
/// walk reaches an output.
type Binding = BTreeMap<String, String>;

/// The candidate-solution meter, charged exactly as [`crate::seminaive`] charges
/// [`MAX_JOIN_STEPS`]: one step per partial or complete solution appended by an atom
/// extension.
///
/// Charging inside the join — rather than counting committed facts afterwards — is what
/// bounds a body that is accidentally Cartesian: such a body enumerates millions of
/// candidates while committing three facts, and a ceiling on commits alone would never see
/// it. The meter permits ONE step past the ceiling so that reaching it is proof the ceiling
/// was passed; the refusal is then raised by [`ChaseState::check_budget`].
#[derive(Debug, Clone, Copy, Default)]
struct StepMeter {
    /// Candidate solutions enumerated so far.
    consumed: u64,
}

impl StepMeter {
    /// Record one enumerated candidate.
    fn charge(&mut self) {
        self.consumed = self.consumed.saturating_add(1);
    }

    /// Whether the ceiling has been passed, so no further candidate may be enumerated.
    fn spent(self) -> bool {
        self.consumed > MAX_JOIN_STEPS
    }
}

/// Extend `solution` by every row of the store that `atom` matches, appending each
/// extension to `out`.
///
/// The four positions are resolved once: a constant renders to its lexical surface, a bound
/// variable reads its binding, an unbound variable is free. The predicate and graph
/// positions select the partitions, the subject and object positions drive the partition's
/// `(subject, object)` index, and the two are independent — exactly the access path
/// [`crate::seminaive`] uses, so an atom that names its predicate costs one ordered-map
/// probe and one that quantifies over it costs a lexical partition sweep.
///
/// A repeated variable — within the atom or shared with `solution` — is enforced per row
/// rather than through the index, so `p(?x, ?p, ?x, ?g)` behaves as the diagonal it is.
fn extend_solution(
    atom: &ClauseAtom,
    store: &RelationStore,
    solution: &Binding,
    out: &mut Vec<Binding>,
    meter: &mut StepMeter,
) {
    let terms = atom.terms();
    // The already-known surface of each position, or `None` when the scan binds it.
    let known: [Option<String>; ATOM_ARITY] = std::array::from_fn(|index| match terms[index] {
        ClauseTerm::Var(name) => solution.get(name).cloned(),
        constant => constant.surface(),
    });

    let mut ids: [Option<TermId>; ATOM_ARITY] = [None; ATOM_ARITY];
    for (index, surface) in known.iter().enumerate() {
        if let Some(surface) = surface {
            // A pinned term the store has never interned matches nothing at all.
            let Some(id) = store.term_id(surface) else {
                return;
            };
            ids[index] = Some(id);
        }
    }

    let bound = match (ids[POSITION_SUBJECT], ids[POSITION_OBJECT]) {
        (Some(subject), Some(object)) => Bound::Both(subject, object),
        (Some(subject), None) => Bound::Subject(subject),
        (None, Some(object)) => Bound::Object(object),
        (None, None) => Bound::Any,
    };

    let interner = store.interner();
    for partition in store.partitions(ids[POSITION_PREDICATE], ids[POSITION_GRAPH]) {
        let mut cursor = partition.select(bound);
        while let Some((subject, object, _row)) = LendingIterator::next(&mut cursor) {
            if meter.spent() {
                return;
            }
            let matched = [
                interner.resolve(subject),
                interner.resolve(partition.predicate()),
                interner.resolve(object),
                interner.resolve(partition.graph()),
            ];
            // Agreement first, allocation second: a row that fails the diagonal never
            // clones the binding.
            let mut fresh: [Option<(&str, &str)>; ATOM_ARITY] = [None; ATOM_ARITY];
            let mut agrees = true;
            for (index, term) in terms.iter().enumerate() {
                let Some(name) = term.variable() else {
                    // A constant position was pinned through the index above, so it agrees.
                    continue;
                };
                let already = solution.get(name).map(String::as_str).or_else(|| {
                    fresh
                        .iter()
                        .flatten()
                        .find(|(bound_name, _)| *bound_name == name)
                        .map(|&(_, value)| value)
                });
                match already {
                    Some(value) if value != matched[index] => {
                        agrees = false;
                        break;
                    }
                    Some(_) => {}
                    None => fresh[index] = Some((name, matched[index])),
                }
            }
            if !agrees {
                continue;
            }
            let mut merged = solution.clone();
            for &(name, value) in fresh.iter().flatten() {
                merged.insert(name.to_owned(), value.to_owned());
            }
            meter.charge();
            out.push(merged);
        }
    }
}

/// Join `atoms` as a conjunctive query over `store`, seeded with `seed`.
///
/// Used twice, and that reuse is the point: once for a clause BODY seeded with the empty
/// binding, and once for the restricted-satisfaction probe over a head disjunct seeded with
/// the frontier binding and the existentials left FREE. The blocking condition is literally
/// "does this conjunctive query have a solution", so writing it as a second, subtly
/// different traversal would be an opportunity for the two to disagree.
fn join_atoms<'a>(
    atoms: impl IntoIterator<Item = &'a ClauseAtom>,
    store: &RelationStore,
    seed: &Binding,
    meter: &mut StepMeter,
) -> Vec<Binding> {
    let mut solutions = vec![seed.clone()];
    for atom in atoms {
        let mut next = Vec::new();
        for solution in &solutions {
            if meter.spent() {
                break;
            }
            extend_solution(atom, store, solution, &mut next, meter);
        }
        solutions = next;
        if solutions.is_empty() {
            break;
        }
    }
    solutions
}

/// The restriction of `solution` to `variables`.
fn restrict(solution: &Binding, variables: &[String]) -> Binding {
    variables
        .iter()
        .filter_map(|name| {
            solution
                .get(name)
                .map(|value| (name.clone(), value.clone()))
        })
        .collect()
}

/// Ground one head atom under a binding that covers every variable it mentions.
///
/// # Panics
///
/// Panics on a variable the binding does not cover. [`plan_firings`] refuses a clause whose
/// head carries a variable that is neither a frontier variable nor an existential, the
/// frontier is bound by the body join, and every existential is bound to a freshly minted
/// witness before this is reached, so an uncovered variable here is a contradiction in that
/// check rather than a data state.
fn ground(atom: &ClauseAtom, binding: &Binding) -> Fact {
    let terms = atom.terms();
    let surfaces: [String; ATOM_ARITY] = std::array::from_fn(|index| match terms[index] {
        ClauseTerm::Var(name) => binding
            .get(name)
            .cloned()
            .expect("every head variable is a bound frontier variable or a minted witness"),
        constant => constant
            .surface()
            .expect("a non-variable term always has a lexical surface"),
    });
    let [subject, predicate, object, graph] = surfaces;
    Fact {
        subject,
        predicate,
        object,
        graph,
    }
}

/// One clause the chase will actually fire, with its loop-invariant shapes hoisted out of
/// every round.
#[derive(Debug, Clone)]
struct Firing {
    /// The clause's index in authored program order.
    clause: usize,
    /// The frontier variables, in lexical name order — the FIXED order a witness address's
    /// frontier values are listed in.
    frontier: Vec<String>,
}

impl Firing {
    /// The one head disjunct this clause fires.
    ///
    /// Every fireable head form has exactly one disjunct: the atomic and conjunctive forms
    /// by definition, and the existential form because [`plan_firings`] refuses a
    /// multi-disjunct existential head.
    fn disjunct<'a>(&self, clause: &'a DlClause) -> &'a HeadDisjunct {
        clause
            .head_disjuncts()
            .first()
            .expect("a fireable clause has exactly one head disjunct")
    }
}

/// Decide which clauses the chase fires, refusing every head form it has no semantics for.
///
/// The head form is decided FIRST, because the later checks are defined in terms of a head
/// the clause may not have: naming an unbound head variable in a clause whose real defect is
/// that its head is `false` would report a consequence rather than the cause.
fn plan_firings(program: &[DlClause]) -> Result<Vec<Firing>, ChaseError> {
    let mut firings = Vec::with_capacity(program.len());
    for (clause_index, clause) in program.iter().enumerate() {
        let disjuncts = clause.head_disjuncts().len();
        match clause.head_form() {
            HeadForm::Inconsistency => {
                return Err(ChaseError::InconsistencyClause {
                    clause: clause_index,
                });
            }
            HeadForm::Disjunctive => {
                return Err(ChaseError::DisjunctiveHead {
                    clause: clause_index,
                    disjuncts,
                });
            }
            HeadForm::Existential if disjuncts > 1 => {
                return Err(ChaseError::DisjunctiveExistentialHead {
                    clause: clause_index,
                    disjuncts,
                });
            }
            // An atomic head is a one-atom, zero-existential conjunctive head and a
            // conjunctive head is a zero-existential one, so all three fire through the
            // same loop.
            HeadForm::Atomic | HeadForm::Conjunctive | HeadForm::Existential => {}
        }

        for (body_index, atom) in clause.body().iter().enumerate() {
            if atom.is_negated() {
                return Err(ChaseError::NegatedBodyAtom {
                    clause: clause_index,
                    body_index,
                });
            }
        }

        let frontier = clause.frontier_variables();
        let existentials: BTreeSet<&str> =
            clause.existentials().iter().map(String::as_str).collect();
        for atom in clause.head_atoms() {
            for term in atom.terms() {
                if let Some(name) = term.variable()
                    && !frontier.contains(name)
                    && !existentials.contains(name)
                {
                    return Err(ChaseError::UnboundHeadVariable {
                        clause: clause_index,
                        variable: name.to_owned(),
                    });
                }
            }
        }

        firings.push(Firing {
            clause: clause_index,
            frontier: frontier.into_iter().collect(),
        });
    }
    Ok(firings)
}

/// The mutable working set carried across the chase's rounds.
#[derive(Debug)]
struct ChaseState {
    /// The accumulated store: the seeded EDB plus everything derived so far.
    store: RelationStore,
    /// Every derived fact with its producing clause, in commit order.
    derivations: Vec<ChaseDerivation>,
    /// The witnesses invented so far.
    witnesses: SkolemRegistry,
    /// The candidate-solution meter.
    meter: StepMeter,
}

impl ChaseState {
    /// The budget consumption observed so far.
    fn report(&self) -> BudgetReport {
        BudgetReport::new(
            self.meter.consumed,
            self.store.row_count(),
            self.store.term_bytes(),
        )
    }

    /// Refuse if any of the three fixed ceilings is already passed.
    ///
    /// Mirrors [`crate::seminaive`]'s own check exactly, including the order the ceilings
    /// are tested in, so the same overrun is reported under the same resource name whichever
    /// engine observed it.
    fn check_budget(&self) -> Result<(), ChaseError> {
        let report = self.report();
        if report.join_steps() > MAX_JOIN_STEPS {
            return Err(ChaseError::BudgetExhausted {
                resource: BudgetResource::JoinSteps,
                report,
            });
        }
        if report.stored_facts() > MAX_STORED_FACTS {
            return Err(ChaseError::BudgetExhausted {
                resource: BudgetResource::StoredFacts,
                report,
            });
        }
        if report.term_arena_bytes() > MAX_TERM_ARENA_BYTES {
            return Err(ChaseError::BudgetExhausted {
                resource: BudgetResource::TermArenaBytes,
                report,
            });
        }
        Ok(())
    }
}

/// One fact the chase derived, and the clause that concluded it.
///
/// The chase's counterpart to [`crate::seminaive::Derivation`], and deliberately smaller
/// than it: a chase firing's antecedents are not a fixed-arity body match (a conjunctive
/// head asserts several facts from one firing, and a witness has no source row at all), so
/// this records the ATTRIBUTION rather than a proof. A consumer that credits a derived
/// triple to the specification rule that concluded it — which is what a reasoning report
/// does — needs exactly the authored clause index, and mapping that index back to a rule is
/// the consumer's own table, because one rule may author several clauses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChaseDerivation {
    /// The derived fact.
    fact: Fact,
    /// The producing clause's index in authored program order.
    clause: usize,
}

impl ChaseDerivation {
    /// The derived fact.
    pub fn fact(&self) -> &Fact {
        &self.fact
    }

    /// The producing clause's index in AUTHORED program order.
    ///
    /// Indexes the `program` slice [`chase`] was called with, so a caller can map it back to
    /// whatever authored the clause. A conjunctive or existential head asserts several facts
    /// from ONE firing, and every one of them carries that same clause index — which is
    /// correct for a consumer counting one credit per triple that entered the closure.
    pub fn clause(&self) -> usize {
        self.clause
    }
}

/// A completed chase: the closed store, the facts it derived, the witnesses it invented,
/// what it consumed, and the certificate that justified running it at all.
#[derive(Debug, Clone)]
pub struct ChaseOutcome {
    /// The universal model: the seeded EDB plus every derived fact.
    facts: RelationStore,
    /// Every derived fact, in commit order — round by round, and lexically within a round.
    derived: Vec<Fact>,
    /// The same facts with their producing clause, in the same order as `derived`.
    derivations: Vec<ChaseDerivation>,
    /// The witnesses the chase invented.
    witnesses: SkolemRegistry,
    /// What the run consumed of the three fixed ceilings.
    budget: BudgetReport,
    /// The termination certificate that admitted the program.
    termination: ChaseTermination,
}

impl ChaseOutcome {
    /// The universal model: the seeded EDB plus every derived fact.
    pub fn facts(&self) -> &RelationStore {
        &self.facts
    }

    /// Take ownership of the universal model.
    pub fn into_facts(self) -> RelationStore {
        self.facts
    }

    /// Every derived fact, in commit order.
    ///
    /// The order is round by round and, within a round, total lexical
    /// `(subject, predicate, object, graph)` order — so it is byte-deterministic while still
    /// showing which layer of the fixpoint produced each fact. It is NOT globally sorted,
    /// because flattening the layers would discard that.
    pub fn derived(&self) -> &[Fact] {
        &self.derived
    }

    /// Every derived fact with the clause that concluded it, in commit order.
    ///
    /// Position `i` here is position `i` of [`Self::derived`] — the same facts, in the same
    /// order, with the attribution attached.
    ///
    /// **Which clause gets the credit.** Exactly one derivation per COMMITTED fact, and the
    /// credit goes to the FIRST commit: a fact a later round re-derives is already in the
    /// store and never enters the vector again, so it keeps the attribution it entered the
    /// closure with. Within a single round the round's candidates are accumulated in a
    /// `BTreeMap` keyed by the fact's four lexical surfaces and attributed by the LOWEST
    /// authored clause index that produced them, so "first" is decided by the program and
    /// the data — never by which clause the loop happened to reach first, and never by the
    /// order the join enumerated rows in.
    pub fn derivations(&self) -> &[ChaseDerivation] {
        &self.derivations
    }

    /// The witnesses the chase invented, so an invented null can be explained.
    pub fn witnesses(&self) -> &SkolemRegistry {
        &self.witnesses
    }

    /// What the run consumed of the three fixed ceilings.
    pub fn budget(&self) -> BudgetReport {
        self.budget
    }

    /// The termination certificate that admitted the program.
    ///
    /// Always [`ChaseTermination::WeaklyAcyclic`] on a successful run — the
    /// [`Unbounded`](ChaseTermination::Unbounded) case is returned as
    /// [`ChaseError::NonTerminating`] instead — and carried here so a caller can report the
    /// proof alongside the facts it justifies.
    pub fn termination(&self) -> &ChaseTermination {
        &self.termination
    }
}

/// Run the restricted existential chase of `program` over the seeded store `edb`.
///
/// The store is consumed and returned closed inside [`ChaseOutcome`], so the universal model
/// and its witnesses stay one value.
///
/// # What it does, in order
///
/// 1. [`certify`] the clause set. An [`Unbounded`](ChaseTermination::Unbounded) verdict is
///    refused as [`ChaseError::NonTerminating`] — the ONLY programs this function rejects
///    for termination are the ones the analysis actually failed to certify.
/// 2. Refuse every head form and body shape the chase has no semantics for; see
///    [`ChaseError`] for the complete list and the reason attached to each.
/// 3. Run the fixpoint. Each round joins every fireable clause's body against the whole
///    accumulated store; for each body solution the head disjunct is run as a conjunctive
///    query with the existentials FREE and the firing is SKIPPED if it already has a
///    solution (the restricted-chase blocking condition — without it the fixpoint does not
///    converge); otherwise one witness is minted per existential, addressed on the frontier
///    binding, and EVERY atom of the disjunct is grounded under the same extended binding,
///    so `∃y. (r(x, y) ∧ C(y))` shares one witness across both atoms. The round's facts are
///    committed in total lexical order and the chase stops when a round commits nothing.
///
/// # Budgets
///
/// There is no budget parameter. Consumption is charged against this crate's fixed
/// [`MAX_JOIN_STEPS`], [`MAX_STORED_FACTS`] and [`MAX_TERM_ARENA_BYTES`] exactly as
/// [`crate::seminaive::evaluate`] charges them, and passing one is a refusal
/// ([`ChaseError::BudgetExhausted`]) carrying an accurate report — never a truncated answer.
///
/// # Errors
///
/// Any [`ChaseError`].
pub fn chase(program: &[DlClause], edb: RelationStore) -> Result<ChaseOutcome, ChaseError> {
    let termination = certify(program);
    if let ChaseTermination::Unbounded { violations } = termination {
        return Err(ChaseError::NonTerminating { violations });
    }
    let firings = plan_firings(program)?;

    let mut state = ChaseState {
        store: edb,
        derivations: Vec::new(),
        witnesses: SkolemRegistry::new(),
        meter: StepMeter::default(),
    };
    state.check_budget()?;

    loop {
        // One round's candidate facts → the clause credited with them, in total lexical
        // order and deduplicated: two clauses deriving the same fact, or one clause firing
        // twice on one frontier, collapse here rather than racing to be committed first.
        // The credit goes to the LOWEST authored clause index that produced the fact
        // (`or_insert` over clauses visited in program order), so attribution is a function
        // of the program and the data rather than of the traversal.
        let mut pending: BTreeMap<Fact, usize> = BTreeMap::new();
        let seed = Binding::new();

        for firing in &firings {
            let clause = &program[firing.clause];
            let disjunct = firing.disjunct(clause);
            let solutions = join_atoms(clause.body(), &state.store, &seed, &mut state.meter);
            for solution in solutions {
                if state.meter.spent() {
                    break;
                }
                // The restricted-chase blocking condition: seed the head with the frontier
                // binding, leave the existentials free, and skip the firing if the store
                // already satisfies it. For a zero-existential head this reduces to "is the
                // ground head already present", which is the ordinary Datalog dedup.
                let head_binding = restrict(&solution, &firing.frontier);
                if !join_atoms(
                    disjunct.atoms(),
                    &state.store,
                    &head_binding,
                    &mut state.meter,
                )
                .is_empty()
                {
                    continue;
                }

                let frontier: Vec<String> = firing
                    .frontier
                    .iter()
                    .map(|name| {
                        head_binding
                            .get(name)
                            .cloned()
                            .expect("a frontier variable is bound by the body join")
                    })
                    .collect();
                let mut extended = head_binding;
                for (ordinal, existential) in clause.existentials().iter().enumerate() {
                    let witness = state.witnesses.mint(WitnessAddress {
                        clause: firing.clause,
                        ordinal,
                        frontier: frontier.clone(),
                    });
                    extended.insert(existential.clone(), witness);
                }
                // Every atom of the disjunct is credited to this ONE clause: a conjunctive
                // or existential head asserts several facts from a single firing, and a
                // consumer counts one credit per triple that entered the closure.
                for atom in disjunct.atoms() {
                    pending
                        .entry(ground(atom, &extended))
                        .or_insert(firing.clause);
                }
            }
            if state.meter.spent() {
                break;
            }
        }
        state.check_budget()?;

        // A fact an earlier round already committed keeps the attribution it entered the
        // closure with: it is in the store, so it is filtered out here and never re-credited.
        let fresh: Vec<ChaseDerivation> = pending
            .into_iter()
            .filter(|(fact, _)| {
                !state
                    .store
                    .contains(&fact.subject, &fact.predicate, &fact.object, &fact.graph)
            })
            .map(|(fact, clause)| ChaseDerivation { fact, clause })
            .collect();
        if fresh.is_empty() {
            break; // the natural fixpoint: this round derived nothing new
        }
        // Every fact here is absent from the store and unique within the round, so the
        // projection is exact rather than an over-estimate: the ceiling is decided before a
        // single row is inserted.
        if state.store.row_count() + fresh.len() > MAX_STORED_FACTS {
            return Err(ChaseError::BudgetExhausted {
                resource: BudgetResource::StoredFacts,
                report: BudgetReport::new(
                    state.meter.consumed,
                    state.store.row_count() + fresh.len(),
                    state.store.term_bytes(),
                ),
            });
        }
        for derivation in fresh {
            let fact = &derivation.fact;
            state
                .store
                .insert(&fact.subject, &fact.predicate, &fact.object, &fact.graph);
            state.derivations.push(derivation);
        }
        state.check_budget()?;
    }

    Ok(ChaseOutcome {
        budget: state.report(),
        facts: state.store,
        // `derived` is the same sequence with the attribution projected away, materialised
        // once here so both accessors are a plain slice read.
        derived: state
            .derivations
            .iter()
            .map(|derivation| derivation.fact.clone())
            .collect(),
        derivations: state.derivations,
        witnesses: state.witnesses,
        termination,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The `rdf:type`-shaped predicate a concept assertion lowers through. PurRDF mints no
    /// vocabulary, so the fixture names its own, under `example.org` like every other
    /// fixture IRI.
    const TYPE: &str = "https://example.org/type";
    const P: &str = "https://example.org/p";
    const R: &str = "https://example.org/r";
    const A: &str = "https://example.org/A";
    const CONCEPT_C: &str = "https://example.org/C";
    const CONCEPT_D: &str = "https://example.org/D";

    fn v(name: &str) -> ClauseTerm {
        ClauseTerm::var(name)
    }

    /// The lexical surface an IRI is stored under.
    fn iri(name: &str) -> String {
        format!("<{name}>")
    }

    /// `C(?var)` as the quad `?var TYPE <C>` in the default graph.
    fn concept(var: &str, concept_iri: &str) -> ClauseAtom {
        ClauseAtom::positive(v(var), TYPE, ClauseTerm::iri(concept_iri))
    }

    /// `p(?subject, ?object)` in the default graph.
    fn role(subject: &str, predicate: &str, object: &str) -> ClauseAtom {
        ClauseAtom::positive(v(subject), predicate, v(object))
    }

    /// A store seeded with `(subject, predicate, object)` triples in the default graph.
    fn store_of(triples: &[(&str, &str, &str)]) -> RelationStore {
        let mut store = RelationStore::new();
        for &(subject, predicate, object) in triples {
            store.insert(
                &iri(subject),
                &iri(predicate),
                &iri(object),
                RelationStore::DEFAULT_GRAPH,
            );
        }
        store
    }

    /// `A(?x) → ∃?y. ( r(?x, ?y) ∧ C(?y) )` — the canonical `A ⊑ ∃r.C` restriction.
    fn restriction() -> DlClause {
        DlClause::new(
            vec![HeadDisjunct::new(vec![
                role("?x", R, "?y"),
                concept("?y", CONCEPT_C),
            ])],
            vec!["?y".to_owned()],
            vec![concept("?x", A)],
        )
    }

    /// The facts of `outcome`'s closed store, as sorted `(s, p, o, g)` surface tuples.
    fn closure(outcome: &ChaseOutcome) -> Vec<(String, String, String, String)> {
        outcome
            .facts()
            .facts_sorted()
            .into_iter()
            .map(|fact| (fact.subject, fact.predicate, fact.object, fact.graph))
            .collect()
    }

    /// The chase of `program` over `edb`, asserting it ran.
    fn run(program: &[DlClause], edb: RelationStore) -> ChaseOutcome {
        chase(program, edb).expect("the fixture program is certified and fireable")
    }

    // ── The restriction fires, terminates and mints blank nodes ─────────────────

    /// `A ⊑ ∃r.C` over two individuals terminates, mints exactly ONE witness per distinct
    /// `?x`, asserts BOTH head atoms over that one witness, and every witness surface is a
    /// blank node.
    #[test]
    fn a_restriction_mints_one_blank_node_witness_per_subject() {
        let program = [restriction()];
        assert!(certify(&program).is_certified());
        let outcome = run(&program, store_of(&[("a", TYPE, A), ("b", TYPE, A)]));

        assert_eq!(
            outcome.witnesses().len(),
            2,
            "one witness per distinct frontier binding, not per head atom: {:?}",
            outcome.witnesses().witnesses().collect::<Vec<_>>()
        );
        // Two head atoms per witness.
        assert_eq!(outcome.derived().len(), 4, "{:?}", outcome.derived());

        for witness in outcome.witnesses().witnesses() {
            assert!(
                witness.starts_with("_:"),
                "a witness is a blank node, never an IRI: {witness}"
            );
            assert!(
                !witness.starts_with('<'),
                "a witness is never an IRI: {witness}"
            );
            assert!(
                witness.starts_with("_:0."),
                "a witness lives in the default blank scope: {witness}"
            );
        }

        // Each witness carries BOTH head atoms — the shared-witness property that makes
        // `A ⊑ ∃r.C` one clause rather than two.
        for subject in ["a", "b"] {
            let witness = outcome
                .derived()
                .iter()
                .find(|fact| fact.subject == iri(subject) && fact.predicate == iri(R))
                .map(|fact| fact.object.clone())
                .expect("every A individual gets an r-successor");
            assert!(outcome.facts().contains(
                &witness,
                &iri(TYPE),
                &iri(CONCEPT_C),
                RelationStore::DEFAULT_GRAPH
            ));
        }
    }

    /// A witness explains itself: the registry hands back the clause, the ordinal and the
    /// frontier VALUES it was addressed on.
    #[test]
    fn a_witness_explains_its_skolem_address() {
        let program = [restriction()];
        let outcome = run(&program, store_of(&[("a", TYPE, A)]));
        let witness = outcome
            .witnesses()
            .witnesses()
            .next()
            .expect("one witness")
            .to_owned();
        let address = outcome
            .witnesses()
            .explain(&witness)
            .expect("a minted witness is explainable");
        assert_eq!(address.clause(), 0);
        assert_eq!(address.ordinal(), 0);
        assert_eq!(address.frontier(), [iri("a")]);
        assert_eq!(outcome.witnesses().explain("_:0.wdeadbeef"), None);
        assert!(!outcome.witnesses().is_empty());
    }

    // ── The restricted check actually blocks ────────────────────────────────────

    /// Seeding the store with a SATISFYING `r(a, b) ∧ C(b)` blocks the firing for `a`: the
    /// obligation is already witnessed, so the restricted chase invents nothing.
    ///
    /// This is the difference between the restricted and the oblivious chase, and it is
    /// what makes the fixpoint converge.
    #[test]
    fn the_restricted_check_blocks_an_already_witnessed_obligation() {
        let program = [restriction()];
        let seeded = store_of(&[
            ("a", TYPE, A),
            ("a", R, "b"),
            ("b", TYPE, CONCEPT_C),
            ("c", TYPE, A),
        ]);
        let before = seeded.facts_sorted();
        let outcome = run(&program, seeded);

        assert!(
            outcome.witnesses().len() == 1,
            "only `c` is unwitnessed: {:?}",
            outcome.witnesses().witnesses().collect::<Vec<_>>()
        );
        assert!(
            outcome
                .derived()
                .iter()
                .all(|fact| fact.subject == iri("c") || fact.subject.starts_with("_:")),
            "nothing was invented for `a`: {:?}",
            outcome.derived()
        );
        // The seeded facts survive untouched.
        for fact in before {
            assert!(outcome.facts().contains(
                &fact.subject,
                &fact.predicate,
                &fact.object,
                &fact.graph
            ));
        }
    }

    /// A PARTIALLY satisfied obligation is not satisfied: `r(a, b)` without `C(b)` leaves
    /// the conjunctive head unwitnessed, so the clause still fires.
    #[test]
    fn a_partially_satisfied_conjunctive_head_still_fires() {
        let program = [restriction()];
        let outcome = run(&program, store_of(&[("a", TYPE, A), ("a", R, "b")]));
        assert_eq!(outcome.witnesses().len(), 1, "{:?}", outcome.derived());
    }

    // ── Idempotence ─────────────────────────────────────────────────────────────

    /// Two runs of the same program over the same EDB produce byte-identical derived facts,
    /// byte-identical DERIVATIONS (fact and clause index alike), byte-identical witnesses
    /// and an identical closure.
    ///
    /// The two runs seed the store in OPPOSITE insertion orders, so anything that leaked
    /// mint order or arrival order into the attribution would show up here.
    #[test]
    fn two_runs_are_byte_identical() {
        let program = [restriction()];
        let first = run(&program, store_of(&[("a", TYPE, A), ("b", TYPE, A)]));
        let second = run(&program, store_of(&[("b", TYPE, A), ("a", TYPE, A)]));
        assert_eq!(first.derived(), second.derived());
        assert_eq!(first.derivations(), second.derivations());
        assert_eq!(closure(&first), closure(&second));
        assert_eq!(
            first.witnesses().witnesses().collect::<Vec<_>>(),
            second.witnesses().witnesses().collect::<Vec<_>>()
        );
        assert_eq!(
            first.budget().stored_facts(),
            second.budget().stored_facts()
        );
    }

    /// Re-deriving the same frontier INSIDE one run reuses ONE witness.
    ///
    /// The body `A(?x) ∧ p(?x, ?z)` has two solutions for `x = a` (one per `z`), and both
    /// share the frontier `{?x = a}`. The registry is a function of the frontier, so both
    /// firings recover the same witness and the round's fact set collapses them — which is
    /// exactly why the fixpoint closes.
    #[test]
    fn one_frontier_reuses_one_witness_within_a_round() {
        let program = [DlClause::new(
            vec![HeadDisjunct::new(vec![
                role("?x", R, "?y"),
                concept("?y", CONCEPT_C),
            ])],
            vec!["?y".to_owned()],
            vec![concept("?x", A), role("?x", P, "?z")],
        )];
        let outcome = run(
            &program,
            store_of(&[("a", TYPE, A), ("a", P, "m"), ("a", P, "n")]),
        );
        assert_eq!(
            outcome.witnesses().len(),
            1,
            "two body solutions, one frontier, one witness: {:?}",
            outcome.witnesses().witnesses().collect::<Vec<_>>()
        );
        assert_eq!(outcome.derived().len(), 2, "{:?}", outcome.derived());
    }

    /// Distinct frontier bindings mint DISTINCT witnesses — a witness is a function of the
    /// bound values, not of the clause alone.
    #[test]
    fn distinct_frontiers_mint_distinct_witnesses() {
        let program = [restriction()];
        let outcome = run(
            &program,
            store_of(&[("a", TYPE, A), ("b", TYPE, A), ("c", TYPE, A)]),
        );
        let witnesses: BTreeSet<&str> = outcome.witnesses().witnesses().collect();
        assert_eq!(witnesses.len(), 3);
    }

    // ── Termination analysis ────────────────────────────────────────────────────

    /// The classic non-terminating rule `r(?x, ?y) → ∃?z. r(?y, ?z)` is reported UNBOUNDED,
    /// and [`chase`] refuses it rather than looping.
    #[test]
    fn the_classic_non_terminating_rule_is_unbounded_and_refused() {
        let program = [DlClause::new(
            vec![HeadDisjunct::atom(role("?y", R, "?z"))],
            vec!["?z".to_owned()],
            vec![role("?x", R, "?y")],
        )];
        let termination = certify(&program);
        assert!(!termination.is_certified(), "{termination:?}");
        let ChaseTermination::Unbounded { violations } = &termination else {
            panic!("expected an unbounded verdict: {termination:?}");
        };
        assert!(!violations.is_empty());
        let mut sorted = violations.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(&sorted, violations, "violations are sorted and deduped");
        assert!(violations[0].contains("lies in a cycle"), "{violations:?}");
        assert!(termination.to_string().contains("not weakly acyclic"));

        let refusal = chase(&program, store_of(&[("a", R, "b")]))
            .expect_err("a non-certified program is refused");
        assert_eq!(
            refusal,
            ChaseError::NonTerminating {
                violations: violations.clone()
            }
        );
        assert!(refusal.to_string().contains("not certified to terminate"));
    }

    /// The constant-refined `type(?x, <C>) → ∃?y. ( p(?x, ?y) ∧ type(?y, <D>) )` IS weakly
    /// acyclic.
    ///
    /// This is the case UNREFINED weak acyclicity gets wrong: without the refinement the
    /// body's `type[subject]` and the head's `type[subject]` are the same node, the
    /// existential edge is a self-loop, and every `C ⊑ ∃p.D` axiom in every ontology would
    /// be refused. It is the whole reason the refinement exists, so it is asserted here.
    #[test]
    fn a_constant_refined_restriction_is_weakly_acyclic() {
        let program = [DlClause::new(
            vec![HeadDisjunct::new(vec![
                role("?x", P, "?y"),
                concept("?y", CONCEPT_D),
            ])],
            vec!["?y".to_owned()],
            vec![concept("?x", CONCEPT_C)],
        )];
        let termination = certify(&program);
        assert!(
            termination.is_certified(),
            "the constant refinement must separate the C-typed and D-typed positions: \
             {termination:?}"
        );
        let ChaseTermination::WeaklyAcyclic {
            positions,
            existential_edges,
        } = termination
        else {
            unreachable!("just asserted certified");
        };
        assert!(positions > 0);
        assert!(existential_edges > 0, "the analysis is not vacuous");

        // And it really runs.
        let outcome = run(&program, store_of(&[("a", TYPE, CONCEPT_C)]));
        assert_eq!(outcome.witnesses().len(), 1);
    }

    /// Swapping the head's class constant for the BODY's turns the same shape into the
    /// non-terminating `C ⊑ ∃p.C`, and the refinement no longer separates the positions —
    /// so the analysis reports it unbounded. The refinement is precise, not permissive.
    #[test]
    fn a_self_typed_restriction_is_unbounded() {
        let program = [DlClause::new(
            vec![HeadDisjunct::new(vec![
                role("?x", P, "?y"),
                concept("?y", CONCEPT_C),
            ])],
            vec!["?y".to_owned()],
            vec![concept("?x", CONCEPT_C)],
        )];
        assert!(!certify(&program).is_certified());
    }

    /// A program with NO existential head is trivially weakly acyclic however cyclic its
    /// Datalog recursion is: transitive closure invents nothing.
    #[test]
    fn a_recursive_datalog_program_is_trivially_weakly_acyclic() {
        let program = [DlClause::datalog(
            role("?x", R, "?z"),
            vec![role("?x", R, "?y"), role("?y", R, "?z")],
        )];
        let termination = certify(&program);
        assert_eq!(
            termination,
            ChaseTermination::WeaklyAcyclic {
                positions: termination_positions(&termination),
                existential_edges: 0,
            }
        );
        assert!(termination.to_string().contains("weakly acyclic"));
    }

    /// The position count of a certified verdict, for an assertion that does not restate
    /// the graph's size.
    fn termination_positions(termination: &ChaseTermination) -> usize {
        match termination {
            ChaseTermination::WeaklyAcyclic { positions, .. } => *positions,
            ChaseTermination::Unbounded { .. } => panic!("expected a certified verdict"),
        }
    }

    /// A WILDCARD predicate is conservatively connected to every constant-predicate position
    /// of the same slot, so a meta-rule that quantifies over the predicate cannot hide a
    /// cycle from the analysis.
    #[test]
    fn a_wildcard_predicate_is_conservatively_connected() {
        // `T(?x, ?p, ?y) → ∃?z. r(?y, ?z)`: the invented null sits at `r[object]`, and the
        // wildcard-predicate body position subsumes it, so the existential edge is in a
        // cycle even though no atom names `r` in the body.
        let program = [DlClause::new(
            vec![HeadDisjunct::atom(role("?y", R, "?z"))],
            vec!["?z".to_owned()],
            vec![ClauseAtom::quad(
                v("?x"),
                v("?p"),
                v("?y"),
                ClauseTerm::DefaultGraph,
            )],
        )];
        assert!(
            !certify(&program).is_certified(),
            "a variable predicate must not be treated as a fresh, unconnected relation"
        );
    }

    // ── Mixed programs: atomic and conjunctive heads fire ───────────────────────

    /// An ATOMIC head fires inside the same fixpoint as an existential one, and the
    /// existential clause consumes what the Datalog clause derived.
    #[test]
    fn an_atomic_head_fires_in_the_same_fixpoint() {
        // `C(?x) → A(?x)` and `A(?x) → ∃?y. ( r(?x, ?y) ∧ C(?y) )`.
        let program = [
            DlClause::datalog(concept("?x", A), vec![concept("?x", CONCEPT_D)]),
            restriction(),
        ];
        assert!(certify(&program).is_certified());
        let outcome = run(&program, store_of(&[("a", TYPE, CONCEPT_D)]));
        assert!(outcome.facts().contains(
            &iri("a"),
            &iri(TYPE),
            &iri(A),
            RelationStore::DEFAULT_GRAPH
        ));
        assert_eq!(
            outcome.witnesses().len(),
            1,
            "the existential clause consumed the Datalog conclusion: {:?}",
            outcome.derived()
        );
        // The Datalog conclusion is committed in an earlier round than the witness facts.
        assert_eq!(outcome.derived()[0].predicate, iri(TYPE));
        assert_eq!(outcome.derived()[0].object, iri(A));
    }

    // ── Attribution: which clause concluded a fact ──────────────────────────────

    /// Both atoms of `A(?x) → ∃?y. ( r(?x, ?y) ∧ C(?y) )` are credited to the SAME clause,
    /// and that clause is the one's AUTHORED position in the program that was passed in.
    ///
    /// The clause is authored at index 1 behind an unrelated clause that never fires, so a
    /// hard-coded `0` — or an index into the fireable subset rather than into the program —
    /// would fail here.
    #[test]
    fn a_conjunctive_existential_head_credits_one_clause() {
        let program = [
            // Authored FIRST and never fires: no individual is a `D`.
            DlClause::datalog(concept("?x", A), vec![concept("?x", CONCEPT_D)]),
            restriction(),
        ];
        let outcome = run(&program, store_of(&[("a", TYPE, A)]));

        assert_eq!(outcome.derivations().len(), 2, "{:?}", outcome.derived());
        for derivation in outcome.derivations() {
            assert_eq!(
                derivation.clause(),
                1,
                "both head atoms of one firing credit the authoring clause: {derivation:?}"
            );
        }
        // The two accessors describe the same sequence.
        assert_eq!(
            outcome
                .derivations()
                .iter()
                .map(ChaseDerivation::fact)
                .cloned()
                .collect::<Vec<_>>(),
            outcome.derived()
        );
    }

    /// A mixed program credits each derived fact to the RIGHT clause: the atomic clause at
    /// index 0 for its conclusion, the existential clause at index 1 for both of its head
    /// atoms.
    #[test]
    fn a_mixed_program_credits_each_fact_to_its_authoring_clause() {
        // 0: `D(?x) → A(?x)`.  1: `A(?x) → ∃?y. ( r(?x, ?y) ∧ C(?y) )`.
        let program = [
            DlClause::datalog(concept("?x", A), vec![concept("?x", CONCEPT_D)]),
            restriction(),
        ];
        let outcome = run(&program, store_of(&[("a", TYPE, CONCEPT_D)]));

        let credited: Vec<(String, String, usize)> = outcome
            .derivations()
            .iter()
            .map(|derivation| {
                (
                    derivation.fact().predicate.clone(),
                    derivation.fact().object.clone(),
                    derivation.clause(),
                )
            })
            .collect();
        assert_eq!(credited.len(), 3, "{credited:?}");

        // Round one: the Datalog conclusion `A(a)`, credited to clause 0.
        assert_eq!(credited[0], (iri(TYPE), iri(A), 0));
        // Round two: `r(a, w)` and `C(w)`, both credited to clause 1.
        let witness = outcome
            .witnesses()
            .witnesses()
            .next()
            .expect("one witness")
            .to_owned();
        let later: BTreeSet<(String, String, usize)> = credited[1..].iter().cloned().collect();
        assert_eq!(
            later,
            BTreeSet::from([(iri(R), witness, 1), (iri(TYPE), iri(CONCEPT_C), 1),]),
            "{credited:?}"
        );
    }

    /// A fact TWO clauses conclude is credited once, to the lower authored index — the
    /// attribution is a function of the program, not of which clause the loop reached first.
    #[test]
    fn a_shared_conclusion_is_credited_to_the_lower_authored_clause() {
        // Both clauses conclude `C(?x)` from `A(?x)`; only one derivation may result.
        let program = [
            DlClause::datalog(concept("?x", CONCEPT_C), vec![concept("?x", A)]),
            DlClause::datalog(concept("?x", CONCEPT_C), vec![concept("?x", A)]),
        ];
        let outcome = run(&program, store_of(&[("a", TYPE, A)]));
        assert_eq!(outcome.derivations().len(), 1, "{:?}", outcome.derived());
        assert_eq!(outcome.derivations()[0].clause(), 0);
        assert_eq!(outcome.derivations()[0].fact().object, iri(CONCEPT_C));
    }

    /// A CONJUNCTIVE head (`m = 1`, several conjuncts, no existential) fires: it is a
    /// zero-existential conjunctive head, and `→ p ∧ q` entails both conjuncts.
    #[test]
    fn a_conjunctive_head_fires() {
        let program = [DlClause::new(
            vec![HeadDisjunct::new(vec![
                concept("?x", CONCEPT_C),
                concept("?x", CONCEPT_D),
            ])],
            Vec::new(),
            vec![concept("?x", A)],
        )];
        assert_eq!(program[0].head_form(), HeadForm::Conjunctive);
        let outcome = run(&program, store_of(&[("a", TYPE, A)]));
        assert!(outcome.witnesses().is_empty(), "nothing was invented");
        assert_eq!(
            outcome
                .derived()
                .iter()
                .map(|fact| fact.object.clone())
                .collect::<Vec<_>>(),
            [iri(CONCEPT_C), iri(CONCEPT_D)]
        );
    }

    // ── Refusals ────────────────────────────────────────────────────────────────

    /// A MULTI-DISJUNCT existential head is refused by name: there is no case split here,
    /// and choosing a disjunct would assert what the program does not entail.
    #[test]
    fn a_multi_disjunct_existential_head_is_refused() {
        let program = [DlClause::new(
            vec![
                HeadDisjunct::atom(role("?x", R, "?y")),
                HeadDisjunct::atom(role("?x", P, "?y")),
            ],
            vec!["?y".to_owned()],
            vec![concept("?x", A)],
        )];
        assert_eq!(program[0].head_form(), HeadForm::Existential);
        let refusal =
            chase(&program, store_of(&[("a", TYPE, A)])).expect_err("a case split is refused");
        assert_eq!(
            refusal,
            ChaseError::DisjunctiveExistentialHead {
                clause: 0,
                disjuncts: 2
            }
        );
        assert!(refusal.to_string().contains("existential head with 2"));
    }

    /// A non-existential DISJUNCTIVE head is refused under its own name.
    #[test]
    fn a_disjunctive_head_is_refused() {
        let program = [DlClause::new(
            vec![
                HeadDisjunct::atom(concept("?x", CONCEPT_C)),
                HeadDisjunct::atom(concept("?x", CONCEPT_D)),
            ],
            Vec::new(),
            vec![concept("?x", A)],
        )];
        assert_eq!(
            chase(&program, RelationStore::new()).expect_err("no least model"),
            ChaseError::DisjunctiveHead {
                clause: 0,
                disjuncts: 2
            }
        );
    }

    /// An INCONSISTENCY clause is refused: the chase returns a model, not a verdict.
    #[test]
    fn an_inconsistency_clause_is_refused() {
        let program = [DlClause::inconsistency(vec![concept("?x", A)])];
        let refusal = chase(&program, RelationStore::new()).expect_err("no model to return");
        assert_eq!(refusal, ChaseError::InconsistencyClause { clause: 0 });
        assert!(refusal.to_string().contains("empty (false) head"));
    }

    /// A NEGATED body atom is refused: NAF and value invention do not compose without a
    /// stratification argument, and there is no stratifier here.
    #[test]
    fn a_negated_body_atom_is_refused() {
        let program = [DlClause::new(
            vec![HeadDisjunct::atom(role("?x", R, "?y"))],
            vec!["?y".to_owned()],
            vec![
                concept("?x", A),
                ClauseAtom::negated(v("?x"), TYPE, ClauseTerm::iri(CONCEPT_C)),
            ],
        )];
        assert_eq!(
            chase(&program, RelationStore::new()).expect_err("NAF is refused"),
            ChaseError::NegatedBodyAtom {
                clause: 0,
                body_index: 1
            }
        );
    }

    /// An UNBOUND head variable — neither existential nor body-bound — is refused rather
    /// than grounded with a fabricated term.
    #[test]
    fn an_unbound_head_variable_is_refused() {
        let program = [DlClause::new(
            vec![HeadDisjunct::new(vec![
                role("?x", R, "?y"),
                role("?w", P, "?y"),
            ])],
            vec!["?y".to_owned()],
            vec![concept("?x", A)],
        )];
        assert_eq!(
            chase(&program, RelationStore::new()).expect_err("`?w` cannot be grounded"),
            ChaseError::UnboundHeadVariable {
                clause: 0,
                variable: "?w".to_owned()
            }
        );
    }

    /// The head form is decided BEFORE the body and head-variable checks, so a clause with
    /// several defects reports the most fundamental one.
    #[test]
    fn the_head_form_is_decided_first() {
        let program = [DlClause::inconsistency(vec![
            concept("?x", A),
            ClauseAtom::negated(v("?x"), TYPE, ClauseTerm::iri(CONCEPT_C)),
        ])];
        assert_eq!(
            chase(&program, RelationStore::new()).expect_err("refused"),
            ChaseError::InconsistencyClause { clause: 0 }
        );
    }

    // ── Reporting surfaces ──────────────────────────────────────────────────────

    /// A successful run carries the certificate and an accurate budget report.
    #[test]
    fn an_outcome_carries_its_certificate_and_budget() {
        let program = [restriction()];
        let outcome = run(&program, store_of(&[("a", TYPE, A)]));
        assert!(outcome.termination().is_certified());
        assert!(outcome.budget().join_steps() > 0);
        assert_eq!(
            outcome.budget().stored_facts(),
            outcome.facts().row_count(),
            "the report describes the store it was taken from"
        );
        assert_eq!(
            outcome.budget().term_arena_bytes(),
            outcome.facts().term_bytes()
        );
        let facts = outcome.facts().facts_sorted().len();
        assert_eq!(outcome.into_facts().facts_sorted().len(), facts);
    }

    /// An EMPTY program closes immediately and derives nothing — the EDB is already its own
    /// universal model.
    #[test]
    fn an_empty_program_derives_nothing() {
        let outcome = run(&[], store_of(&[("a", TYPE, A)]));
        assert!(outcome.derived().is_empty());
        assert!(outcome.witnesses().is_empty());
        assert_eq!(outcome.facts().row_count(), 1);
        assert_eq!(
            certify(&[]),
            ChaseTermination::WeaklyAcyclic {
                positions: 0,
                existential_edges: 0
            }
        );
    }

    /// The witness surface is a pure function of the address: the same address always
    /// renders to the same blank node, and any component change renders to a different one.
    #[test]
    fn a_witness_surface_is_a_function_of_its_address() {
        let base = WitnessAddress {
            clause: 0,
            ordinal: 0,
            frontier: vec![iri("a")],
        };
        assert_eq!(witness_surface(&base), witness_surface(&base.clone()));
        let surface = witness_surface(&base);
        assert!(surface.starts_with("_:0.w"), "{surface}");
        assert_eq!(
            surface.len(),
            "_:0.w".len() + 2 * WITNESS_LABEL_BYTES,
            "{surface}"
        );

        let mut others = BTreeSet::from([surface]);
        for changed in [
            WitnessAddress {
                clause: 1,
                ..base.clone()
            },
            WitnessAddress {
                ordinal: 1,
                ..base.clone()
            },
            WitnessAddress {
                frontier: vec![iri("b")],
                ..base.clone()
            },
            WitnessAddress {
                frontier: vec![iri("a"), iri("a")],
                ..base.clone()
            },
            WitnessAddress {
                frontier: Vec::new(),
                ..base
            },
        ] {
            assert!(
                others.insert(witness_surface(&changed)),
                "every address component reaches the digest: {changed:?}"
            );
        }
    }

    /// The registry is idempotent: minting one address twice yields one entry and one
    /// surface.
    #[test]
    fn the_registry_is_idempotent() {
        let address = WitnessAddress {
            clause: 3,
            ordinal: 1,
            frontier: vec![iri("a"), iri("b")],
        };
        let mut registry = SkolemRegistry::new();
        assert!(registry.is_empty());
        let first = registry.mint(address.clone());
        let second = registry.mint(address.clone());
        assert_eq!(first, second);
        assert_eq!(registry.len(), 1);
        assert_eq!(registry.explain(&first), Some(&address));
    }

    /// A position renders both its own slot and its refinement, so a violation names two
    /// readable nodes.
    #[test]
    fn a_position_renders_its_slot_and_refinement() {
        let rendered = Position::of(&concept("?x", CONCEPT_C), Slot::Subject).render();
        assert_eq!(
            rendered,
            format!(
                "[s=?, p={}, o={}, g=(default graph)]",
                iri(TYPE),
                iri(CONCEPT_C)
            )
        );
        let wildcard = Position::of(&role("?x", P, "?y"), Slot::Subject).render();
        assert_eq!(
            wildcard,
            format!("[s=?, p={}, o=*, g=(default graph)]", iri(P))
        );
    }
}
