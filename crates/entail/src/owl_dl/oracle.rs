// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! A differential test of the [`hyper`](crate::owl_dl::hyper) hypertableau against a naive
//! model-enumeration oracle AND against the concept-tree
//! [`tableau`](crate::owl_dl::tableau) it replaced.
//!
//! The decision core's own corpus pins VERDICTS: a hand-built knowledge base, a hand-derived
//! answer, one assertion. That validates the answers somebody thought to write down. This
//! module validates the SEARCH: it generates small knowledge bases over a tiny signature and
//! compares the hypertableau's verdict against an oracle that decides satisfiability by
//! enumerating every interpretation over a bounded domain and evaluating each axiom directly
//! against the Description-Logic semantics.
//!
//! # Two references, because they fail differently
//!
//! Every generated knowledge base is decided FOUR times: by the hypertableau, by the
//! hypertableau again (determinism), by the hypertableau over the ALL-META encoding of the
//! same terminology (the ENCODING differential — see [`check`]), and by the concept-tree
//! tableau. The oracle is exact and
//! shares nothing with either calculus, but it is bounded — over a knowledge base that can
//! force an element beyond the named individuals it can only ever exhibit a model, never rule
//! one out (see [`bounded_domain`]). The concept-tree tableau is not exact, but it is
//! UNBOUNDED: it decides the same fragment by a completely different rule set — concept
//! structure read at search time, ancestor blocking, eight separate clash triggers — so it
//! checks the direction the oracle is silent about. The two together are what make a
//! `consistent` verdict over a successor-generating knowledge base checked rather than merely
//! unrefuted.
//!
//! A DIVERGENCE between the two calculi fails the run. It is not a tolerance and not a
//! ledger entry: both decide `SHOIQ(D)`, so exactly one of them is wrong about the semantics
//! and which one has to be established from the axioms.
//!
//! The oracle is deliberately the stupidest possible program that answers the question. It
//! guesses; it does not reason. Nothing in it is shared with the thing it checks: it never
//! reads [`Kb::meta`] or [`Kb::absorbed`] (the two encodings of the terminology) but
//! only [`Kb::tbox`], the authoritative inclusion list, and it never asks the tableau what a
//! role's extension is — it CHECKS a guessed extension against the role axioms instead of
//! computing a closure, because a check is smaller than a closure and this file's whole
//! value is that it is small enough to be read and believed.
//!
//! # The semantics, transcribed
//!
//! An interpretation fixes a finite domain `Δ = {d₀ … d_{k-1}}`, a subset of `Δ` for every
//! concept name, a subset of `Δ × Δ` for every role name, and one element of `Δ` for every
//! individual name. A concept's extension is then
//!
//! ```text
//! ⟦⊤⟧ = Δ                       ⟦⊥⟧ = ∅
//! ⟦A⟧ = the guessed subset      ⟦¬C⟧ = Δ \ ⟦C⟧
//! ⟦C ⊓ D⟧ = ⟦C⟧ ∩ ⟦D⟧           ⟦C ⊔ D⟧ = ⟦C⟧ ∪ ⟦D⟧
//! ⟦∃r.C⟧ = { x | ∃y. (x,y) ∈ ⟦r⟧ ∧ y ∈ ⟦C⟧ }
//! ⟦∀r.C⟧ = { x | ∀y. (x,y) ∈ ⟦r⟧ → y ∈ ⟦C⟧ }
//! ⟦≥n r.C⟧ = { x | |{ y | (x,y) ∈ ⟦r⟧ ∧ y ∈ ⟦C⟧ }| ≥ n }
//! ⟦≤n r.C⟧ = { x | |{ y | (x,y) ∈ ⟦r⟧ ∧ y ∈ ⟦C⟧ }| ≤ n }
//! ⟦{a₁…aₙ}⟧ = { ⟦a₁⟧ … ⟦aₙ⟧ }   ⟦∃r.Self⟧ = { x | (x,x) ∈ ⟦r⟧ }
//! ⟦r⁻⟧ = { (y,x) | (x,y) ∈ ⟦r⟧ }
//! ```
//!
//! and the interpretation is a MODEL of the knowledge base when every inclusion
//! `sub ⊑ sup` in [`Kb::tbox`] satisfies `⟦sub⟧ ⊆ ⟦sup⟧`, every [`Kb::abox_types`] pair
//! `(a, C)` has `⟦a⟧ ∈ ⟦C⟧`, every [`Kb::abox_roles`] triple `(a, p, b)` has
//! `(⟦a⟧, ⟦b⟧) ∈ ⟦p⟧`, every [`Kb::same_as`] pair agrees, every [`Kb::different_from`] pair
//! differs, and every role axiom holds of the guessed relations: `⟦r⟧ ⊆ ⟦s⟧` for each
//! sub-role recorded in [`Kb::role_sub`], `⟦r⟧ = { (y,x) | (x,y) ∈ ⟦s⟧ }` for each
//! [`Kb::inverses`] partner, transitive closure for each [`Kb::transitive`] role, no
//! symmetric pair for each [`Kb::asymmetric`] role, and an empty intersection for each
//! [`Kb::disjoint_roles`] pair. Concept extensions are held as bitmasks over `Δ`, which
//! makes each line above one machine word operation and keeps the transcription literal.
//!
//! The recursion runs over the concept table's structural decomposition rather than over a
//! [`Concept`] tree, because [`Kb::tbox`] holds concept IDS and the tree behind an id is not
//! exposed. The one thing a [`Decomp`] leaf does not carry is WHICH class an atomic
//! `Named`/`NegNamed` leaf is, so [`Case::assemble`] interns `A` and `¬A` for every class in
//! the signature up front and records the id → name correspondence itself.
//!
//! # The signature, and the arithmetic that bounds it
//!
//! Every generated knowledge base is drawn over at most three class names (`A`, `B`, `C` —
//! term ids 10…12), two role names (`r`, `s` — 20…21) and four individual names
//! (`a`, `b`, `c`, `d` — 30…33). Each property fixes a [`Signature`] naming how many of each
//! it uses and how large a domain the oracle enumerates, because the number of
//! interpretations over a domain of size `k` is
//!
//! ```text
//! 2^(k·concepts) · 2^(k²·roles) · k^individuals
//! ```
//!
//! — doubly exponential in `k` through the roles. Two role names over a four-element domain
//! is already 2.7 × 10⁸ interpretations before any concept is guessed, which is past what a
//! test may spend, so a two-role signature stops at `k = 2` and a one-role signature reaches
//! `k = 3`. [`the_enumerated_search_spaces_are_pinned`] states every property's exact search
//! space as a literal, so the cost of this file is a number in it rather than a surprise.
//!
//! Every signature names at least one individual. The DL semantics requires `Δ ≠ ∅`, and the
//! tableau's completion graph is nonempty exactly when the knowledge base has an individual
//! to build a root for, so a signature with no individual would compare a nonempty-domain
//! question against an empty-graph one.
//!
//! The signature is the ABSTRACT `SHOIQ(D)` fragment, and the concrete domain is outside it by
//! construction rather than by omission: one interpretation here fixes a single domain `Δ`,
//! while a data range is a subset of a second, disjoint value domain `Δ_D` that no amount of
//! guessing over `Δ` can stand in for. Reading a data range as a subset of `Δ` would be a
//! DIFFERENT semantics, so the generator emits none, every case's data-range table stays
//! empty, and the concrete-domain rules of the tableau never fire for these inputs — which is
//! what keeps the oracle exact over what it does cover.
//!
//! # Which direction is asserted, and which is only recorded
//!
//! **Asserted unconditionally:** the oracle exhibits a model ⇒ the tableau MUST answer
//! consistent. A bounded model is a model, so a tableau that rejects a knowledge base the
//! oracle has just exhibited a model of is unsound, full stop. That is the assertion every
//! property makes, and the failure message prints the model so the refutation is checkable
//! by hand.
//!
//! **Asserted where the bound is sufficient, counted where it is not:** the oracle finds no
//! model over any domain up to its bound while the tableau answers consistent. In general
//! this is NOT a divergence. `SHOIQ(D)` has no bounded-model property — `≥3 r.⊤` alone has no
//! model over a two-element domain and is perfectly consistent — so "no model of size ≤ k"
//! is usually silent about satisfiability, and such a case is tallied as `unbounded`.
//!
//! It is not silent for every knowledge base, and the exception is asserted. When nothing in
//! the axiom set can force an element beyond the named individuals — see
//! [`forces_unnamed_element`] — a model that exists restricts to the individuals' own
//! equivalence classes, because dropping elements only makes `∀`, `≤n` and `¬` easier and no
//! `∃`/`≥n` remains to break. Provided the signature's bound is wide enough to give every
//! individual its own element, "no model up to the bound" IS "no model", and a consistent
//! verdict is an UNSOUNDNESS — the direction that asserts something false rather than
//! withholding something true. That case fails.
//!
//! Two limits of that assertion, stated because the coverage claim depends on them.
//! `forces_unnamed_element` disqualifies any axiom set mentioning `∃`, `∀`, `≥n` or `≤n` at
//! all, so the asserted direction covers the QUANTIFIER-FREE fragment — boolean combinations,
//! nominals and self-restrictions — and not the counting or successor-generating machinery.
//! And the two signatures whose individuals outnumber their domain bound are excluded
//! entirely. A property that produced only `unbounded` cases would be asserting nothing in
//! this direction, which is why each property also asserts that a substantial share of its
//! cases were decided by an exhibited model.
//!
//! What the tableau's `false` does get checked against is the strongest thing available:
//! [`Case::smallest_model`] searches EVERY domain size from 1 up to the signature's bound, so
//! a `refuted` tally entry means the oracle failed to find a model at `k` and at every size
//! below it, and the one-role signatures push that bound to 3 — one domain size beyond what
//! the two-role signatures can afford. Any model found at any of those sizes turns the case
//! back into the unconditional assertion above.
//!
//! A run that hits its step cap has no verdict to compare, so it is skipped and tallied as
//! `exhausted`; each property asserts that share stays negligible, so the suite cannot quietly
//! degenerate into skipping everything. The cap this suite decides under is narrowed from the
//! tableau's own — see [`STEP_CAP`] for the measurement that forces it.
//!
//! # Determinism
//!
//! Each property runs its own [`TestRunner`] over a FIXED [`RngAlgorithm::ChaCha`] seed, so
//! the same knowledge bases are generated on every run, on every machine, and a failure
//! reproduces. Nothing here reads a clock or a `HashMap`. The hypertableau's own determinism
//! is itself asserted: every generated knowledge base is decided twice and the two
//! [`Decision`](graph::Decision)s must be the same WHOLE struct — verdict, round count, both
//! stop flags and all three shape counters — which is also why each property's round total can
//! be held to a measured ceiling ([`run_property`]) rather than to a timing.

use std::cell::RefCell;

use proptest::prelude::*;
use proptest::strategy::{BoxedStrategy, Union};
use proptest::test_runner::{Config, RngAlgorithm, TestCaseError, TestRng, TestRunner};

use crate::owl_dl::Kb;
use crate::owl_dl::concept::{Concept, Decomp, Role};
use crate::owl_dl::graph::{self, Assumptions};
use crate::owl_dl::{hyper, tableau};

// ── The signature ───────────────────────────────────────────────────────────────

/// The class term ids a generated concept's named leaves are drawn from (`A`, `B`, `C`).
const CONCEPT_NAMES: [u32; 3] = [10, 11, 12];

/// The object-property term ids a generated role is drawn from (`r`, `s`).
const ROLE_NAMES: [u32; 2] = [20, 21];

/// The individual term ids a generated nominal or assertion is drawn from
/// (`a`, `b`, `c`, `d`).
const INDIVIDUAL_NAMES: [u32; 4] = [30, 31, 32, 33];

/// The largest domain any property enumerates, and so the width of the bitmask a subset of
/// the domain is held in.
const MAX_DOMAIN: usize = 3;

/// A readable name for a signature term id, for a failure message.
fn term_name(id: u32) -> String {
    let letters = ["A", "B", "C"];
    if let Some(i) = CONCEPT_NAMES.iter().position(|&x| x == id) {
        return letters[i].to_owned();
    }
    if let Some(i) = ROLE_NAMES.iter().position(|&x| x == id) {
        return ["r", "s"][i].to_owned();
    }
    if let Some(i) = INDIVIDUAL_NAMES.iter().position(|&x| x == id) {
        return ["a", "b", "c", "d"][i].to_owned();
    }
    format!("#{id}")
}

/// The finite signature a generated knowledge base is drawn over, and the domain sizes the
/// oracle enumerates for it.
#[derive(Debug, Clone, Copy)]
struct Signature {
    /// How many of [`CONCEPT_NAMES`] the generator may use.
    concepts: usize,
    /// How many of [`ROLE_NAMES`] the generator may use; zero for a purely boolean property.
    roles: usize,
    /// How many of [`INDIVIDUAL_NAMES`] the generator may use; always at least one, so the
    /// domain the two sides compare over is nonempty.
    individuals: usize,
    /// The largest domain the oracle enumerates. It enumerates every smaller one too, so a
    /// model of ANY size up to this bound is found.
    max_domain: usize,
}

impl Signature {
    /// The class term ids this signature admits.
    fn concept_names(self) -> &'static [u32] {
        &CONCEPT_NAMES[..self.concepts]
    }

    /// The role term ids this signature admits.
    fn role_names(self) -> &'static [u32] {
        &ROLE_NAMES[..self.roles]
    }

    /// The individual term ids this signature admits.
    fn individual_names(self) -> &'static [u32] {
        &INDIVIDUAL_NAMES[..self.individuals]
    }

    /// How many interpretations exist over a domain of `size` elements:
    /// `2^(size·concepts) · 2^(size²·roles) · size^individuals`.
    fn interpretations(self, size: usize) -> u64 {
        let concepts = 1u64 << (size * self.concepts);
        let roles = 1u64 << (size * size * self.roles);
        let individuals = (size as u64).pow(self.individuals as u32);
        concepts * roles * individuals
    }

    /// How many interpretations the oracle enumerates in the worst case for this signature —
    /// every interpretation over every domain size from 1 up to [`Signature::max_domain`].
    fn search_space(self) -> u64 {
        (1..=self.max_domain).map(|k| self.interpretations(k)).sum()
    }
}

/// The widest signature: every construct, three classes, two roles, three individuals. Two
/// roles cap the domain at two elements.
const WIDE: Signature = Signature {
    concepts: 3,
    roles: 2,
    individuals: 3,
    max_domain: 2,
};

/// The deepest-domain general signature: one role buys a third domain element, so a tableau
/// `false` here is matched by the oracle failing at sizes 1, 2 and 3.
const DEEP: Signature = Signature {
    concepts: 2,
    roles: 1,
    individuals: 2,
    max_domain: 3,
};

/// Nominals under inverse roles and cardinality.
const NOMINAL_INVERSE: Signature = Signature {
    concepts: 2,
    roles: 1,
    individuals: 2,
    max_domain: 3,
};

/// Multi-member nominals against `owl:differentFrom` — three individuals to have something
/// to enumerate over, one class to keep the third domain element affordable.
const ONE_OF: Signature = Signature {
    concepts: 1,
    roles: 1,
    individuals: 3,
    max_domain: 3,
};

/// Qualified cardinality against a role hierarchy: two roles, so the domain stops at two.
const ROLE_HIERARCHY: Signature = Signature {
    concepts: 2,
    roles: 2,
    individuals: 3,
    max_domain: 2,
};

/// Complement against disjunction: no role at all, which is what makes a three-element
/// domain and three class names affordable together.
const BOOLEAN: Signature = Signature {
    concepts: 3,
    roles: 0,
    individuals: 3,
    max_domain: 3,
};

/// The ABSORPTION signature: quantifier-free, so `bounded_domain` holds of every case it
/// generates and the over-permissive direction is asserted rather than counted.
///
/// No role at all is what buys three class names AND a three-element domain together, and
/// three class names are what the shapes this family is about need: `A ≡ B ⊓ C` alongside
/// `A ⊑ D` is a conjunctive antecedent whose consequent splits, and `A ≡ B ⊔ C` alongside a
/// disjointness axiom is a disjunctive antecedent that splits the other way.
const ABSORPTION: Signature = Signature {
    concepts: 3,
    roles: 0,
    individuals: 3,
    max_domain: 3,
};

/// The REPORTED-SHAPE signature: three classes and two roles, at the domain bound two roles
/// allow.
///
/// Sized for the interaction the reported ontology is made of — a `∀`-restriction whose filler
/// is an intersection, an exact cardinality, and an inverse role — which needs a class for the
/// restricted concept, one for the intersection's named conjunct and one for the inner `∀`'s
/// filler, plus a role to quantify over and a second to count.
const REPORTED: Signature = Signature {
    concepts: 3,
    roles: 2,
    individuals: 2,
    max_domain: 2,
};

/// The CYCLE signature: two classes over one role, which is what buys a three-element domain.
///
/// One role is all a cycle needs — `A ≡ ∃r.A` uses one, and the two-cycle
/// `A ≡ ∃r.B`, `B ≡ ∃r⁻.A` uses the same one in both directions — and spending the second on
/// nothing would cost the third domain element, which is exactly the element a two-cycle's
/// smallest model needs.
const CYCLE: Signature = Signature {
    concepts: 2,
    roles: 1,
    individuals: 2,
    max_domain: 3,
};

/// The signature the hand-written regressions are stated over: everything they need
/// (two classes, two roles, four individuals) at the domain bound two roles allow.
const HAND: Signature = Signature {
    concepts: 2,
    roles: 2,
    individuals: 4,
    max_domain: 2,
};

// ── Interpretations ─────────────────────────────────────────────────────────────

/// A binary relation over a bounded domain, held as one bitmask row per element.
#[derive(Clone, Copy)]
struct Relation {
    /// `rows[x]` has bit `y` set exactly when `(x, y)` is in the relation — the extension of
    /// the named role.
    rows: [u32; MAX_DOMAIN],
    /// `cols[y]` has bit `x` set exactly when `(x, y)` is in the relation — the extension of
    /// the inverse role `r⁻`, which the semantics defines as `{ (y,x) | (x,y) ∈ ⟦r⟧ }`.
    cols: [u32; MAX_DOMAIN],
}

impl Relation {
    /// The empty relation.
    const EMPTY: Self = Self {
        rows: [0; MAX_DOMAIN],
        cols: [0; MAX_DOMAIN],
    };

    /// The relation whose `size × size` incidence bits are the low bits of `code`, bit
    /// `x·size + y` standing for the pair `(x, y)`.
    fn decode(code: u64, size: usize) -> Self {
        let mut out = Self::EMPTY;
        for x in 0..size {
            for y in 0..size {
                if (code >> (x * size + y)) & 1 == 1 {
                    out.rows[x] |= 1 << y;
                    out.cols[y] |= 1 << x;
                }
            }
        }
        out
    }

    /// Whether every pair of this relation is also a pair of `other` (`⟦r⟧ ⊆ ⟦s⟧`).
    fn subset_of(&self, other: &Self, size: usize) -> bool {
        (0..size).all(|x| self.rows[x] & !other.rows[x] == 0)
    }

    /// Whether this relation is exactly the transpose of `other` (`⟦r⟧ = ⟦s⁻⟧`).
    fn is_inverse_of(&self, other: &Self, size: usize) -> bool {
        (0..size).all(|x| self.rows[x] == other.cols[x])
    }

    /// Whether this relation is transitively closed: `(x,y)` and `(y,z)` present implies
    /// `(x,z)` present.
    fn is_transitive(&self, size: usize) -> bool {
        (0..size).all(|x| {
            (0..size)
                .filter(|&y| (self.rows[x] >> y) & 1 == 1)
                .all(|y| self.rows[y] & !self.rows[x] == 0)
        })
    }

    /// Whether no pair `(x, y)` is present together with `(y, x)` — self-loops included,
    /// which is how asymmetry subsumes irreflexivity.
    fn is_asymmetric(&self, size: usize) -> bool {
        (0..size).all(|x| self.rows[x] & self.cols[x] == 0)
    }

    /// Whether the two relations share no pair (`⟦r⟧ ∩ ⟦s⟧ = ∅`).
    fn is_disjoint_from(&self, other: &Self, size: usize) -> bool {
        (0..size).all(|x| self.rows[x] & other.rows[x] == 0)
    }
}

/// One interpretation `I = (Δ, ·ᴵ)` over the domain `Δ = {d₀ … d_{size-1}}`.
#[derive(Clone, Copy)]
struct Interpretation {
    /// `|Δ|`.
    size: usize,
    /// The bitmask of all of `Δ` — the extension of `⊤`.
    full: u32,
    /// `concepts[i]` is the extension of the `i`-th class name of the signature.
    concepts: [u32; CONCEPT_NAMES.len()],
    /// `roles[i]` is the extension of the `i`-th role name of the signature.
    roles: [Relation; ROLE_NAMES.len()],
    /// `individuals[i]` is the element the `i`-th individual name of the signature denotes.
    individuals: [usize; INDIVIDUAL_NAMES.len()],
}

/// The marker [`Case::named`] carries for a concept id that is not an atomic class leaf.
const NOT_A_CLASS: u8 = u8::MAX;

// ── The generated knowledge base and its oracle ─────────────────────────────────

/// One generated axiom, in the vocabulary [`Kb`] holds directly.
#[derive(Debug, Clone)]
enum Axiom {
    /// A general concept inclusion `sub ⊑ sup`.
    Gci(Concept, Concept),
    /// A concept assertion `a : C`.
    Type(u32, Concept),
    /// A role assertion `a r b`.
    RoleAssertion(u32, u32, u32),
    /// `a owl:sameAs b`.
    SameAs(u32, u32),
    /// `a owl:differentFrom b`.
    DifferentFrom(u32, u32),
    /// `sub rdfs:subPropertyOf sup`.
    SubRole(u32, u32),
    /// `r owl:inverseOf s` (`r ≡ s⁻`, and with `r = s` the symmetry axiom).
    InverseOf(u32, u32),
    /// `r rdf:type owl:TransitiveProperty`.
    Transitive(u32),
    /// `r rdf:type owl:AsymmetricProperty`.
    Asymmetric(u32),
    /// `r owl:propertyDisjointWith s`.
    DisjointRoles(u32, u32),
}

/// A generated knowledge base, the signature it was drawn over, and the atomic-leaf
/// correspondence the oracle's recursion needs.
struct Case {
    /// The knowledge base exactly as the tableau receives it.
    kb: Kb,
    /// The signature it was drawn over.
    sig: Signature,
    /// The axioms it was built from, for a failure message.
    axioms: Vec<Axiom>,
    /// Concept id → index into [`Signature::concept_names`] for each positive atomic class
    /// leaf, [`NOT_A_CLASS`] elsewhere.
    named: Vec<u8>,
    /// Concept id → index into [`Signature::concept_names`] for each negated atomic class
    /// leaf, [`NOT_A_CLASS`] elsewhere.
    neg_named: Vec<u8>,
}

impl Case {
    /// Build the knowledge base the axioms describe, under the encoding this crate decides
    /// with: every faithful antecedent absorbed into a guarded clause, everything else
    /// internalized.
    ///
    /// Every general concept inclusion goes through [`Kb::push_gci`] and is clausified by
    /// [`Kb::finalize`] — the same path the reverse mapping takes — so both terminology
    /// encodings are exercised, while the oracle reads neither and only ever consults
    /// [`Kb::tbox`].
    ///
    /// Every individual the signature names is declared, whether an axiom mentions it or
    /// not, so the tableau always has a root node and the oracle always has an element to
    /// map it to.
    fn assemble(sig: Signature, axioms: &[Axiom]) -> Self {
        Self::encoded(sig, axioms, false)
    }

    /// The same knowledge base under a chosen ENCODING of its terminology.
    ///
    /// `internalize_only` builds the textbook encoding — every inclusion becomes
    /// `nnf(¬C ⊔ D)` in every node's label, nothing is absorbed — which is what the encoding
    /// differential in [`check`] decides beside the absorbed one. Absorption is a claim that
    /// two encodings of one terminology have one meaning, and the only way to check a claim
    /// about two encodings is to have both.
    fn encoded(sig: Signature, axioms: &[Axiom], internalize_only: bool) -> Self {
        let mut kb = Kb::empty();
        kb.internalize_only = internalize_only;
        for &a in sig.individual_names() {
            kb.individuals.insert(a);
        }
        for axiom in axioms {
            match axiom {
                Axiom::Gci(sub, sup) => kb.push_gci(sub.clone(), sup.clone()),
                Axiom::Type(a, c) => {
                    let cid = kb.table.intern(c.clone());
                    kb.abox_types.push((*a, cid));
                }
                Axiom::RoleAssertion(a, p, b) => kb.abox_roles.push((*a, *p, *b)),
                Axiom::SameAs(a, b) => kb.same_as.push((*a, *b)),
                Axiom::DifferentFrom(a, b) => kb.different_from.push((*a, *b)),
                Axiom::SubRole(sub, sup) => {
                    kb.role_sub.entry(*sup).or_default().insert(*sub);
                }
                Axiom::InverseOf(r, s) => {
                    kb.inverses.entry(*r).or_default().insert(*s);
                    kb.inverses.entry(*s).or_default().insert(*r);
                }
                Axiom::Transitive(r) => {
                    kb.transitive.insert(*r);
                }
                Axiom::Asymmetric(r) => {
                    kb.asymmetric.insert(*r);
                }
                Axiom::DisjointRoles(r, s) => {
                    kb.disjoint_roles.insert((*r, *s));
                    kb.disjoint_roles.insert((*s, *r));
                }
            }
        }
        // Pin the atomic-leaf correspondence. A `Decomp::Named` leaf does not say which
        // class it is (the tableau reads it opaquely), so the oracle interns `A` and `¬A`
        // for every class in the signature and remembers the ids it got back. Interning is
        // store-once, so these are the very ids the generated axioms already use.
        let mut pinned: Vec<(u32, u8)> = Vec::new();
        let mut pinned_negated: Vec<(u32, u8)> = Vec::new();
        for (index, &class) in sig.concept_names().iter().enumerate() {
            let index = index as u8;
            pinned.push((kb.table.intern(Concept::Named(class)), index));
            pinned_negated.push((
                kb.table
                    .intern(Concept::Not(Box::new(Concept::Named(class)))),
                index,
            ));
        }
        kb.finalize();
        let mut named = vec![NOT_A_CLASS; kb.table.len()];
        let mut neg_named = vec![NOT_A_CLASS; kb.table.len()];
        for (id, index) in pinned {
            named[id as usize] = index;
        }
        for (id, index) in pinned_negated {
            neg_named[id as usize] = index;
        }
        Self {
            kb,
            sig,
            axioms: axioms.to_vec(),
            named,
            neg_named,
        }
    }

    /// The signature index of the role name with term id `p`.
    fn role_index(&self, p: u32) -> usize {
        self.sig
            .role_names()
            .iter()
            .position(|&q| q == p)
            .expect("a generated role is a signature role")
    }

    /// The signature index of the individual name with term id `a`.
    fn individual_index(&self, a: u32) -> usize {
        self.sig
            .individual_names()
            .iter()
            .position(|&b| b == a)
            .expect("a generated individual is a signature individual")
    }

    /// The rows of `⟦role⟧` under `i`: the guessed relation for a named role, its transpose
    /// for an inverse one.
    fn rows<'a>(&self, i: &'a Interpretation, role: Role) -> &'a [u32; MAX_DOMAIN] {
        match role {
            Role::Named(p) => &i.roles[self.role_index(p)].rows,
            Role::Inv(p) => &i.roles[self.role_index(p)].cols,
        }
    }

    /// `⟦c⟧` under `i`, as a bitmask over `Δ` — the semantics table in the module docs, one
    /// arm per line.
    fn extension(&self, i: &Interpretation, c: u32) -> u32 {
        match *self.kb.table.decomp(c) {
            Decomp::Top => i.full,
            Decomp::Bottom => 0,
            Decomp::Named => i.concepts[self.class_index(&self.named, c)],
            Decomp::NegNamed => i.full & !i.concepts[self.class_index(&self.neg_named, c)],
            Decomp::And(ref cs) => cs.iter().fold(i.full, |m, &c| m & self.extension(i, c)),
            Decomp::Or(ref cs) => cs.iter().fold(0, |m, &c| m | self.extension(i, c)),
            Decomp::Some(role, filler) => {
                let f = self.extension(i, filler);
                let rows = self.rows(i, role);
                elements_where(i.size, |x| rows[x] & f != 0)
            }
            Decomp::All(role, filler) => {
                let f = self.extension(i, filler);
                let rows = self.rows(i, role);
                elements_where(i.size, |x| rows[x] & !f == 0)
            }
            Decomp::Min(n, role, filler) => {
                let f = self.extension(i, filler);
                let rows = self.rows(i, role);
                elements_where(i.size, |x| (rows[x] & f).count_ones() >= n)
            }
            Decomp::Max(n, role, filler) => {
                let f = self.extension(i, filler);
                let rows = self.rows(i, role);
                elements_where(i.size, |x| (rows[x] & f).count_ones() <= n)
            }
            Decomp::Nominal(ref members) => self.nominal_extension(i, members),
            Decomp::NegNominal(ref members) => i.full & !self.nominal_extension(i, members),
            Decomp::SelfRestriction(role) => {
                let rows = self.rows(i, role);
                elements_where(i.size, |x| (rows[x] >> x) & 1 == 1)
            }
            Decomp::NegSelfRestriction(role) => {
                let rows = self.rows(i, role);
                elements_where(i.size, |x| (rows[x] >> x) & 1 == 0)
            }
            // A concrete-domain leaf cannot occur: the generator emits no data range, so
            // every case's data-range table is empty. See the module docs — the abstract
            // domain is the oracle's whole scope, by construction rather than by omission.
            Decomp::Data(_) | Decomp::NegData(_) => {
                unreachable!("a data range reached an oracle whose signature is purely abstract")
            }
        }
    }

    /// `⟦{a₁ … aₙ}⟧` under `i` — the elements the listed individual names denote.
    fn nominal_extension(&self, i: &Interpretation, members: &[u32]) -> u32 {
        members
            .iter()
            .fold(0, |m, &a| m | 1 << i.individuals[self.individual_index(a)])
    }

    /// The signature class index a `map` entry records for concept id `c`.
    fn class_index(&self, map: &[u8], c: u32) -> usize {
        let index = map[c as usize];
        assert!(
            index != NOT_A_CLASS,
            "an atomic class leaf outside the signature reached the oracle"
        );
        index as usize
    }

    /// Whether `i` satisfies every ROLE axiom — the constraints the extensions of the roles
    /// themselves must meet, CHECKED against a guess rather than closed over one.
    fn role_axioms_hold(&self, i: &Interpretation) -> bool {
        for (&sup, subs) in &self.kb.role_sub {
            let sup = &i.roles[self.role_index(sup)];
            if !subs
                .iter()
                .all(|&sub| i.roles[self.role_index(sub)].subset_of(sup, i.size))
            {
                return false;
            }
        }
        for (&r, partners) in &self.kb.inverses {
            let r = &i.roles[self.role_index(r)];
            if !partners
                .iter()
                .all(|&s| r.is_inverse_of(&i.roles[self.role_index(s)], i.size))
            {
                return false;
            }
        }
        if !self
            .kb
            .transitive
            .iter()
            .all(|&r| i.roles[self.role_index(r)].is_transitive(i.size))
        {
            return false;
        }
        if !self
            .kb
            .asymmetric
            .iter()
            .all(|&r| i.roles[self.role_index(r)].is_asymmetric(i.size))
        {
            return false;
        }
        self.kb.disjoint_roles.iter().all(|&(left, right)| {
            let left = &i.roles[self.role_index(left)];
            left.is_disjoint_from(&i.roles[self.role_index(right)], i.size)
        })
    }

    /// Whether `i` satisfies every general concept inclusion of [`Kb::tbox`] — the
    /// authoritative list, not either of the tableau's encodings of it.
    fn tbox_holds(&self, i: &Interpretation) -> bool {
        self.kb
            .tbox
            .iter()
            .all(|&(sub, sup)| self.extension(i, sub) & !self.extension(i, sup) == 0)
    }

    /// Whether `i` satisfies every equality and inequality assertion — integer comparisons
    /// between the elements two individual names denote, with no concept in sight.
    fn identities_hold(&self, i: &Interpretation) -> bool {
        let element = |a: u32| i.individuals[self.individual_index(a)];
        self.kb
            .same_as
            .iter()
            .all(|&(a, b)| element(a) == element(b))
            && self
                .kb
                .different_from
                .iter()
                .all(|&(a, b)| element(a) != element(b))
    }

    /// Whether `i` satisfies every role assertion `a p b`: `(⟦a⟧, ⟦b⟧) ∈ ⟦p⟧`.
    fn role_assertions_hold(&self, i: &Interpretation) -> bool {
        self.kb.abox_roles.iter().all(|&(a, p, b)| {
            let from = i.individuals[self.individual_index(a)];
            let to = i.individuals[self.individual_index(b)];
            (i.roles[self.role_index(p)].rows[from] >> to) & 1 == 1
        })
    }

    /// Whether `i` satisfies every concept assertion `a : C`: `⟦a⟧ ∈ ⟦C⟧`.
    fn type_assertions_hold(&self, i: &Interpretation) -> bool {
        self.kb.abox_types.iter().all(|&(a, c)| {
            (self.extension(i, c) >> i.individuals[self.individual_index(a)]) & 1 == 1
        })
    }

    /// Whether `i` is a model of the whole knowledge base.
    ///
    /// A conjunction, so the order of the conjuncts changes nothing about the answer — and
    /// the ones that need no concept recursion are asked first, because the innermost
    /// enumeration loop varies only which element each individual name denotes, and a
    /// `different_from` pair rejects most of those assignments with an integer comparison.
    fn models(&self, i: &Interpretation) -> bool {
        self.role_axioms_hold(i)
            && self.identities_hold(i)
            && self.role_assertions_hold(i)
            && self.type_assertions_hold(i)
            && self.tbox_holds(i)
    }

    /// A model over a domain of exactly `size` elements, by enumerating every interpretation
    /// over that domain.
    ///
    /// The role extensions are the outer loop and an interpretation whose roles already
    /// violate a role axiom skips the inner loops — a pure fast path for a rejection
    /// [`Case::models`] would make anyway, which is why [`Case::models`] still checks the
    /// role axioms itself and remains the complete definition.
    fn model_at(&self, size: usize) -> Option<Interpretation> {
        let relation_codes = 1u64 << (size * size);
        let concept_codes = 1u64 << size;
        let elements = size as u64;
        let mut i = Interpretation {
            size,
            full: (1u32 << size) - 1,
            concepts: [0; CONCEPT_NAMES.len()],
            roles: [Relation::EMPTY; ROLE_NAMES.len()],
            individuals: [0; INDIVIDUAL_NAMES.len()],
        };
        for role_code in 0..relation_codes.pow(self.sig.roles as u32) {
            let mut rest = role_code;
            for slot in 0..self.sig.roles {
                i.roles[slot] = Relation::decode(rest % relation_codes, size);
                rest /= relation_codes;
            }
            if !self.role_axioms_hold(&i) {
                continue;
            }
            for concept_code in 0..concept_codes.pow(self.sig.concepts as u32) {
                let mut rest = concept_code;
                for slot in 0..self.sig.concepts {
                    i.concepts[slot] = (rest % concept_codes) as u32;
                    rest /= concept_codes;
                }
                for individual_code in 0..elements.pow(self.sig.individuals as u32) {
                    let mut rest = individual_code;
                    for slot in 0..self.sig.individuals {
                        i.individuals[slot] = (rest % elements) as usize;
                        rest /= elements;
                    }
                    if self.models(&i) {
                        return Some(i);
                    }
                }
            }
        }
        None
    }

    /// A model over the smallest domain that has one, searching every size from 1 up to the
    /// signature's bound.
    fn smallest_model(&self) -> Option<Interpretation> {
        (1..=self.sig.max_domain).find_map(|size| self.model_at(size))
    }

    /// The axioms, one per line, for a failure message.
    fn axioms_text(&self) -> String {
        self.axioms
            .iter()
            .map(|axiom| format!("  {axiom:?}"))
            .collect::<Vec<String>>()
            .join("\n")
    }

    /// A model, rendered so the refutation it witnesses can be checked by hand.
    fn model_text(&self, i: &Interpretation) -> String {
        let set = |mask: u32| -> String {
            let members: Vec<String> = (0..i.size)
                .filter(|&x| (mask >> x) & 1 == 1)
                .map(|x| format!("d{x}"))
                .collect();
            format!("{{{}}}", members.join(", "))
        };
        let mut lines: Vec<String> = vec![format!("  Δ = {}", set(i.full))];
        for (index, &class) in self.sig.concept_names().iter().enumerate() {
            lines.push(format!(
                "  ⟦{}⟧ = {}",
                term_name(class),
                set(i.concepts[index])
            ));
        }
        for (index, &role) in self.sig.role_names().iter().enumerate() {
            let pairs: Vec<String> = (0..i.size)
                .flat_map(|x| (0..i.size).map(move |y| (x, y)))
                .filter(|&(x, y)| (i.roles[index].rows[x] >> y) & 1 == 1)
                .map(|(x, y)| format!("(d{x}, d{y})"))
                .collect();
            lines.push(format!(
                "  ⟦{}⟧ = {{{}}}",
                term_name(role),
                pairs.join(", ")
            ));
        }
        for (index, &individual) in self.sig.individual_names().iter().enumerate() {
            lines.push(format!(
                "  ⟦{}⟧ = d{}",
                term_name(individual),
                i.individuals[index]
            ));
        }
        lines.join("\n")
    }
}

/// The bitmask of the elements of a `size`-element domain satisfying `pred`.
fn elements_where(size: usize, mut pred: impl FnMut(usize) -> bool) -> u32 {
    let mut out = 0;
    for x in 0..size {
        if pred(x) {
            out |= 1 << x;
        }
    }
    out
}

// ── The differential check ──────────────────────────────────────────────────────

/// How the cases of one property were resolved.
#[derive(Debug, Default)]
struct Tally {
    /// The oracle exhibited a model, so the tableau's `consistent` was ASSERTED.
    modelled: u32,
    /// The tableau answered inconsistent and the oracle found no model over any domain up to
    /// the signature's bound — agreement, as far as a bounded domain can show it.
    refuted: u32,
    /// The tableau answered consistent, the oracle found no bounded model, and the knowledge
    /// base COULD force an element beyond the named individuals — so the bound is silent and
    /// nothing is asserted. A case where nothing can force such an element never lands here:
    /// it is asserted instead, and a consistent verdict there is a failure.
    unbounded: u32,
    /// The tableau ran out of steps, so there was no verdict to compare.
    exhausted: u32,
    /// Cases where the oracle found NO model AND the bound was sufficient, so the
    /// over-permissive direction was genuinely asserted rather than counted.
    ///
    /// Reported and floored per property, because a direction that binds zero times is not
    /// being checked — and an assertion that never fires reads exactly like one that passes.
    bound_asserted: u32,
    /// Cases where BOTH decision cores finished, so their verdicts were compared and agreed.
    ///
    /// Reported and floored for the same reason `bound_asserted` is: a differential that
    /// compares nothing passes silently. This is the population over which zero divergence
    /// between the hypertableau and the concept-tree tableau is asserted.
    differential: u32,
    /// Cases where BOTH TBox ENCODINGS finished, so the absorbed clause table and the
    /// all-meta internalization were compared and agreed.
    ///
    /// Floored for the reason `differential` is: an encoding differential that compares
    /// nothing passes silently, and absorption's whole soundness argument is the claim this
    /// population checks.
    encodings: u32,
    /// Derivation rounds the hypertableau spent over EVERY case of this property, summed.
    ///
    /// The property's cost, in the cap's own units. Ceilinged in [`run_property`] against a
    /// measured literal, because the whole corpus is generated from a fixed seed and decided
    /// by a deterministic search: this number is reproducible, so a change that makes the
    /// search do materially more work has a place to show up other than the wall clock.
    steps: u64,
    /// The most rounds any ONE case of this property spent.
    ///
    /// What [`STEP_CAP`] has to accommodate, and the number that says whether the cap is
    /// truncating a search that would otherwise have decided.
    max_case_steps: u64,
    /// The largest completion graph any case of this property built, in nodes.
    peak_nodes: u64,
    /// `⊔`-rule applications over every case, summed.
    disjunctions: u64,
    /// The deepest any case's branch stack got.
    peak_depth: u64,
}

impl Tally {
    /// How many cases were seen.
    fn total(&self) -> u32 {
        self.modelled + self.refuted + self.unbounded + self.exhausted
    }

    /// Fold one hypertableau decision's cost in: work sums, sizes peak.
    fn spent(&mut self, decision: &graph::Decision) {
        self.steps = self.steps.saturating_add(decision.steps);
        self.max_case_steps = self.max_case_steps.max(decision.steps);
        self.peak_nodes = self.peak_nodes.max(decision.peak_nodes);
        self.disjunctions = self.disjunctions.saturating_add(decision.disjunctions);
        self.peak_depth = self.peak_depth.max(decision.peak_depth);
    }
}

/// The step cap this suite decides a generated knowledge base under.
///
/// The tableau's own [`step_cap`](tableau::step_cap) is a termination backstop sized for a
/// real ontology, and a caller may narrow it but never widen it. Narrowing is what this suite
/// needs, because the cap counts saturation ROUNDS while the work inside ONE round grows with
/// the completion graph, and the graph grows geometrically per round — so wall time is
/// superlinear in the cap, not proportional to it. A three-axiom knowledge base built from
/// `≥2 s.∀s⁻.{b,c}` and two `≤n` inclusions costs about twenty times as much at 1000 rounds as
/// at 300, and about thirty times that again at 3000, which is enough to make one generated
/// case outlast the whole rest of the suite. A property test that has to decide thousands of
/// adversarial knowledge bases cannot let one of them run unbounded, so it stops such a search
/// early, where [`Decision::exhausted`](tableau::Decision) makes the truncation VISIBLE and
/// [`check`] skips the case rather than reading "no branch succeeded yet" as a verdict. The
/// skipped share is asserted to stay negligible in [`run_property`], so narrowing the cap
/// cannot quietly become a way of not testing anything.
///
/// # Where the number comes from
///
/// It is MEASURED, and the measurement is this: over the whole corpus, the most rounds any
/// case that DECIDES spends is 439 — one knowledge base in `complement ⊗ disjunction`, whose
/// cost the narrowest-first `⊔`-rule raised from 178 (see
/// [`Hyper::find_branch`](crate::owl_dl::hyper) for why that trade is worth making). 500 is
/// that maximum plus about a seventh.
///
/// The criterion is deliberate: a case the calculus can decide must not be reported as
/// exhausted, because an exhausted case is one [`check`] compares NEITHER differential on —
/// so a cap set below a decidable case's cost quietly shrinks what the suite checks while
/// every assertion still passes. A cap of 400 did exactly that to the 439-round case.
///
/// What the cap does NOT try to accommodate is the one case that no affordable cap decides.
/// The `wide` corpus contains a knowledge base whose completion graph simply grows with
/// whatever it is given — 139 nodes at a cap of 400, 172 at 500, 205 at 600, 339 at 1000 —
/// and it exhausts at every one of them. Chasing it is what the superlinear cost above buys
/// nothing for: the whole suite costs about 6 s at a cap of 400, 7.4 s at 450, 9.2 s at 500
/// and 15 s at 600, and almost all of that increase is that single case running longer before
/// being truncated anyway. So the cap is set to decide everything decidable and to truncate
/// that one, which the ≤5% exhausted quota in [`run_property`] absorbs at 1 case in 8,900.
const STEP_CAP: u64 = 500;

/// Whether "no model up to the signature's bound" means "no model" for this knowledge base.
///
/// Two conditions, both necessary. Nothing in the axioms may force an element beyond the named
/// individuals (see [`forces_unnamed_element`]), and the enumeration must be wide enough to
/// give every individual its own element — three individuals forced apart need three, and two
/// of the signatures enumerate only two.
fn bounded_domain(sig: Signature, axioms: &[Axiom]) -> bool {
    sig.individuals <= sig.max_domain
        && axioms.iter().all(|axiom| match axiom {
            Axiom::Gci(sub, sup) => !forces_unnamed_element(sub) && !forces_unnamed_element(sup),
            Axiom::Type(_, c) => !forces_unnamed_element(c),
            _ => true,
        })
}

/// Whether `c` can force the domain to hold an element none of the named individuals
/// denotes.
///
/// `∃r.C` and `≥n r.C` do so outright. `≤n r.C` and `∀r.C` do so UNDER NEGATION, because
/// `¬(≤n r.C)` is `≥(n+1) r.C` and `¬∀r.C` is `∃r.¬C` — a reading that is easy to miss and
/// whose omission would make the bounded-domain test below unsound in the one direction it
/// exists to check. Rather than track polarity, any occurrence of the four counts, which
/// over-approximates and can only ever DECLINE to assert.
fn forces_unnamed_element(c: &Concept) -> bool {
    match c {
        Concept::Some(..) | Concept::All(..) | Concept::Min(..) | Concept::Max(..) => true,
        Concept::Not(inner) => forces_unnamed_element(inner),
        Concept::And(members) | Concept::Or(members) => members.iter().any(forces_unnamed_element),
        Concept::Top
        | Concept::Bottom
        | Concept::Named(_)
        | Concept::Nominal(_)
        | Concept::SelfRestriction(_)
        | Concept::Data(_) => false,
    }
}

/// Hold one decision's three shape counters to the invariants that give them meaning.
///
/// Three of them, and each rules out a different way a counter can be plumbed wrong:
///
/// * a search that spent a round built a graph, so `peak_nodes ≥ 1` — a counter observed at
///   the wrong point in the loop, or never observed at all, reads zero here. That direction
///   holds for THIS suite rather than in general: every signature names at least one
///   individual and [`Case::encoded`] declares every one of them, so the completion graph is
///   never empty. A TBox-only question elsewhere in the crate legitimately has no root at all;
/// * a branch stack cannot be deeper than the number of case splits that pushed onto it, so
///   `peak_depth ≤ disjunctions` — a depth taken from the wrong quantity fails this;
/// * a case split costs at least the round that derived the head it split on, so
///   `disjunctions ≤ steps` — a count incremented per ALTERNATIVE rather than per rule
///   application would exceed the round count on a wide disjunction.
///
/// Applied to BOTH cores, which is what keeps the reference tableau's counters honest: it is
/// `cfg(test)`-only and no service reads its numbers, so without this they would be three
/// fields nothing ever looks at.
fn counters_are_coherent(
    core: &str,
    decision: &graph::Decision,
    case: &Case,
) -> Result<(), TestCaseError> {
    let complaint = |detail: &str| {
        TestCaseError::fail(format!(
            "the {core}'s shape counters are incoherent: {detail}\n{decision:?}\naxioms:\n{}",
            case.axioms_text()
        ))
    };
    if decision.steps > 0 && decision.peak_nodes == 0 {
        return Err(complaint("it spent rounds over a graph of no nodes"));
    }
    if decision.peak_depth > decision.disjunctions {
        return Err(complaint("its branch stack is deeper than its case splits"));
    }
    if decision.disjunctions > decision.steps {
        return Err(complaint("it case split more often than it derived"));
    }
    Ok(())
}

/// Check one generated knowledge base, recording how it resolved.
///
/// Seven things happen here: the hypertableau is asked twice and must answer identically; its
/// verdict is compared against the concept-tree tableau's, which must AGREE; BOTH cores' shape
/// counters are held to [`counters_are_coherent`]; it is compared
/// against ITSELF over the all-meta encoding of the same terminology, which must also agree;
/// a case neither core could finish is skipped; where the oracle exhibits a model the
/// hypertableau's `consistent` is asserted unconditionally; and where the oracle finds NO
/// model and
/// [`forces_unnamed_element`] says the bound was sufficient, `consistent` is asserted to be
/// false.
fn check(sig: Signature, axioms: &[Axiom], tally: &RefCell<Tally>) -> Result<(), TestCaseError> {
    let case = Case::assemble(sig, axioms);
    let cap = graph::step_cap(&case.kb).min(STEP_CAP);
    let first = hyper::decide(&case.kb, &Assumptions::of_kb(), cap);
    let again = hyper::decide(&case.kb, &Assumptions::of_kb(), cap);
    // The WHOLE decision, not three of its fields: a field added to `Decision` that varied run
    // to run would slip past a tuple comparison naming the fields that existed when it was
    // written.
    tally.borrow_mut().spent(&first);
    if first != again {
        return Err(TestCaseError::fail(format!(
            "the hypertableau decided the same knowledge base two different ways:\n\
             {first:?}\nthen\n{again:?}\naxioms:\n{}",
            case.axioms_text()
        )));
    }
    // THE DIFFERENTIAL. The concept-tree tableau decides the same fragment by a different
    // rule set, so where both finish their verdicts must be the same verdict. A divergence is
    // a soundness or completeness bug in one of the two — never a recorded difference.
    let reference = tableau::decide(&case.kb, &Assumptions::of_kb(), cap);
    // The shape counters of BOTH cores, held to the invariants that make them meaningful.
    // The two calculi are expected to cost different amounts — that is what makes them a
    // differential rather than a copy — so what is checked is not that their numbers agree
    // but that each core's own three are consistent with the search it ran.
    for (core, decision) in [
        ("hypertableau", &first),
        ("concept-tree tableau", &reference),
    ] {
        counters_are_coherent(core, decision, &case)?;
    }
    if !first.exhausted && !reference.exhausted {
        if first.consistent != reference.consistent {
            return Err(TestCaseError::fail(format!(
                "the hypertableau and the concept-tree tableau disagree: hypertableau says \
                 {}, concept-tree tableau says {}\naxioms:\n{}\n{}",
                first.consistent,
                reference.consistent,
                case.axioms_text(),
                case.smallest_model().map_or_else(
                    || "the oracle found no model up to the signature's bound".to_owned(),
                    |model| format!("oracle model:\n{}", case.model_text(&model))
                )
            )));
        }
        tally.borrow_mut().differential += 1;
    }
    // THE ENCODING DIFFERENTIAL. The same knowledge base, the same calculus, the OTHER
    // encoding of its terminology: every inclusion internalized, nothing absorbed. Absorption
    // rests on a semantic claim — that a guarded clause fires exactly where the antecedent
    // holds in the model read off a completion graph — and a claim of that shape cannot be
    // checked by a rule set agreeing with itself. A guard that matched too FEW nodes would
    // leave an axiom unenforced and show up here as a knowledge base the absorbed encoding
    // calls consistent and the internalized one refutes.
    let internalized = Case::encoded(sig, axioms, true);
    let other = hyper::decide(&internalized.kb, &Assumptions::of_kb(), cap);
    if !first.exhausted && !other.exhausted {
        if first.consistent != other.consistent {
            return Err(TestCaseError::fail(format!(
                "the two TBox encodings disagree: absorbed says {}, all-meta says {}\n\
                 absorbed clauses: {:?}\nmeta: {:?}\naxioms:\n{}",
                first.consistent,
                other.consistent,
                case.kb.absorbed,
                case.kb.meta,
                case.axioms_text(),
            )));
        }
        tally.borrow_mut().encodings += 1;
    }
    if first.exhausted {
        tally.borrow_mut().exhausted += 1;
        return Ok(());
    }
    match case.smallest_model() {
        Some(model) => {
            if !first.consistent {
                return Err(TestCaseError::fail(format!(
                    "the hypertableau rejected a knowledge base the oracle exhibits a model of\n\
                     axioms:\n{}\nmodel:\n{}",
                    case.axioms_text(),
                    case.model_text(&model)
                )));
            }
            tally.borrow_mut().modelled += 1;
        }
        None if first.consistent => {
            // The oracle found no model up to its bound. For a knowledge base that can
            // force an element beyond the named individuals, that is silent — `≥3 r.⊤` is
            // consistent and has no model over two elements — and the case is only
            // counted (the concept-tree tableau above is what checks it instead). But when NOTHING in the axiom set can force such an element, a
            // model, if one exists, restricts to the individuals' own equivalence classes:
            // removing elements can only make `∀`, `≤n` and `¬` easier, and there is no
            // `∃`/`≥n` left to break. So provided the enumeration is wide enough to give
            // every individual its own element, "no model up to the bound" IS "no model",
            // and a consistent verdict is an UNSOUNDNESS — the direction that asserts
            // something false rather than withholding something true.
            if bounded_domain(sig, axioms) {
                tally.borrow_mut().bound_asserted += 1;
                return Err(TestCaseError::fail(format!(
                    "the hypertableau accepted a knowledge base with NO model, and nothing in \
                     it can force an element beyond the {} named individuals, so every \
                     interpretation up to {} elements was checked and none is a model \
                     — this is an unsoundness\naxioms:\n{}",
                    sig.individuals,
                    sig.max_domain,
                    case.axioms_text()
                )));
            }
            tally.borrow_mut().unbounded += 1;
        }
        None => {
            let mut t = tally.borrow_mut();
            t.refuted += 1;
            if bounded_domain(sig, axioms) {
                t.bound_asserted += 1;
            }
        }
    }
    Ok(())
}

/// A fixed 32-byte ChaCha seed, distinguished by `tag` so two properties do not walk the
/// same sequence of knowledge bases. Fixed is the whole point: the suite generates the same
/// corpus on every run, so a failure reproduces and a pass means something stable.
fn seed(tag: u8) -> [u8; 32] {
    let mut bytes = [0x5a; 32];
    bytes[0] = tag;
    bytes
}

/// Run one property: `cases` generated knowledge bases over `sig`, each put through
/// [`check`], and then a health check on the tally so the property cannot pass by asserting
/// nothing.
#[allow(clippy::too_many_arguments)]
fn run_property(
    name: &str,
    sig: Signature,
    cases: u32,
    tag: u8,
    bound_floor: u32,
    step_ceiling: u64,
    strategy: &BoxedStrategy<Vec<Axiom>>,
) {
    let config = Config {
        cases,
        // No on-disk regression files: the fixed seed already makes every run identical.
        failure_persistence: None,
        ..Config::default()
    };
    let mut runner =
        TestRunner::new_with_rng(config, TestRng::from_seed(RngAlgorithm::ChaCha, &seed(tag)));
    let tally = RefCell::new(Tally::default());
    if let Err(failure) = runner.run(strategy, |axioms| check(sig, &axioms, &tally)) {
        panic!("{name} over {sig:?}: {failure}");
    }
    let tally = tally.into_inner();
    println!("{name}: {cases} knowledge bases over {sig:?} → {tally:?}");
    assert!(
        tally.total() >= cases,
        "{name} ran {} cases, not the {cases} it configured: {tally:?}",
        tally.total()
    );
    assert!(
        tally.exhausted * 20 <= cases,
        "{name} skipped more than 5% of its cases on the step cap, so it is no longer \
         checking the decision core: {tally:?}"
    );
    // THE COST CEILING. The corpus is generated from a fixed seed and decided by a
    // deterministic search, so the round total below is a reproducible number rather than a
    // timing, and pinning a ceiling on it is what makes a search regression fail a test
    // instead of slowly making the suite slower. It is a CEILING with stated headroom rather
    // than an equality: the question is "did the search's cost change materially", and an
    // equality would turn every generator tweak into a mechanical re-pin of seven constants.
    // Each caller's literal is the measured total plus roughly a tenth, and the comment at
    // the call site states the measurement.
    assert!(
        tally.steps <= step_ceiling,
        "{name} spent {} derivation rounds over its {cases} cases, above its ceiling of \
         {step_ceiling}. The search is doing materially more work than when this ceiling was \
         measured: {tally:?}",
        tally.steps
    );
    // The DIFFERENTIAL population. Both cores decide almost every generated case inside the
    // narrowed cap, so a share that collapses means the two are no longer being compared —
    // which is the one way a zero-divergence claim can pass by asserting nothing.
    assert!(
        tally.differential * 20 >= cases * 19,
        "{name} compared the two decision cores on fewer than 95% of its cases, so the \
         zero-divergence claim rests on almost nothing: {tally:?}"
    );
    // The ENCODING population, floored on the same argument: the absorbed clause table and
    // the internalized disjunction are two spellings of one terminology, and a comparison
    // that ran on almost nothing would let a guard that fires too narrowly pass.
    assert!(
        tally.encodings * 20 >= cases * 19,
        "{name} compared the two TBox encodings on fewer than 95% of its cases, so the \
         absorption claim rests on almost nothing: {tally:?}"
    );
    assert!(
        tally.modelled * 4 >= cases,
        "{name} decided fewer than a quarter of its cases by an exhibited model, so the \
         unconditional direction is barely being asserted: {tally:?}"
    );
    // The OVER-PERMISSIVE direction, floored per property. An assertion that never fires
    // reads exactly like one that passes, so each property states how often its bound was
    // sufficient.
    //
    // A FLOOR rather than an equality, deliberately. The seed is fixed, so these counts are
    // deterministic and an equality would compile — but the question being asked is "does this
    // direction bind often enough to be checking something", not "is this number exactly N". A
    // floor set ~20% below the observed count answers the first: a generator or tableau change
    // that materially narrows the checked population fails, while an improvement that widens
    // it does not generate churn on four constants. The two ZERO cases are equalities, because
    // there the question really is "is this still structurally impossible". Two properties state ZERO, and that is asserted as an equality rather than
    // waved past: their signatures name more individuals than their enumeration has elements,
    // so `bounded_domain` can never hold for them, and widening either signature must force
    // this number to be revisited rather than silently starting to mean something.
    if bound_floor == 0 {
        assert_eq!(
            tally.bound_asserted, 0,
            "{name} now asserts the over-permissive direction {} time(s) where its signature \
             made that impossible — its individuals used to outnumber its domain bound. Give \
             it a real floor: {tally:?}",
            tally.bound_asserted
        );
    } else {
        assert!(
            tally.bound_asserted >= bound_floor,
            "{name} asserted the over-permissive direction only {} time(s), below its floor \
             of {bound_floor}. A generator change has narrowed what this property checks: \
             {tally:?}",
            tally.bound_asserted
        );
    }
}

// ── The generators ──────────────────────────────────────────────────────────────

/// A strategy over the signature's class names, as atomic concepts.
fn arb_named(sig: Signature) -> BoxedStrategy<Concept> {
    prop::sample::select(sig.concept_names().to_vec())
        .prop_map(Concept::Named)
        .boxed()
}

/// A strategy over nominals `{a₁ … aₙ}` with up to `members` members, canonicalized by
/// [`Concept::nominal`].
fn arb_nominal(sig: Signature, members: usize) -> BoxedStrategy<Concept> {
    let names = sig.individual_names().to_vec();
    prop::collection::vec(
        prop::sample::select(names),
        1..=members.min(sig.individuals),
    )
    .prop_map(Concept::nominal)
    .boxed()
}

/// A strategy over roles, named and inverse in equal measure.
fn arb_role(sig: Signature) -> BoxedStrategy<Role> {
    let names = sig.role_names().to_vec();
    (prop::sample::select(names), any::<bool>())
        .prop_map(|(p, inverse)| {
            if inverse {
                Role::Inv(p)
            } else {
                Role::Named(p)
            }
        })
        .boxed()
}

/// A strategy over roles biased towards the INVERSE direction, for the properties that are
/// about what inverses do.
fn arb_inverse_role(sig: Signature) -> BoxedStrategy<Role> {
    let names = sig.role_names().to_vec();
    Union::new_weighted(vec![
        (
            3,
            prop::sample::select(names.clone())
                .prop_map(Role::Inv)
                .boxed(),
        ),
        (1, prop::sample::select(names).prop_map(Role::Named).boxed()),
    ])
    .boxed()
}

/// A strategy over the atomic concepts of the signature: `⊤`, `⊥`, a class name, a nominal,
/// and `∃r.Self`.
fn arb_leaf(sig: Signature) -> BoxedStrategy<Concept> {
    let mut options: Vec<(u32, BoxedStrategy<Concept>)> = vec![
        (1, Just(Concept::Top).boxed()),
        (1, Just(Concept::Bottom).boxed()),
    ];
    if sig.concepts > 0 {
        options.push((8, arb_named(sig)));
    }
    if sig.individuals > 0 {
        options.push((4, arb_nominal(sig, 2)));
    }
    if sig.roles > 0 {
        options.push((2, arb_role(sig).prop_map(Concept::SelfRestriction).boxed()));
    }
    Union::new_weighted(options).boxed()
}

/// A strategy over concepts of at most `depth` nested constructors, covering every
/// [`Concept`] variant the signature admits.
fn arb_concept(sig: Signature, depth: u32) -> BoxedStrategy<Concept> {
    if depth == 0 {
        return arb_leaf(sig);
    }
    let inner = arb_concept(sig, depth - 1);
    let mut options: Vec<(u32, BoxedStrategy<Concept>)> = vec![
        (8, arb_leaf(sig)),
        (
            3,
            inner
                .clone()
                .prop_map(|c| Concept::Not(Box::new(c)))
                .boxed(),
        ),
        (
            4,
            prop::collection::vec(inner.clone(), 1..=2)
                .prop_map(Concept::And)
                .boxed(),
        ),
        (
            4,
            prop::collection::vec(inner.clone(), 1..=2)
                .prop_map(Concept::Or)
                .boxed(),
        ),
    ];
    if sig.roles > 0 {
        options.push((
            4,
            (arb_role(sig), inner.clone())
                .prop_map(|(r, c)| Concept::Some(r, Box::new(c)))
                .boxed(),
        ));
        options.push((
            4,
            (arb_role(sig), inner.clone())
                .prop_map(|(r, c)| Concept::All(r, Box::new(c)))
                .boxed(),
        ));
        options.push((
            3,
            (0u32..=3, arb_role(sig), inner.clone())
                .prop_map(|(n, r, c)| Concept::Min(n, r, Box::new(c)))
                .boxed(),
        ));
        options.push((
            3,
            (0u32..=3, arb_role(sig), inner)
                .prop_map(|(n, r, c)| Concept::Max(n, r, Box::new(c)))
                .boxed(),
        ));
    }
    Union::new_weighted(options).boxed()
}

/// A strategy over every axiom kind the signature admits.
fn arb_axiom(sig: Signature) -> BoxedStrategy<Axiom> {
    let individuals = sig.individual_names().to_vec();
    let mut options: Vec<(u32, BoxedStrategy<Axiom>)> = vec![
        (
            6,
            (arb_concept(sig, 2), arb_concept(sig, 2))
                .prop_map(|(sub, sup)| Axiom::Gci(sub, sup))
                .boxed(),
        ),
        (
            6,
            (
                prop::sample::select(individuals.clone()),
                arb_concept(sig, 2),
            )
                .prop_map(|(a, c)| Axiom::Type(a, c))
                .boxed(),
        ),
        (
            2,
            (
                prop::sample::select(individuals.clone()),
                prop::sample::select(individuals.clone()),
            )
                .prop_map(|(a, b)| Axiom::SameAs(a, b))
                .boxed(),
        ),
        (
            3,
            (
                prop::sample::select(individuals.clone()),
                prop::sample::select(individuals.clone()),
            )
                .prop_map(|(a, b)| Axiom::DifferentFrom(a, b))
                .boxed(),
        ),
    ];
    if sig.roles > 0 {
        let roles = sig.role_names().to_vec();
        options.push((
            4,
            (
                prop::sample::select(individuals.clone()),
                prop::sample::select(roles.clone()),
                prop::sample::select(individuals),
            )
                .prop_map(|(a, p, b)| Axiom::RoleAssertion(a, p, b))
                .boxed(),
        ));
        options.push((
            2,
            (
                prop::sample::select(roles.clone()),
                prop::sample::select(roles.clone()),
            )
                .prop_map(|(sub, sup)| Axiom::SubRole(sub, sup))
                .boxed(),
        ));
        options.push((
            2,
            (
                prop::sample::select(roles.clone()),
                prop::sample::select(roles.clone()),
            )
                .prop_map(|(r, s)| Axiom::InverseOf(r, s))
                .boxed(),
        ));
        options.push((
            1,
            prop::sample::select(roles.clone())
                .prop_map(Axiom::Transitive)
                .boxed(),
        ));
        options.push((
            1,
            prop::sample::select(roles.clone())
                .prop_map(Axiom::Asymmetric)
                .boxed(),
        ));
        options.push((
            1,
            (
                prop::sample::select(roles.clone()),
                prop::sample::select(roles),
            )
                .prop_map(|(r, s)| Axiom::DisjointRoles(r, s))
                .boxed(),
        ));
    }
    Union::new_weighted(options).boxed()
}

/// One to six axioms drawn from `axiom`.
fn arb_axioms(axiom: BoxedStrategy<Axiom>) -> BoxedStrategy<Vec<Axiom>> {
    prop::collection::vec(axiom, 1..=6).boxed()
}

/// One to three axiom GROUPS drawn from `group`, flattened.
///
/// The unit of the two families below is an EQUIVALENCE, which [`Kb`] holds as two inclusions
/// that have to travel together: generating them independently would produce one half of an
/// equivalence far more often than both, and the whole point of those families is what the
/// CONVERSE direction does to the clause set. Three groups of up to four inclusions is the
/// same order of axiom count as [`arb_axioms`]' one to six.
fn arb_axiom_groups(group: BoxedStrategy<Vec<Axiom>>) -> BoxedStrategy<Vec<Axiom>> {
    prop::collection::vec(group, 1..=3)
        .prop_map(|groups| groups.concat())
        .boxed()
}

/// `left ≡ right`, as the two inclusions [`Kb`] holds an equivalence as.
fn equivalence(left: &Concept, right: &Concept) -> Vec<Axiom> {
    vec![
        Axiom::Gci(left.clone(), right.clone()),
        Axiom::Gci(right.clone(), left.clone()),
    ]
}

// ── The general properties ──────────────────────────────────────────────────────

/// Knowledge bases checked by the widest-signature property.
const WIDE_CASES: u32 = 400;

/// Knowledge bases checked by the deepest-domain property.
const DEEP_CASES: u32 = 300;

/// Every axiom kind, every concept constructor, three classes, two roles, three
/// individuals, models up to two elements.
#[test]
fn a_random_knowledge_base_is_consistent_whenever_the_oracle_exhibits_a_model() {
    run_property(
        "wide",
        WIDE,
        WIDE_CASES,
        1,
        0,
        // Measured 2,090 rounds, of which 500 are the one case that exhausts at any cap.
        2_300,
        &arb_axioms(arb_axiom(WIDE)),
    );
}

/// The same property one domain element deeper: with a single role name a third element is
/// affordable, so a tableau `false` is matched by the oracle failing at sizes 1, 2 AND 3.
#[test]
fn a_random_knowledge_base_agrees_with_the_oracle_over_a_three_element_domain() {
    run_property(
        "deep",
        DEEP,
        DEEP_CASES,
        2,
        20,
        // Measured 1,179 rounds.
        1_300,
        &arb_axioms(arb_axiom(DEEP)),
    );
}

// ── The four interaction properties ─────────────────────────────────────────────

/// Knowledge bases checked by the nominal/inverse/cardinality property.
const NOMINAL_INVERSE_CASES: u32 = 400;

/// Knowledge bases checked by the `owl:oneOf`/`owl:differentFrom` property.
const ONE_OF_CASES: u32 = 600;

/// Knowledge bases checked by the cardinality/role-hierarchy property.
const ROLE_HIERARCHY_CASES: u32 = 1500;

/// Knowledge bases checked by the complement/disjunction property.
const BOOLEAN_CASES: u32 = 2500;

/// `≤n r⁻.{a}` / `≥n r⁻.C` — a cardinality restriction over a mostly-inverse role with a
/// mostly-nominal filler, the shape where the counting rules, the inverse-role closure and
/// the identification the `o`-rule performs all bear on the same node.
fn arb_inverse_cardinality(sig: Signature) -> BoxedStrategy<Concept> {
    let filler = Union::new_weighted(vec![
        (4, arb_nominal(sig, 2)),
        (2, arb_named(sig)),
        (1, Just(Concept::Top).boxed()),
    ])
    .boxed();
    (0u32..=2, arb_inverse_role(sig), filler, any::<bool>())
        .prop_map(|(n, role, filler, bounded)| {
            if bounded {
                Concept::Max(n, role, Box::new(filler))
            } else {
                Concept::Min(n + 1, role, Box::new(filler))
            }
        })
        .boxed()
}

/// Nominals, inverse roles and cardinality in the same knowledge base.
#[test]
fn nominals_under_inverse_roles_and_cardinality_agree_with_the_oracle() {
    let sig = NOMINAL_INVERSE;
    let individuals = sig.individual_names().to_vec();
    let roles = sig.role_names().to_vec();
    let axiom = Union::new_weighted(vec![
        (
            6,
            (
                prop::sample::select(individuals.clone()),
                arb_inverse_cardinality(sig),
            )
                .prop_map(|(a, c)| Axiom::Type(a, c))
                .boxed(),
        ),
        (
            4,
            (arb_named(sig), arb_inverse_cardinality(sig))
                .prop_map(|(sub, sup)| Axiom::Gci(sub, sup))
                .boxed(),
        ),
        (
            3,
            (
                prop::sample::select(individuals.clone()),
                prop::sample::select(roles.clone()),
                prop::sample::select(individuals.clone()),
            )
                .prop_map(|(a, p, b)| Axiom::RoleAssertion(a, p, b))
                .boxed(),
        ),
        (
            3,
            (
                prop::sample::select(individuals.clone()),
                prop::sample::select(individuals),
            )
                .prop_map(|(a, b)| Axiom::DifferentFrom(a, b))
                .boxed(),
        ),
        (
            2,
            (
                prop::sample::select(roles.clone()),
                prop::sample::select(roles),
            )
                .prop_map(|(r, s)| Axiom::InverseOf(r, s))
                .boxed(),
        ),
    ])
    .boxed();
    run_property(
        "nominal ⊗ inverse ⊗ cardinality",
        sig,
        NOMINAL_INVERSE_CASES,
        3,
        10,
        // Measured 777 rounds.
        860,
        &arb_axioms(axiom),
    );
}

/// A multi-member `owl:oneOf` against `owl:differentFrom` — the interaction where the `o`-rule
/// must identify rather than compare names, and where the identification being blocked by a
/// recorded `≠` is what makes non-membership a clash.
#[test]
fn multi_member_nominals_against_distinctness_agree_with_the_oracle() {
    let sig = ONE_OF;
    let individuals = sig.individual_names().to_vec();
    let enumeration = arb_nominal(sig, 3);
    let axiom = Union::new_weighted(vec![
        (
            6,
            (
                prop::sample::select(individuals.clone()),
                enumeration.clone(),
            )
                .prop_map(|(a, c)| Axiom::Type(a, c))
                .boxed(),
        ),
        (
            4,
            (
                prop::sample::select(individuals.clone()),
                enumeration.clone(),
            )
                .prop_map(|(a, c)| Axiom::Type(a, Concept::Not(Box::new(c))))
                .boxed(),
        ),
        (
            5,
            (
                prop::sample::select(individuals.clone()),
                prop::sample::select(individuals.clone()),
            )
                .prop_map(|(a, b)| Axiom::DifferentFrom(a, b))
                .boxed(),
        ),
        (
            2,
            (
                prop::sample::select(individuals.clone()),
                prop::sample::select(individuals.clone()),
            )
                .prop_map(|(a, b)| Axiom::SameAs(a, b))
                .boxed(),
        ),
        (
            3,
            (arb_named(sig), enumeration)
                .prop_map(|(sub, sup)| Axiom::Gci(sub, sup))
                .boxed(),
        ),
        (
            2,
            (
                prop::sample::select(individuals.clone()),
                prop::sample::select(sig.role_names().to_vec()),
                prop::sample::select(individuals),
            )
                .prop_map(|(a, p, b)| Axiom::RoleAssertion(a, p, b))
                .boxed(),
        ),
    ])
    .boxed();
    run_property(
        "owl:oneOf ⊗ owl:differentFrom",
        sig,
        ONE_OF_CASES,
        4,
        250,
        // Measured 699 rounds.
        770,
        &arb_axioms(axiom),
    );
}

/// Qualified cardinality against a role hierarchy: `≥n s.C` where `r ⊑ s`, and `≤m r.C` on
/// the same node, so the counting rules have to read the role hierarchy's closure and not
/// the role's spelling.
#[test]
fn qualified_cardinality_under_a_role_hierarchy_agrees_with_the_oracle() {
    let sig = ROLE_HIERARCHY;
    let individuals = sig.individual_names().to_vec();
    let roles = sig.role_names().to_vec();
    let filler = Union::new_weighted(vec![
        (3, arb_named(sig)),
        (2, Just(Concept::Top).boxed()),
        (1, arb_nominal(sig, 1)),
    ])
    .boxed();
    let counted = (
        0u32..=2,
        prop::sample::select(roles.clone()),
        filler,
        any::<bool>(),
    )
        .prop_map(|(n, p, filler, bounded)| {
            if bounded {
                Concept::Max(n, Role::Named(p), Box::new(filler))
            } else {
                Concept::Min(n + 1, Role::Named(p), Box::new(filler))
            }
        })
        .boxed();
    let axiom = Union::new_weighted(vec![
        (
            6,
            (prop::sample::select(individuals.clone()), counted.clone())
                .prop_map(|(a, c)| Axiom::Type(a, c))
                .boxed(),
        ),
        (
            4,
            (arb_named(sig), counted)
                .prop_map(|(sub, sup)| Axiom::Gci(sub, sup))
                .boxed(),
        ),
        (
            6,
            (
                prop::sample::select(roles.clone()),
                prop::sample::select(roles),
            )
                .prop_map(|(sub, sup)| Axiom::SubRole(sub, sup))
                .boxed(),
        ),
        (
            3,
            (
                prop::sample::select(individuals.clone()),
                prop::sample::select(sig.role_names().to_vec()),
                prop::sample::select(individuals.clone()),
            )
                .prop_map(|(a, p, b)| Axiom::RoleAssertion(a, p, b))
                .boxed(),
        ),
        (
            3,
            (
                prop::sample::select(individuals.clone()),
                prop::sample::select(individuals),
            )
                .prop_map(|(a, b)| Axiom::DifferentFrom(a, b))
                .boxed(),
        ),
    ])
    .boxed();
    run_property(
        "cardinality ⊗ role hierarchy",
        sig,
        ROLE_HIERARCHY_CASES,
        5,
        0,
        // Measured 2,273 rounds.
        2_500,
        &arb_axioms(axiom),
    );
}

/// Complement against disjunction: `¬(C ⊔ D)` and `(C ⊓ ¬C) ⊔ D` alongside freely nested
/// boolean concepts. With no role in the signature every concept is boolean, which is what
/// makes a three-element domain and three class names affordable together.
#[test]
fn complement_against_disjunction_agrees_with_the_oracle() {
    let sig = BOOLEAN;
    let individuals = sig.individual_names().to_vec();
    let boolean = Union::new_weighted(vec![
        (4, arb_concept(sig, 3)),
        (
            3,
            (arb_named(sig), arb_named(sig))
                .prop_map(|(c, d)| Concept::Not(Box::new(Concept::Or(vec![c, d]))))
                .boxed(),
        ),
        (
            3,
            (arb_named(sig), arb_named(sig))
                .prop_map(|(c, d)| {
                    Concept::Or(vec![
                        Concept::And(vec![c.clone(), Concept::Not(Box::new(c))]),
                        d,
                    ])
                })
                .boxed(),
        ),
    ])
    .boxed();
    let axiom = Union::new_weighted(vec![
        (
            6,
            (prop::sample::select(individuals.clone()), boolean.clone())
                .prop_map(|(a, c)| Axiom::Type(a, c))
                .boxed(),
        ),
        (
            6,
            (boolean.clone(), boolean)
                .prop_map(|(sub, sup)| Axiom::Gci(sub, sup))
                .boxed(),
        ),
        (
            2,
            (
                prop::sample::select(individuals.clone()),
                prop::sample::select(individuals.clone()),
            )
                .prop_map(|(a, b)| Axiom::DifferentFrom(a, b))
                .boxed(),
        ),
        (
            1,
            (
                prop::sample::select(individuals.clone()),
                prop::sample::select(individuals),
            )
                .prop_map(|(a, b)| Axiom::SameAs(a, b))
                .boxed(),
        ),
    ])
    .boxed();
    run_property(
        "complement ⊗ disjunction",
        sig,
        BOOLEAN_CASES,
        6,
        700,
        // Measured 9,248 rounds — the most expensive property, and the one holding the
        // 439-round case STEP_CAP is sized for.
        10_200,
        &arb_axioms(axiom),
    );
}

// ── The absorption property ─────────────────────────────────────────────────────

/// Knowledge bases checked by the absorption property.
const ABSORPTION_CASES: u32 = 2000;

/// Every ABSORBABLE inclusion shape, in the fragment where "no bounded model" IS "no model".
///
/// The other properties reach absorption incidentally and mostly through antecedents that
/// quantify, which is exactly the fragment `bounded_domain` disqualifies — so their
/// over-permissive direction is silent about the shapes this pass actually rewrites. This one
/// is quantifier-free by construction: every axiom it generates is a boolean combination or a
/// nominal, `forces_unnamed_element` is false of all of them, and a `consistent` verdict the
/// oracle cannot exhibit a model for is therefore an UNSOUNDNESS that fails the run rather
/// than a case that gets counted.
///
/// The shapes, and what each is here to exercise:
///
/// * `A ≡ B ⊓ C` — one direction absorbs to a conjunctive GUARD (`B(x) ∧ C(x) → A(x)`), the
///   other splits its conjunctive consequent into two one-atom clauses;
/// * `A ≡ B ⊔ C` — one direction splits its disjunctive antecedent into two clauses, the
///   other keeps a `⊔` consequent that still branches;
/// * `A ⊓ B ⊑ ⊥` — disjointness, whose guard is the pair and whose head is the clash;
/// * `⊤ ⊑ D` — the unguarded clause, which is what a range-less domain-less global axiom is;
/// * `{a} ⊑ D` — the nominal guard, matched against the node's identity and never its label;
/// * `A ⊑ ¬B` — a NEGATED consequent under a guard, beside the negative ANTECEDENT `¬A ⊑ B`
///   that absorption must refuse and internalize.
#[test]
fn the_absorbable_inclusion_shapes_agree_with_the_oracle() {
    let sig = ABSORPTION;
    let individuals = sig.individual_names().to_vec();
    let named = arb_named(sig);
    let axiom = Union::new_weighted(vec![
        (
            5,
            (named.clone(), named.clone(), named.clone())
                .prop_map(|(a, b, c)| Axiom::Gci(a, Concept::And(vec![b, c])))
                .boxed(),
        ),
        (
            5,
            (named.clone(), named.clone(), named.clone())
                .prop_map(|(a, b, c)| Axiom::Gci(Concept::And(vec![b, c]), a))
                .boxed(),
        ),
        (
            4,
            (named.clone(), named.clone(), named.clone())
                .prop_map(|(a, b, c)| Axiom::Gci(Concept::Or(vec![b, c]), a))
                .boxed(),
        ),
        (
            4,
            (named.clone(), named.clone(), named.clone())
                .prop_map(|(a, b, c)| Axiom::Gci(a, Concept::Or(vec![b, c])))
                .boxed(),
        ),
        (
            4,
            (named.clone(), named.clone())
                .prop_map(|(a, b)| Axiom::Gci(Concept::And(vec![a, b]), Concept::Bottom))
                .boxed(),
        ),
        (
            4,
            (named.clone(), named.clone())
                .prop_map(|(a, b)| Axiom::Gci(a, Concept::Not(Box::new(b))))
                .boxed(),
        ),
        (
            3,
            (named.clone(), named.clone())
                .prop_map(|(a, b)| Axiom::Gci(Concept::Not(Box::new(a)), b))
                .boxed(),
        ),
        (
            3,
            named
                .clone()
                .prop_map(|a| Axiom::Gci(Concept::Top, a))
                .boxed(),
        ),
        (
            3,
            (arb_nominal(sig, 2), named.clone())
                .prop_map(|(members, a)| Axiom::Gci(members, a))
                .boxed(),
        ),
        (
            6,
            (prop::sample::select(individuals.clone()), named)
                .prop_map(|(a, c)| Axiom::Type(a, c))
                .boxed(),
        ),
        (
            4,
            (
                prop::sample::select(individuals.clone()),
                arb_nominal(sig, 2),
            )
                .prop_map(|(a, c)| Axiom::Type(a, c))
                .boxed(),
        ),
        (
            4,
            (
                prop::sample::select(individuals.clone()),
                prop::sample::select(individuals.clone()),
            )
                .prop_map(|(a, b)| Axiom::DifferentFrom(a, b))
                .boxed(),
        ),
        (
            2,
            (
                prop::sample::select(individuals.clone()),
                prop::sample::select(individuals),
            )
                .prop_map(|(a, b)| Axiom::SameAs(a, b))
                .boxed(),
        ),
    ])
    .boxed();
    run_property(
        "absorption ⊗ boolean",
        sig,
        ABSORPTION_CASES,
        7,
        300,
        // Measured 3,923 rounds.
        4_320,
        &arb_axioms(axiom),
    );
}

// ── The reported-shape family ───────────────────────────────────────────────────

/// Knowledge bases checked by the reported-shape property.
const REPORTED_CASES: u32 = 600;

/// The interaction an ordinary ontology reached the search's step cap on: a `∀`-restriction
/// under an EQUIVALENCE, an exact cardinality under another, and an inverse role.
///
/// The other families reach equivalences incidentally and mostly over named classes, where
/// both directions absorb into guarded clauses and neither branches. This one generates the
/// converse direction on purpose, because that is where the cost lives: `A ≡ ∀r.(S ⊓ ∀p.D)` is
/// two inclusions, and the second — `∀r.(S ⊓ ∀p.D) ⊑ A` — has an antecedent no faithful
/// absorption can guard, so it reaches the search as a disjunction that every node must
/// resolve. An exact cardinality does the same on the counting side: `A ≡ =n c.⊤` puts
/// `≥n c.⊤ ⊓ ≤n c.⊤ ⊑ A` in front of the search, whose antecedent is a conjunction of two
/// counting concepts.
///
/// The shapes, and what each is here for:
///
/// * `A ≡ ∀r.(S ⊓ ∀p.D)` — the non-absorbable `∀`-GCI, over a mostly-INVERSE outer role, so
///   the obligation the converse direction derives flows back up an edge;
/// * `A ≡ ≥n c.⊤ ⊓ ≤n c.⊤` — an exact cardinality equivalence, whose converse antecedent is
///   two counting concepts at once;
/// * `r owl:inverseOf s` — the axiom that makes the `∀r⁻` obligations above reach a
///   predecessor rather than dead-end;
/// * `⊤ ⊑ ∀p.S` — an `rdfs:range`, the unguarded inclusion the reported ontology carried;
/// * plain inclusions, type assertions and `owl:differentFrom`, so the family is a knowledge
///   base rather than a terminology.
#[test]
fn the_reported_equivalence_shape_agrees_with_the_oracle() {
    let sig = REPORTED;
    let individuals = sig.individual_names().to_vec();
    let roles = sig.role_names().to_vec();
    let named = arb_named(sig);
    let group = Union::new_weighted(vec![
        (
            6,
            (
                named.clone(),
                arb_inverse_role(sig),
                named.clone(),
                arb_role(sig),
                named.clone(),
            )
                .prop_map(|(a, outer, conjunct, inner, filler)| {
                    equivalence(
                        &a,
                        &Concept::All(
                            outer,
                            Box::new(Concept::And(vec![
                                conjunct,
                                Concept::All(inner, Box::new(filler)),
                            ])),
                        ),
                    )
                })
                .boxed(),
        ),
        (
            5,
            (named.clone(), 0u32..=2, arb_role(sig))
                .prop_map(|(a, n, counted)| {
                    equivalence(
                        &a,
                        &Concept::And(vec![
                            Concept::Min(n, counted, Box::new(Concept::Top)),
                            Concept::Max(n, counted, Box::new(Concept::Top)),
                        ]),
                    )
                })
                .boxed(),
        ),
        (
            3,
            (
                prop::sample::select(roles.clone()),
                prop::sample::select(roles),
            )
                .prop_map(|(r, s)| vec![Axiom::InverseOf(r, s)])
                .boxed(),
        ),
        (
            3,
            (arb_role(sig), named.clone())
                .prop_map(|(p, s)| vec![Axiom::Gci(Concept::Top, Concept::All(p, Box::new(s)))])
                .boxed(),
        ),
        (
            5,
            (prop::sample::select(individuals.clone()), named.clone())
                .prop_map(|(a, c)| vec![Axiom::Type(a, c)])
                .boxed(),
        ),
        (
            3,
            (named.clone(), named)
                .prop_map(|(a, b)| vec![Axiom::Gci(a, b)])
                .boxed(),
        ),
        (
            2,
            (
                prop::sample::select(individuals.clone()),
                prop::sample::select(individuals),
            )
                .prop_map(|(a, b)| vec![Axiom::DifferentFrom(a, b)])
                .boxed(),
        ),
    ])
    .boxed();
    run_property(
        "∀-equivalence ⊗ exact cardinality ⊗ inverse",
        sig,
        REPORTED_CASES,
        8,
        12,
        // Measured 2,724 rounds over 973 case splits — the second most branch-heavy
        // property in the suite, which is the point of it.
        3_000,
        &arb_axiom_groups(group),
    );
}

// ── The cycle family ────────────────────────────────────────────────────────────

/// Knowledge bases checked by the cyclic-equivalence property.
const CYCLE_CASES: u32 = 600;

/// CYCLIC equivalences: `A ≡ ∃r.A`, and the two-cycle `A ≡ ∃r.B` with `B ≡ ∃r⁻.A`.
///
/// A cyclic definition is the shape whose completion graph is INFINITE and whose termination
/// is therefore blocking's alone: `A ≡ ∃r.A` forces every `A`-node to mint an `A`-successor
/// forever, and nothing in the clause set stops it — what stops it is that the second such
/// node has the first's blocking signature. The two-cycle is the same argument one step
/// harder, because the role alternates direction and so the signature has to agree on the
/// INCOMING edge as well as on the two labels.
///
/// The converse halves are what make these equivalences rather than inclusions, and they are
/// the expensive ones: `∃r.A ⊑ A` re-roots at the filler and lands its head on the matched
/// node's PREDECESSOR, which is the clause form whose interaction with blocking the calculus's
/// model construction has to make good. A family that generated only `A ⊑ ∃r.A` would exercise
/// the minting and never that.
///
/// The bounded oracle is mostly silent here — a cycle forces elements beyond the named
/// individuals, so `bounded_domain` is false of nearly every case — and that is exactly why
/// the family is worth having: the concept-tree tableau is UNBOUNDED, and this is the corner
/// where it, rather than the enumeration, is the reference.
#[test]
fn cyclic_equivalences_agree_with_the_oracle() {
    let sig = CYCLE;
    let individuals = sig.individual_names().to_vec();
    let named = arb_named(sig);
    let group = Union::new_weighted(vec![
        (
            6,
            (named.clone(), arb_role(sig))
                .prop_map(|(a, r)| equivalence(&a, &Concept::Some(r, Box::new(a.clone()))))
                .boxed(),
        ),
        (
            6,
            (named.clone(), named.clone(), arb_role(sig))
                .prop_map(|(a, b, r)| {
                    let back = match r {
                        Role::Named(p) => Role::Inv(p),
                        Role::Inv(p) => Role::Named(p),
                    };
                    let mut out = equivalence(&a, &Concept::Some(r, Box::new(b.clone())));
                    out.extend(equivalence(&b, &Concept::Some(back, Box::new(a))));
                    out
                })
                .boxed(),
        ),
        (
            4,
            (named.clone(), arb_role(sig), named.clone())
                .prop_map(|(a, r, b)| vec![Axiom::Gci(a, Concept::All(r, Box::new(b)))])
                .boxed(),
        ),
        (
            4,
            (named.clone(), 0u32..=2, arb_role(sig))
                .prop_map(|(a, n, r)| {
                    vec![Axiom::Gci(a, Concept::Max(n, r, Box::new(Concept::Top)))]
                })
                .boxed(),
        ),
        (
            3,
            (named.clone(), named.clone())
                .prop_map(|(a, b)| vec![Axiom::Gci(a, Concept::Not(Box::new(b)))])
                .boxed(),
        ),
        (
            5,
            (prop::sample::select(individuals.clone()), named)
                .prop_map(|(a, c)| vec![Axiom::Type(a, c)])
                .boxed(),
        ),
        (
            2,
            (
                prop::sample::select(individuals.clone()),
                prop::sample::select(individuals),
            )
                .prop_map(|(a, b)| vec![Axiom::DifferentFrom(a, b)])
                .boxed(),
        ),
    ])
    .boxed();
    run_property(
        "cyclic equivalence",
        sig,
        CYCLE_CASES,
        9,
        11,
        // Measured 887 rounds over THREE case splits in 600 knowledge bases: a cyclic
        // equivalence absorbs on both sides, so what makes these cases hard is blocking
        // rather than branching, and the number that would move if blocking stopped
        // biting is this one.
        980,
        &arb_axiom_groups(group),
    );
}

// ── What the suite costs, pinned ────────────────────────────────────────────────

/// How many knowledge bases the whole suite decides.
const TOTAL_CASES: u32 = WIDE_CASES
    + DEEP_CASES
    + NOMINAL_INVERSE_CASES
    + ONE_OF_CASES
    + ROLE_HIERARCHY_CASES
    + BOOLEAN_CASES
    + ABSORPTION_CASES
    + REPORTED_CASES
    + CYCLE_CASES;

/// The exhaustive search is the price of an oracle nobody has to trust, so its size is
/// stated rather than discovered: each literal below is
/// `Σ_{k=1..max} 2^(k·concepts) · 2^(k²·roles) · k^individuals`, the interpretations one
/// case enumerates when it finds no model at all.
///
/// The two-role signatures stop at `k = 2` because `k = 3` over two roles is 2^18 role
/// guesses alone, and `k = 4` is 2^32 — the doubly exponential term is what fixes every
/// domain bound in this file.
#[test]
fn the_enumerated_search_spaces_are_pinned() {
    assert_eq!(WIDE.search_space(), 131_104);
    assert_eq!(DEEP.search_space(), 295_944);
    assert_eq!(NOMINAL_INVERSE.search_space(), 295_944);
    assert_eq!(ONE_OF.search_space(), 111_108);
    assert_eq!(ROLE_HIERARCHY.search_space(), 32_784);
    assert_eq!(BOOLEAN.search_space(), 14_344);
    assert_eq!(ABSORPTION.search_space(), 14_344);
    assert_eq!(REPORTED.search_space(), 65_568);
    assert_eq!(CYCLE.search_space(), 295_944);
    assert_eq!(HAND.search_space(), 65_552);
    assert_eq!(TOTAL_CASES, 8900, "generated knowledge bases per run");
}

// ── Hand-written regressions ───────────────────────────────────────────────────

/// Check one hand-written case against ALL THREE sides: the oracle must agree with the verdict
/// derived in the case's own comment, and so must both decision cores. A regression that only
/// compared two implementations could be wrong twice over.
fn assert_verdict(axioms: &[Axiom], satisfiable: bool) {
    let case = Case::assemble(HAND, axioms);
    let cap = graph::step_cap(&case.kb);
    let decision = hyper::decide(&case.kb, &Assumptions::of_kb(), cap);
    let reference = tableau::decide(&case.kb, &Assumptions::of_kb(), cap);
    assert!(
        !decision.exhausted && !reference.exhausted,
        "a hand-written regression must be decidable inside the step cap:\n{}",
        case.axioms_text()
    );
    let model = case.smallest_model();
    assert_eq!(
        model.is_some(),
        satisfiable,
        "the oracle disagrees with the derived verdict:\n{}",
        case.axioms_text()
    );
    let rendered = model.map_or_else(String::new, |m| format!("model:\n{}", case.model_text(&m)));
    assert_eq!(
        decision.consistent,
        satisfiable,
        "the hypertableau disagrees with the derived verdict:\n{}\n{rendered}",
        case.axioms_text(),
    );
    assert_eq!(
        reference.consistent,
        satisfiable,
        "the concept-tree tableau disagrees with the derived verdict:\n{}\n{rendered}",
        case.axioms_text(),
    );
}

/// The `i`-th class name of [`HAND`], as a concept.
fn class(i: usize) -> Concept {
    Concept::Named(CONCEPT_NAMES[i])
}

/// A nominal over the named individuals of [`HAND`], by index.
fn nominal(indices: &[usize]) -> Concept {
    Concept::nominal(indices.iter().map(|&i| INDIVIDUAL_NAMES[i]).collect())
}

/// The `i`-th individual name of [`HAND`].
fn individual(i: usize) -> u32 {
    INDIVIDUAL_NAMES[i]
}

/// The `i`-th role name of [`HAND`].
fn role(i: usize) -> u32 {
    ROLE_NAMES[i]
}

/// `d : {a, b, c}` with nothing saying `d` differs from any member.
///
/// SATISFIABLE. OWL 2 makes no unique name assumption, so `d` may simply BE one of the
/// three under a second name; the one-element model `Δ = {d₀}` with every name denoting
/// `d₀` witnesses it. Reporting a clash here — because `d` is not syntactically in the
/// enumeration — would be an unsoundness, not a missing feature.
#[test]
fn membership_in_an_enumeration_makes_no_unique_name_assumption() {
    assert_verdict(&[Axiom::Type(individual(3), nominal(&[0, 1, 2]))], true);
}

/// The dual: `d : {a, b, c}` together with `d ≠ a`, `d ≠ b`, `d ≠ c`.
///
/// UNSATISFIABLE. `⟦{a,b,c}⟧ = {⟦a⟧, ⟦b⟧, ⟦c⟧}` and `⟦d⟧` is in it, so `⟦d⟧` equals one of
/// the three — which every `≠` forbids. Every identification the `o`-rule could make is
/// blocked, and that is what separates a sound `o`-rule from a deleted one.
#[test]
fn membership_in_an_enumeration_clashes_when_apart_from_every_member() {
    assert_verdict(
        &[
            Axiom::Type(individual(3), nominal(&[0, 1, 2])),
            Axiom::DifferentFrom(individual(3), individual(0)),
            Axiom::DifferentFrom(individual(3), individual(1)),
            Axiom::DifferentFrom(individual(3), individual(2)),
        ],
        false,
    );
}

/// `d : ≥2 r.{a}`.
///
/// UNSATISFIABLE over every domain. `⟦{a}⟧` is the single element `⟦a⟧`, so
/// `{ y | (⟦d⟧,y) ∈ ⟦r⟧ ∧ y ∈ ⟦{a}⟧ }` has at most one member and can never reach two.
#[test]
fn two_witnesses_inside_a_singleton_nominal_is_unsatisfiable() {
    assert_verdict(
        &[Axiom::Type(
            individual(3),
            Concept::Min(2, Role::Named(role(0)), Box::new(nominal(&[0]))),
        )],
        false,
    );
}

/// `r ⊑ s` with `d : ≥2 s.⊤ ⊓ ≤1 r.⊤`.
///
/// SATISFIABLE. The two `s`-successors need not be `r`-successors — the inclusion runs the
/// other way. `Δ = {d₀, d₁}`, `⟦s⟧ = {(d₀,d₀), (d₀,d₁)}`, `⟦r⟧ = ∅`, `⟦d⟧ = d₀` is a model,
/// so a counting rule that read the role hierarchy in the wrong direction would show up here.
#[test]
fn counting_a_super_role_does_not_count_its_sub_role() {
    assert_verdict(
        &[
            Axiom::SubRole(role(0), role(1)),
            Axiom::Type(
                individual(3),
                Concept::And(vec![
                    Concept::Min(2, Role::Named(role(1)), Box::new(Concept::Top)),
                    Concept::Max(1, Role::Named(role(0)), Box::new(Concept::Top)),
                ]),
            ),
        ],
        true,
    );
}

/// `s ⊑ r` with `d : ≥2 s.⊤ ⊓ ≤1 r.⊤`.
///
/// UNSATISFIABLE. Now every `s`-pair is an `r`-pair, so the two distinct `s`-successors the
/// `≥2` demands are two distinct `r`-successors the `≤1` forbids. This is the direction that
/// makes the previous case evidence rather than a coincidence.
#[test]
fn counting_a_sub_role_does_count_towards_its_super_role() {
    assert_verdict(
        &[
            Axiom::SubRole(role(1), role(0)),
            Axiom::Type(
                individual(3),
                Concept::And(vec![
                    Concept::Min(2, Role::Named(role(1)), Box::new(Concept::Top)),
                    Concept::Max(1, Role::Named(role(0)), Box::new(Concept::Top)),
                ]),
            ),
        ],
        false,
    );
}

/// `d : ¬(A ⊔ B) ⊓ A`.
///
/// UNSATISFIABLE. `⟦¬(A ⊔ B)⟧ = Δ \ (⟦A⟧ ∪ ⟦B⟧)`, which is disjoint from `⟦A⟧`, so no
/// element is in both conjuncts. De Morgan under the negation-normal-form rewriting is what
/// this pins.
#[test]
fn a_complemented_disjunction_excludes_each_disjunct() {
    assert_verdict(
        &[Axiom::Type(
            individual(3),
            Concept::And(vec![
                Concept::Not(Box::new(Concept::Or(vec![class(0), class(1)]))),
                class(0),
            ]),
        )],
        false,
    );
}

/// `b : ∃r⁻.{a}` with `a : ≤0 r.⊤`.
///
/// UNSATISFIABLE. `⟦b⟧ ∈ ⟦∃r⁻.{a}⟧` means some `y` with `(⟦b⟧,y) ∈ ⟦r⁻⟧` and `y = ⟦a⟧`,
/// i.e. `(⟦a⟧, ⟦b⟧) ∈ ⟦r⟧` — which `≤0 r.⊤` on `a` forbids. The inverse role, the nominal
/// and the cardinality bound all have to be read together to see it.
#[test]
fn an_inverse_role_into_a_nominal_is_an_edge_out_of_it() {
    assert_verdict(
        &[
            Axiom::Type(
                individual(1),
                Concept::Some(Role::Inv(role(0)), Box::new(nominal(&[0]))),
            ),
            Axiom::Type(
                individual(0),
                Concept::Max(0, Role::Named(role(0)), Box::new(Concept::Top)),
            ),
        ],
        false,
    );
}

/// `r` transitive, `a r b`, `b r c`, and `a : ≤0 r.{c}`.
///
/// UNSATISFIABLE. Transitivity puts `(⟦a⟧, ⟦c⟧)` in `⟦r⟧`, so `a` has an `r`-successor in
/// `{c}` and the `≤0` is violated. The composed edge is entailed by the axiom, not asserted,
/// which is what makes this a test of the role's EXTENSION rather than of the triples.
#[test]
fn transitivity_supplies_the_composed_edge() {
    assert_verdict(
        &[
            Axiom::Transitive(role(0)),
            Axiom::RoleAssertion(individual(0), role(0), individual(1)),
            Axiom::RoleAssertion(individual(1), role(0), individual(2)),
            Axiom::Type(
                individual(0),
                Concept::Max(0, Role::Named(role(0)), Box::new(nominal(&[2]))),
            ),
        ],
        false,
    );
}

/// `r` asymmetric with `a : ∃r.Self`.
///
/// UNSATISFIABLE. `∃r.Self` puts `(⟦a⟧, ⟦a⟧)` in `⟦r⟧`, and asymmetry forbids `(x,y)` and
/// `(y,x)` together — with `y = x` that is exactly the self-loop, which is how asymmetry
/// subsumes irreflexivity.
#[test]
fn asymmetry_forbids_a_self_loop() {
    assert_verdict(
        &[
            Axiom::Asymmetric(role(0)),
            Axiom::Type(
                individual(0),
                Concept::SelfRestriction(Role::Named(role(0))),
            ),
        ],
        false,
    );
}

/// `⊤ ⊑ ¬∃r.Self` with an asserted self-loop `a r a`.
///
/// UNSATISFIABLE, and separated by the negated-self-restriction clash alone. This is the whole
/// content of `owl:IrreflexiveProperty`, and `asymmetry_forbids_a_self_loop` above does NOT
/// reach it: asymmetry is a role axiom checked over the edge set, while this is a CONCEPT in a
/// node's label checked against that node's own edges. Deleting the `¬∃r.Self` clash left
/// every other test in this workspace passing, and the generators effectively never produce a
/// negated self-restriction beside a matching loop, so this case has to be written down.
#[test]
fn an_irreflexive_role_forbids_an_asserted_self_loop() {
    assert_verdict(
        &[
            Axiom::Gci(
                Concept::Top,
                Concept::Not(Box::new(Concept::SelfRestriction(Role::Named(role(0))))),
            ),
            Axiom::RoleAssertion(individual(0), role(0), individual(0)),
        ],
        false,
    );

    // The same axiom without the loop is satisfiable, so the assertion above turns on the
    // clash rather than on `¬∃r.Self` being unsatisfiable on sight.
    assert_verdict(
        &[
            Axiom::Gci(
                Concept::Top,
                Concept::Not(Box::new(Concept::SelfRestriction(Role::Named(role(0))))),
            ),
            Axiom::RoleAssertion(individual(0), role(0), individual(1)),
        ],
        true,
    );
}

/// `r` and `s` disjoint with `a r b` and `a s b`.
///
/// UNSATISFIABLE. The pair `(⟦a⟧, ⟦b⟧)` would be in both extensions, and disjointness says
/// the intersection is empty. Nothing about any node's concept label says so, which is why
/// this axiom cannot be internalized as an inclusion.
#[test]
fn disjoint_roles_reject_a_shared_pair() {
    assert_verdict(
        &[
            Axiom::DisjointRoles(role(0), role(1)),
            Axiom::RoleAssertion(individual(0), role(0), individual(1)),
            Axiom::RoleAssertion(individual(0), role(1), individual(1)),
        ],
        false,
    );
}

/// The inverse-role/∀⁻ family the generated corpus reaches thinly: chains under
/// `X ⊑ ∃r.X` with universal obligations flowing back through `r⁻` across two and three
/// levels, where blocking-discipline differences would live if this rule set could
/// exhibit one.
///
/// Both cores decide every case and must agree; the satisfiable ones are additionally
/// confirmed by a looping bounded model. These shapes were first written as a deliberate
/// hunt for a knowledge base separating pairwise from label-only blocking (see the
/// blocking notes in [`crate::owl_dl::hyper`]); none separated, and the family is kept
/// so the differential's reach over this corner is a fixture, not an accident of the
/// generators.
#[test]
fn inverse_universal_chains_decide_identically_in_both_cores() {
    use crate::owl_dl::{graph, hyper, tableau};
    let x = HAND.concept_names()[0];
    let a = HAND.concept_names()[1];
    let b = *HAND.concept_names().get(2).unwrap_or(&a);
    let r = HAND.role_names()[0];
    let i0 = HAND.individual_names()[0];
    let named = |c: u32| Concept::Named(c);
    let some = |ro, c: Concept| Concept::Some(Role::Named(ro), Box::new(c));
    let all_inv = |ro, c: Concept| Concept::All(Role::Inv(ro), Box::new(c));
    let all = |ro, c: Concept| Concept::All(Role::Named(ro), Box::new(c));
    let not = |c: Concept| Concept::Not(Box::new(c));
    let and = |l: Concept, rr: Concept| Concept::And(vec![l, rr]);

    // (extra axioms, expected satisfiability, has a model within the 2-element bound) —
    // expectations follow from the semantics: a chain rooted in X∧A where an obligation
    // eventually forces ¬A back onto a node that must carry A is unsatisfiable; one whose
    // obligations are absorbable by a small loop model is satisfiable, and where the
    // smallest such loop needs more elements than the oracle's bound the third flag is
    // false — the bounded enumeration is then silent, which is its documented limit, and
    // the verdict rests on the two calculi agreeing.
    let cases: Vec<(Vec<Axiom>, bool, bool)> = vec![
        (
            vec![Axiom::Gci(named(x), all(r, all_inv(r, not(named(a)))))],
            false,
            false,
        ),
        (
            vec![Axiom::Gci(named(x), all_inv(r, not(named(a))))],
            false,
            false,
        ),
        (
            vec![
                Axiom::Gci(named(x), some(r, all_inv(r, named(b)))),
                Axiom::Gci(named(b), not(named(a))),
            ],
            false,
            false,
        ),
        (
            vec![Axiom::Gci(
                named(x),
                all(r, some(r, all_inv(r, not(named(a))))),
            )],
            true,
            false,
        ),
        (
            vec![Axiom::Gci(
                named(x),
                all(r, all(r, all_inv(r, all_inv(r, not(named(a)))))),
            )],
            false,
            false,
        ),
        (
            vec![
                Axiom::Gci(and(named(x), named(a)), all(r, not(named(a)))),
                Axiom::Gci(named(x), all(r, some(r, named(x)))),
            ],
            true,
            true,
        ),
        (
            vec![Axiom::Gci(
                named(x),
                some(r, and(named(x), all_inv(r, named(a)))),
            )],
            true,
            true,
        ),
        (
            vec![Axiom::Gci(
                named(x),
                all(
                    r,
                    all_inv(r, and(named(a), all(r, all_inv(r, not(named(a)))))),
                ),
            )],
            false,
            false,
        ),
    ];
    for (k, (extra, satisfiable, bounded_model)) in cases.iter().enumerate() {
        let mut axioms = vec![
            Axiom::Type(i0, and(named(x), named(a))),
            Axiom::Gci(named(x), some(r, named(x))),
        ];
        axioms.extend(extra.iter().cloned());
        let case = Case::assemble(HAND, &axioms);
        let cap = graph::step_cap(&case.kb);
        let h = hyper::decide(&case.kb, &Assumptions::of_kb(), cap);
        let t = tableau::decide(&case.kb, &Assumptions::of_kb(), cap);
        assert!(!h.exhausted && !t.exhausted, "case {k} must decide");
        assert_eq!(h.consistent, t.consistent, "case {k}: the cores diverge");
        assert_eq!(h.consistent, *satisfiable, "case {k}: wrong verdict");
        if *bounded_model {
            assert!(
                case.smallest_model().is_some(),
                "case {k}: this satisfiable chain has a loop model inside the bound"
            );
        }
    }
}
