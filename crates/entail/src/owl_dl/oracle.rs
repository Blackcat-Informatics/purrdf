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
//! Every generated knowledge base is decided FIVE times: by the hypertableau, by the
//! hypertableau again (determinism), by the hypertableau over the ALL-META encoding of the
//! same terminology (the ENCODING differential — see [`check`]), by the hypertableau under a
//! WEAKENED blocking condition (the BLOCKING differential — see [`blocking_differential`]),
//! and by the concept-tree tableau. The oracle is exact and
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
//! The signature above is the ABSTRACT `SHOIQ(D)` fragment, and the ENUMERATOR is confined to
//! it by construction rather than by omission: one interpretation here fixes a single domain
//! `Δ`, while a data range is a subset of a second, disjoint value domain `Δ_D` — infinite for
//! `xsd:integer` alone — that no amount of guessing over `Δ` can stand in for. Reading a data
//! range as a subset of `Δ` would be a DIFFERENT semantics, so no interpretation this file
//! builds has one.
//!
//! The CONCRETE domain is nevertheless covered, by a family that gives up the enumerator
//! rather than the coverage — see
//! [`the_concrete_domain_shapes_agree_across_the_encodings_and_the_calculi`]. Its knowledge
//! bases state data ranges and assert literal-valued data properties, so [`Case::enumerable`]
//! is false of every one of them and the oracle is not run at all; what decides them is the
//! two differentials that need no enumeration — the two TBox ENCODINGS against each other, and
//! the two CALCULI against each other — plus the requirement that the corpus reach both
//! verdicts. That is stated as the family's own floors rather than borrowed from the
//! enumerated families' assertions, because an assertion about `Δ` is not evidence about
//! `Δ_D`.
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
use purrdf_core::TermValue;
use purrdf_xsd::XsdDatatype;
use purrdf_xsd::range::{DataRange, Facet};

use crate::owl_dl::Kb;
use crate::owl_dl::concept::{Concept, Decomp, Role};
use crate::owl_dl::graph::{self, Assumptions};
use crate::owl_dl::{hyper, parser, tableau};
use crate::vocab::{XSD_INTEGER, XSD_STRING};

// ── The signature ───────────────────────────────────────────────────────────────

/// The class term ids a generated concept's named leaves are drawn from (`A`, `B`, `C`, `D`).
///
/// Four rather than three because one family needs four: the co-typed property defines every
/// class of its signature by an equivalence and asserts every one of them of ONE individual,
/// and `n = 4` is the size of the shape it is about. Every other signature takes the first
/// two or three, so the fourth name costs them nothing — a signature enumerates
/// `2^(k · self.concepts)` concept extensions, never `2^(k · CONCEPT_NAMES.len())`.
const CONCEPT_NAMES: [u32; 4] = [10, 11, 12, 13];

/// The object-property term ids a generated role is drawn from (`r`, `s`).
const ROLE_NAMES: [u32; 2] = [20, 21];

/// The individual term ids a generated nominal or assertion is drawn from
/// (`a`, `b`, `c`, `d`).
const INDIVIDUAL_NAMES: [u32; 4] = [30, 31, 32, 33];

/// The DATA-property term ids the concrete-domain family's assertions and restrictions are
/// drawn from (`u`, `v`).
///
/// Kept apart from [`ROLE_NAMES`] because the two quantify over different domains: a `u`-edge
/// reaches an element of `Δ_D`, and every filler this family puts under one is a data range or
/// a literal nominal. Mixing the two would generate knowledge bases outside OWL 2 DL — a
/// property that is both an object and a data property — and the differentials would then be
/// comparing two readings of a document the specification rules out.
const DATA_PROPERTY_NAMES: [u32; 2] = [40, 41];

/// The literals the concrete-domain family asserts, as `(lexical form, datatype IRI)`.
///
/// A literal's term id IS its index here, which is what lets a generated axiom name one
/// before any knowledge base exists: [`Case::encoded`] interns this table into the knowledge
/// base's term interner FIRST and in order, and interning is store-once and first-seen, so
/// index and id coincide. Nothing else in the file is interned there — the class, role and
/// individual names are bare symbols the interner never sees — so no id can collide.
///
/// The four are chosen for what separates them rather than for variety. `"1"` and `"01"` are
/// two RDF TERMS denoting ONE element of `Δ_D`, so a `≥2` restriction may not count them as
/// two; `"2"` is a second value of the same space; and `"cat"` is a value of a DISJOINT one,
/// so asserting it where an integer range is demanded is a clash the abstract rules cannot
/// see.
const LITERALS: [(&str, &str); 4] = [
    ("1", XSD_INTEGER),
    ("01", XSD_INTEGER),
    ("2", XSD_INTEGER),
    ("cat", XSD_STRING),
];

/// `xsd:integer` — the whole value space, infinite, so it bounds no counting question.
const DR_INTEGER: u32 = 0;

/// `xsd:string` — a value space DISJOINT from the integers, which is what makes a node
/// carrying both ranges a clash.
const DR_STRING: u32 = 1;

/// `xsd:integer[0 … 2]` — three values, stated with facets, so `≥4 u.DR` is refutable by
/// counting alone.
const DR_SMALL: u32 = 2;

/// `{1}` — one value, the narrowest range a `∀u.DR` can narrow a counting question down to.
const DR_ONE: u32 = 3;

/// The data ranges the concrete-domain family draws from, in the order [`Case::encoded`]
/// interns them — so `Concept::Data(i)` names `data_ranges()[i]`, and the `DR_*` constants
/// above are those indices.
///
/// Every one of them is EXACTLY decided by `purrdf-xsd`, and their cardinalities are pinned by
/// [`the_concrete_domains_arithmetic_is_pinned`]. That matters because the concrete-domain
/// rules answer `Undecided` as "no clash": a family built on ranges the decision procedure
/// could not pin down would generate cases whose clashes are silently withheld, and would then
/// report a corpus of consistent verdicts as coverage.
fn data_ranges() -> Vec<DataRange> {
    vec![
        DataRange::Datatype(XsdDatatype::Integer),
        DataRange::Datatype(XsdDatatype::String),
        DataRange::Restriction {
            base: XsdDatatype::Integer,
            facets: vec![
                Facet::MinInclusive(integer(0)),
                Facet::MaxInclusive(integer(2)),
            ],
        },
        DataRange::OneOf(vec![integer(1)]),
    ]
}

/// The `xsd:integer` value `n`, parsed by the same code the reverse mapping parses a literal
/// with.
fn integer(n: i64) -> purrdf_xsd::XsdValue {
    purrdf_xsd::parse(&n.to_string(), XsdDatatype::Integer).expect("an integer literal parses")
}

/// The largest domain any property enumerates, and so the width of the bitmask a subset of
/// the domain is held in.
const MAX_DOMAIN: usize = 3;

/// A readable name for a signature term id, for a failure message.
fn term_name(id: u32) -> String {
    let letters = ["A", "B", "C", "D"];
    if let Some(i) = CONCEPT_NAMES.iter().position(|&x| x == id) {
        return letters[i].to_owned();
    }
    if let Some(i) = ROLE_NAMES.iter().position(|&x| x == id) {
        return ["r", "s"][i].to_owned();
    }
    if let Some(i) = INDIVIDUAL_NAMES.iter().position(|&x| x == id) {
        return ["a", "b", "c", "d"][i].to_owned();
    }
    if let Some(i) = DATA_PROPERTY_NAMES.iter().position(|&x| x == id) {
        return ["u", "v"][i].to_owned();
    }
    // A concrete-domain case's literals are the only terms whose id is small, because they are
    // the only ones interned; see [`LITERALS`].
    if let Some(&(lexical, datatype)) = LITERALS.get(id as usize) {
        return format!("{lexical:?}^^<{datatype}>");
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

/// The ∀-EQUIVALENCE signature: three classes and two roles, at the domain bound two roles
/// allow.
///
/// Sized for the interaction the equivalence-over-untyped-restrictions ontology is made of — a
/// `∀`-restriction whose filler is an intersection, an exact cardinality, and an inverse role —
/// which needs a class for the restricted concept, one for the intersection's named conjunct
/// and one for the inner `∀`'s filler, plus a role to quantify over and a second to count.
const FORALL_EQUIVALENCE: Signature = Signature {
    concepts: 3,
    roles: 2,
    individuals: 2,
    max_domain: 2,
};

/// The CO-TYPED signature: four class names, ONE role, ONE individual, two domain elements.
///
/// Sized by what the co-typed shape needs and by what the oracle can afford. FOUR class names,
/// because the property below defines every one of them by an equivalence and asserts every
/// one of them of the single individual — four is the `n` the shape is about, and a signature
/// of three could not reach it. ONE individual, because co-typing is the whole point: the
/// disjunctions the four converse inclusions produce have to interleave on one node rather
/// than stand beside each other. And that is what pays for four classes — the enumeration is
/// `2^(k·4) · 2^(k²) · k`, which at `k ≤ 2` is 8,224 interpretations, less than any other
/// family here.
const CO_TYPED: Signature = Signature {
    concepts: 4,
    roles: 1,
    individuals: 1,
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

/// The CONCRETE-DOMAIN signature: two classes, two individuals, and NO abstract role.
///
/// The roles this family quantifies over are the two DATA properties of
/// [`DATA_PROPERTY_NAMES`], which no [`Signature`] field counts, because no field of a
/// [`Signature`] describes anything the enumerator does not enumerate — and this family's
/// knowledge bases are never enumerated at all (see [`Case::enumerable`]). Two classes are
/// what the guarded shapes need (`A ⊑ ∀u.DR` beside `A ⊓ B ⊑ ⊥`), and `max_domain` is stated
/// for the same reason the other signatures state it: [`bounded_domain`] reads it. It is never
/// reached here.
const DATA: Signature = Signature {
    concepts: 2,
    roles: 0,
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
    /// A DATA-property assertion `a u "lexical"^^datatype`, naming the literal by its index
    /// in [`LITERALS`] — which is also the term id it is interned under.
    ///
    /// Held apart from [`Axiom::RoleAssertion`] even though [`Kb`] holds both in
    /// [`Kb::abox_roles`]: the object of this one is an element of `Δ_D`, which is what
    /// [`Case::encoded`] must see to register the literal's value and singleton range.
    DataAssertion(u32, u32, u32),
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
        // THE CONCRETE DOMAIN, if these axioms reach it, before anything else: the literal
        // table has to be the first thing interned for a literal's term id to be its index in
        // it, and the family's data ranges have to be interned in their own order for a
        // generated `Concept::Data(i)` to name the range it was written for.
        let concrete = axioms.iter().any(reaches_the_data_domain);
        if concrete {
            for &(lexical, datatype) in &LITERALS {
                kb.interner
                    .intern(TermValue::typed_literal(lexical, datatype));
            }
            for range in data_ranges() {
                kb.data_ranges.intern(range);
            }
        }
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
                Axiom::RoleAssertion(a, p, b) | Axiom::DataAssertion(a, p, b) => {
                    kb.abox_roles.push((*a, *p, *b));
                }
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
        // Every literal an axiom REACHES gets its value class and its singleton data range,
        // through the very function the reverse mapping uses — see
        // [`parser::register_literals`]. A second implementation of the ill-typed, unmodelled
        // and value-space rules here would be a differential comparing this file against
        // itself. It runs after the concepts are interned because a literal is reached
        // through an interned NOMINAL as well as through an assertion.
        if concrete {
            kb.literal_class = parser::register_literals(
                &kb.interner,
                &mut kb.table,
                &mut kb.data_ranges,
                &kb.abox_roles,
                &mut kb.abox_types,
                &mut kb.boundaries,
                None,
            )
            .expect("registering literals polls no stop signal");
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
            // A concrete-domain leaf cannot occur, because a case whose knowledge base holds
            // any data range is never enumerated: [`Case::enumerable`] is false of it and
            // [`check`] returns before reaching this recursion. An interpretation here fixes
            // ONE domain, and `Δ_D` is a second one — see the module docs.
            Decomp::Data(_) | Decomp::NegData(_) => {
                unreachable!("a data range reached an enumeration over the abstract domain alone")
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
            // `take(..)` rather than an index range: the arrays are sized by the
            // full name tables, but only the signature's first `sig.roles` slots
            // are live for this case. Iterating the live prefix directly keeps
            // that bound in one place and drops the bounds check per write.
            for role in i.roles.iter_mut().take(self.sig.roles) {
                *role = Relation::decode(rest % relation_codes, size);
                rest /= relation_codes;
            }
            if !self.role_axioms_hold(&i) {
                continue;
            }
            for concept_code in 0..concept_codes.pow(self.sig.concepts as u32) {
                let mut rest = concept_code;
                for concept in i.concepts.iter_mut().take(self.sig.concepts) {
                    *concept = (rest % concept_codes) as u32;
                    rest /= concept_codes;
                }
                for individual_code in 0..elements.pow(self.sig.individuals as u32) {
                    let mut rest = individual_code;
                    for individual in i.individuals.iter_mut().take(self.sig.individuals) {
                        *individual = (rest % elements) as usize;
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

    /// Whether the bounded enumerator can say ANYTHING about this case.
    ///
    /// It cannot once the knowledge base reaches the concrete domain. An [`Interpretation`]
    /// here fixes one finite domain `Δ` and guesses a subset of it per class name; a data
    /// range is a subset of a SECOND, disjoint domain `Δ_D` whose extension the datatype map
    /// fixes rather than the interpretation, and which is infinite for `xsd:integer` alone.
    /// There is no bound to raise and no guess to add: the enumerator would have to become a
    /// different program, deciding `xsd:integer[0…2] ⊓ xsd:string = ∅` from the value spaces —
    /// which is precisely what [`crate::owl_dl::data`] already does and what the tableau is
    /// being CHECKED on here.
    ///
    /// So the concrete-domain family gives the enumerator up rather than fake it, and
    /// [`check`] says so by tallying such a case as `concrete` and asserting nothing about a
    /// model. What decides those cases is stated at
    /// [`the_concrete_domain_shapes_agree_across_the_encodings_and_the_calculi`].
    fn enumerable(&self) -> bool {
        self.kb.data_ranges.is_empty()
    }

    /// What the oracle has to say about this case, for a failure message: a model if it found
    /// one, and otherwise WHICH of the two silences this is — no model up to the bound, or no
    /// enumeration at all.
    fn oracle_text(&self) -> String {
        if !self.enumerable() {
            return format!(
                "the oracle does not enumerate this case: its knowledge base reaches Δ_D, \
                 which no interpretation over Δ can stand in for\n{}",
                concrete_legend()
            );
        }
        self.smallest_model().map_or_else(
            || "the oracle found no model up to the signature's bound".to_owned(),
            |model| format!("oracle model:\n{}", self.model_text(&model)),
        )
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

// ── The concrete domain ─────────────────────────────────────────────────────────

/// Whether `id` names one of [`LITERALS`].
///
/// A comparison against the table's length rather than a lookup, and sound because of how the
/// ids are laid out: the literals are the only terms this file interns, so they occupy
/// `0 … LITERALS.len()-1`, while every class, role, individual and data-property name is a
/// bare symbol from 10 upwards that the interner never sees.
fn is_literal_index(id: u32) -> bool {
    (id as usize) < LITERALS.len()
}

/// Whether `role` is one of [`DATA_PROPERTY_NAMES`].
fn is_data_property(role: Role) -> bool {
    let (Role::Named(p) | Role::Inv(p)) = role;
    DATA_PROPERTY_NAMES.contains(&p)
}

/// Whether `c` mentions the concrete domain — a data range, a nominal naming a literal (which
/// is how `owl:hasValue` over a data property reads), or a restriction over a DATA property.
///
/// The third is not redundant: `≤1 u.⊤` names no range and no literal, and it is still a
/// statement about how many elements of `Δ_D` a node reaches. It is also a shape the
/// enumerator could not touch even if it wanted to — [`Case::role_index`] knows only the
/// signature's abstract roles — so reading it as abstract would not weaken a check, it would
/// panic in one.
fn mentions_the_data_domain(c: &Concept) -> bool {
    match c {
        Concept::Data(_) => true,
        Concept::Nominal(members) => members.iter().copied().any(is_literal_index),
        Concept::Not(inner) => mentions_the_data_domain(inner),
        Concept::Some(role, inner) | Concept::All(role, inner) => {
            is_data_property(*role) || mentions_the_data_domain(inner)
        }
        Concept::Min(_, role, inner) | Concept::Max(_, role, inner) => {
            is_data_property(*role) || mentions_the_data_domain(inner)
        }
        Concept::SelfRestriction(role) => is_data_property(*role),
        Concept::And(members) | Concept::Or(members) => {
            members.iter().any(mentions_the_data_domain)
        }
        Concept::Top | Concept::Bottom | Concept::Named(_) => false,
    }
}

/// Whether `axiom` reaches the concrete domain, and so whether [`Case::encoded`] must intern
/// the literal table and the family's data ranges before it builds anything.
///
/// Asked of the AXIOMS rather than declared by the signature, so that a family which reaches
/// `Δ_D` only through some of its cases could not silently get an abstract knowledge base for
/// the rest — the ids a generated `Concept::Data(i)` carries would then name nothing.
fn reaches_the_data_domain(axiom: &Axiom) -> bool {
    match axiom {
        Axiom::DataAssertion(..) => true,
        Axiom::Gci(sub, sup) => mentions_the_data_domain(sub) || mentions_the_data_domain(sup),
        Axiom::Type(_, c) => mentions_the_data_domain(c),
        Axiom::SameAs(a, b) | Axiom::DifferentFrom(a, b) => {
            is_literal_index(*a) || is_literal_index(*b)
        }
        Axiom::RoleAssertion(_, p, _) => is_data_property(Role::Named(*p)),
        Axiom::SubRole(..)
        | Axiom::InverseOf(..)
        | Axiom::Transitive(_)
        | Axiom::Asymmetric(_)
        | Axiom::DisjointRoles(..) => false,
    }
}

/// What the numbers in a concrete-domain case's axioms name, for a failure message: a
/// `Data(i)` leaf's range and a literal's term id are both indices into tables this file
/// fixes, and neither reads as anything without them.
fn concrete_legend() -> String {
    let mut lines = vec!["data ranges:".to_owned()];
    for (index, range) in data_ranges().iter().enumerate() {
        lines.push(format!("  Data({index}) = {range:?}"));
    }
    lines.push("literals:".to_owned());
    for (index, &(lexical, datatype)) in LITERALS.iter().enumerate() {
        lines.push(format!("  #{index} = {lexical:?}^^<{datatype}>"));
    }
    lines.join("\n")
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
    /// Cases the bounded enumerator was not run on AT ALL, because the knowledge base reaches
    /// the concrete domain — see [`Case::enumerable`].
    ///
    /// Not a weaker form of `unbounded`: that one is an enumeration that finished and found
    /// nothing, this one is an enumeration that was never a question. Both differentials still
    /// decide such a case, and the property that generates them says so at its call site.
    concrete: u32,
    /// How the hypertableau DECIDED, over every case it decided: consistent, and inconsistent.
    ///
    /// Reported for every property and floored by the one whose enumerator is silent. A corpus
    /// that reached only one verdict would be comparing two encodings and two calculi that
    /// agree because nothing in it ever closes a branch — which is exactly how a concrete
    /// domain whose clash rules never fired would look.
    consistent: u32,
    /// The other half of the verdict population above.
    inconsistent: u32,
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
    /// Cases where the hypertableau finished under BOTH blocking conditions — the shipped
    /// pairwise one and the label-only mutation — so their verdicts were compared and agreed.
    ///
    /// This is the population the `hyper` module docs' empirical claim about the
    /// predecessor-label half of the blocking signature rests on, and it is floored for the
    /// same reason the other two differentials are.
    blocking: u32,
    /// WORK units the hypertableau spent over EVERY case of this property, summed.
    work: u64,
    /// The most work any ONE case of this property spent.
    max_case_work: u64,
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
        self.modelled + self.refuted + self.unbounded + self.exhausted + self.concrete
    }

    /// Fold one hypertableau decision's cost in: work sums, sizes peak.
    fn spent(&mut self, decision: &graph::Decision) {
        self.work = self.work.saturating_add(decision.work);
        self.max_case_work = self.max_case_work.max(decision.work);
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
/// case that DECIDES spends is 306 — one knowledge base in `complement ⊗ disjunction`. 350 is
/// that maximum plus about a seventh.
///
/// The criterion is deliberate: a case the calculus can decide must not be reported as
/// exhausted, because an exhausted case is one [`check`] compares NEITHER differential on —
/// so a cap set below a decidable case's cost quietly shrinks what the suite checks while
/// every assertion still passes. A cap of 300 does exactly that to the 306-round case, and
/// the way it shows is one `exhausted` case in a corpus that otherwise has none.
///
/// The maximum used to be 439, and 500 was this constant, because the `⊔`-rule selected the
/// NARROWEST open disjunction rather than the first one — a rule whose own measurements
/// ([`Hyper::find_branch`](crate::owl_dl::hyper)) retired it. The case it cost 439 rounds
/// decides in 178 under the first-open rule; the 306-round case above costs the same under
/// both, which is what makes it the corpus's ceiling rather than an artifact of either.
///
/// What the cap does NOT try to accommodate is the one case that no affordable cap decides.
/// The `wide` corpus contains a knowledge base whose completion graph simply grows with
/// whatever it is given — 122 nodes at a cap of 350, 139 at 400, 172 at 500, 205 at 600, 339
/// at 1000 — and it exhausts at every one of them. Chasing it is what the superlinear cost
/// above buys nothing for: raising the cap from 350 to 500 costs the whole suite about two
/// and a half times its wall time, and almost all of that increase is that single case running
/// longer before being truncated anyway. So the cap is set to decide everything decidable and
/// to truncate that one, which the ≤5% exhausted quota in [`run_property`] absorbs at 1 case
/// in 9,800.
const STEP_CAP: u64 = 350;

/// The budget this suite decides a generated knowledge base under: the narrowed round cap
/// above, and the knowledge base's OWN work cap.
///
/// The round cap is narrowed and the work cap is not, and the asymmetry is the point. The
/// round cap has to be narrowed because rounds are not a cost — the work inside one grows
/// geometrically with the completion graph, so a corpus of thousands of adversarial knowledge
/// bases cannot let a round budget stand. The work cap already IS a cost, sized by
/// [`work_cap`](graph::work_cap) from the knowledge base's size, and these knowledge bases are
/// tiny: narrowing it would only trade one truncation criterion for another while making the
/// suite's exhausted share depend on two numbers instead of one.
fn suite_budget(kb: &Kb, rounds: u64) -> graph::Budget {
    let derived = graph::Budget::for_kb(kb);
    graph::Budget {
        steps: derived.steps.min(rounds),
        work: derived.work,
    }
}

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

/// THE BLOCKING DIFFERENTIAL: decide `case` a second time with the hypertableau's blocking
/// signature MUTATED to compare labels alone, and require the verdict to be the one the
/// shipped pairwise condition reached.
///
/// # What this is evidence about
///
/// The [`hyper`] module documents the pairwise (double) blocking condition — same label, same
/// PREDECESSOR label, same incoming edge — and, beside it, an empirical honesty: no knowledge
/// base is known that separates the predecessor-label half from label-only blocking in this
/// rule set. That claim used to be a paragraph describing a mutation nobody could re-run. This
/// function is the mutation, and it runs over every knowledge base every property in this file
/// generates.
///
/// Label-only blocking blocks at least as many nodes — every pairwise blocker is a label
/// blocker, and some label blockers are not pairwise ones — and blocking withholds exactly one
/// rule, the `≥`-rule. So the mutation never builds a LARGER completion graph than the shipped
/// condition does, and the direction a difference would show up in is
/// therefore sharp: a knowledge base the shipped condition refutes and the mutation calls
/// consistent is a witness that the withheld expansion was the one that closed the branch, and
/// that the predecessor-label half is load-bearing after all. That is a discovery about the
/// calculus, not a tolerance, so it fails the run.
///
/// # Why the flag is flipped rather than a third knowledge base assembled
///
/// Blocking is a property of the SEARCH and not of the encoding: the clause table, the concept
/// ids, the ABox and both TBox encodings are untouched by it. Assembling a second `Kb` would
/// therefore compare two searches over knowledge bases that are equal by construction while
/// costing every case another clausification, and the comparison would be over a `Kb` whose
/// only interesting property is that it is the same one. The flag is set for the one call and
/// cleared immediately, so everything downstream — the failure messages, the oracle — sees the
/// knowledge base as the shipped discipline decides it.
fn blocking_differential(
    case: &mut Case,
    shipped: &graph::Decision,
    cap: graph::Budget,
    tally: &RefCell<Tally>,
) -> Result<(), TestCaseError> {
    case.kb.label_only_blocking = true;
    let mutated = hyper::decide(&case.kb, &Assumptions::of_kb(), cap);
    case.kb.label_only_blocking = false;
    if shipped.exhausted || mutated.exhausted {
        return Ok(());
    }
    if shipped.consistent != mutated.consistent {
        return Err(TestCaseError::fail(format!(
            "THE BLOCKING CONDITION IS LOAD-BEARING: pairwise blocking says {}, label-only \
             blocking says {}. This is the knowledge base the `hyper` module docs say is not \
             known to exist — it separates the predecessor-label half of the blocking \
             signature from label-only blocking, and the docs' empirical claim is false of \
             it.\npairwise: {shipped:?}\nlabel-only: {mutated:?}\naxioms:\n{}\n{}",
            shipped.consistent,
            mutated.consistent,
            case.axioms_text(),
            case.oracle_text()
        )));
    }
    tally.borrow_mut().blocking += 1;
    Ok(())
}

/// Check one generated knowledge base, recording how it resolved.
///
/// Nine things happen here: the hypertableau is asked twice and must answer identically; its
/// verdict is compared against the concept-tree tableau's, which must AGREE; BOTH cores' shape
/// counters are held to [`counters_are_coherent`]; it is compared
/// against ITSELF over the all-meta encoding of the same terminology, which must also agree;
/// it is compared against ITSELF again under the label-only blocking mutation, which must
/// agree too ([`blocking_differential`]); a case neither core could finish is skipped; a case
/// whose knowledge base reaches `Δ_D` is recorded as one the enumeration cannot speak to
/// ([`Case::enumerable`]); where the oracle exhibits a model the hypertableau's `consistent`
/// is asserted unconditionally; and where the oracle finds NO model and
/// [`forces_unnamed_element`] says the bound was sufficient, `consistent` is asserted to be
/// false.
fn check(
    sig: Signature,
    axioms: &[Axiom],
    tally: &RefCell<Tally>,
    rounds: u64,
) -> Result<(), TestCaseError> {
    let mut case = Case::assemble(sig, axioms);
    let cap = suite_budget(&case.kb, rounds);
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
    // THE BLOCKING DIFFERENTIAL: the same knowledge base, the same encoding and the same
    // calculus, under a WEAKENED blocking condition.
    blocking_differential(&mut case, &first, cap, tally)?;
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
                case.oracle_text()
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
            // Both sides of the differential, not just the absorbed one: `case.kb.meta` is
            // whatever [`absorb`](crate::owl_dl::absorb) left un-guarded under the encoding
            // that DID absorb, and `internalized.kb.meta` is the comparand that actually
            // decided `other` — the full internalized TBox with nothing absorbed. Printing
            // only the first would show a reader the antecedents absorption chose to guard
            // and never the axioms the all-meta run reasoned over instead.
            return Err(TestCaseError::fail(format!(
                "the two TBox encodings disagree: absorbed says {}, all-meta says {}\n\
                 absorbed clauses: {:?}\nabsorbed meta: {:?}\nall-meta meta: {:?}\naxioms:\n{}",
                first.consistent,
                other.consistent,
                case.kb.absorbed,
                case.kb.meta,
                internalized.kb.meta,
                case.axioms_text(),
            )));
        }
        tally.borrow_mut().encodings += 1;
    }
    if first.exhausted {
        tally.borrow_mut().exhausted += 1;
        return Ok(());
    }
    if first.consistent {
        tally.borrow_mut().consistent += 1;
    } else {
        tally.borrow_mut().inconsistent += 1;
    }
    // THE CONCRETE DOMAIN. A knowledge base that states a data range is decided by both
    // differentials above and by nothing below: the enumeration guesses subsets of ONE finite
    // domain, and `Δ_D` is a second one it cannot represent. Tallied rather than skipped
    // silently, and floored at the call site, so "the oracle said nothing" is a number a
    // property has to state rather than an absence.
    if !case.enumerable() {
        tally.borrow_mut().concrete += 1;
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

/// What a property's ORACLE direction rests on — the health check [`run_property`] holds its
/// tally to once the run is over.
///
/// Three arms because there are three genuinely different situations, and collapsing them into
/// one number would make two of them read like a floor somebody forgot to raise.
#[derive(Debug, Clone, Copy)]
enum Bound {
    /// The enumeration decides every case, and the OVER-PERMISSIVE direction — "no model up to
    /// the bound" read as "no model" — was asserted at least this many times.
    ///
    /// A floor rather than an equality: the seeds are fixed so the count is deterministic, but
    /// the question is "does this direction bind often enough to be checking something", and a
    /// floor set ~20% below the observed count answers it without turning every generator
    /// tweak into a re-pin.
    Asserted(u32),
    /// [`bounded_domain`] can never hold for this property, so the over-permissive direction
    /// is structurally unavailable — either the signature names more individuals than its
    /// enumeration has elements, or every axiom it generates quantifies and
    /// [`forces_unnamed_element`] is true of all of them.
    ///
    /// Asserted as an equality against zero. The call site says WHICH of the two reasons
    /// applies, and a change to either has to revisit this rather than let the number quietly
    /// start meaning something.
    Impossible,
    /// The knowledge bases inhabit TWO domains, so the enumerator is not run at all — see
    /// [`Case::enumerable`].
    ///
    /// What is asserted instead is stated here rather than inherited: that the enumerator's
    /// silence is TOTAL (every deciding case is tallied `concrete`, so no case slipped into
    /// an enumerated direction that would be reasoning about `Δ_D` with a guess over `Δ`), and
    /// that the corpus reaches BOTH verdicts at least this often — because two encodings and
    /// two calculi agreeing over a corpus nothing ever refutes is agreement about nothing.
    Concrete {
        /// The fewest cases the corpus must decide CONSISTENT.
        consistent: u32,
        /// The fewest it must decide INCONSISTENT — the clashes that reach the verdict
        /// through the concrete domain.
        inconsistent: u32,
    },
}

/// Run one property: `cases` generated knowledge bases over `sig`, each put through
/// [`check`], and then a health check on the tally so the property cannot pass by asserting
/// nothing.
// Nine parameters because nine independent knobs are what a `proptest` property over a
// generated corpus needs named at the call site — the test's name, the signature, the case
// count, the seed tag, the oracle direction's floor, the round and step ceilings, the work
// ceiling, and the axiom strategy — and bundling them into a struct would hide which ones a
// given `#[test]` chose to override.
#[allow(clippy::too_many_arguments)]
fn run_property(
    name: &str,
    sig: Signature,
    cases: u32,
    tag: u8,
    bound: Bound,
    rounds: u64,
    step_ceiling: u64,
    work_ceiling: u64,
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
    if let Err(failure) = runner.run(strategy, |axioms| check(sig, &axioms, &tally, rounds)) {
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
    // THE WORK CEILING, on the same argument and for the quantity the round total cannot
    // express. A round is a PASS rather than a unit of cost, so a change that made each round
    // several times more expensive while taking fewer of them would pass the ceiling above
    // and fail here — which is exactly the direction the work budget exists to watch.
    assert!(
        tally.work <= work_ceiling,
        "{name} spent {} work units over its {cases} cases, above its ceiling of \
         {work_ceiling}. The search's per-round cost has changed materially, which the round \
         total above cannot show: {tally:?}",
        tally.work
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
    // The BLOCKING population, floored on the same argument. The `hyper` module docs make an
    // empirical claim about the predecessor-label half of the blocking signature, and this is
    // the population that claim is measured over: a share that collapsed would leave the claim
    // resting on whatever cases happened to survive both caps.
    assert!(
        tally.blocking * 20 >= cases * 19,
        "{name} compared the two blocking conditions on fewer than 95% of its cases, so the \
         claim that the pairwise condition changes no verdict rests on almost nothing: \
         {tally:?}"
    );
    // The ORACLE direction, in whichever of its three forms this property has — see [`Bound`],
    // where each arm says what it is asserting and why the other two would be dishonest for
    // it. An assertion that never fires reads exactly like one that passes, so no property
    // gets to be silent here.
    match bound {
        Bound::Asserted(floor) => {
            assert_modelled(name, cases, &tally);
            assert!(
                tally.bound_asserted >= floor,
                "{name} asserted the over-permissive direction only {} time(s), below its \
                 floor of {floor}. A generator change has narrowed what this property checks: \
                 {tally:?}",
                tally.bound_asserted
            );
        }
        Bound::Impossible => {
            assert_modelled(name, cases, &tally);
            assert_eq!(
                tally.bound_asserted, 0,
                "{name} now asserts the over-permissive direction {} time(s) where its \
                 signature made that impossible — its individuals used to outnumber its domain \
                 bound. Give it a real floor: {tally:?}",
                tally.bound_asserted
            );
        }
        Bound::Concrete {
            consistent,
            inconsistent,
        } => {
            // TOTAL silence, asserted as an equality: every case that decided was one the
            // enumerator could say nothing about. A case that slipped past this would have
            // been decided by guessing subsets of `Δ` for a knowledge base whose axioms range
            // over `Δ_D`, which is a different semantics rather than a weaker check.
            assert_eq!(
                tally.concrete + tally.exhausted,
                tally.total(),
                "{name} enumerated {} of its cases, but its knowledge bases inhabit two \
                 domains and the enumeration guesses subsets of one: {tally:?}",
                tally.total() - tally.concrete - tally.exhausted
            );
            assert!(
                tally.consistent >= consistent && tally.inconsistent >= inconsistent,
                "{name} reached {} consistent and {} inconsistent verdicts, below its floors \
                 of {consistent} and {inconsistent}. Two encodings and two calculi agreeing \
                 over a corpus that never clashes — or never completes — is agreement about \
                 nothing: {tally:?}",
                tally.consistent,
                tally.inconsistent
            );
        }
    }
}

/// The UNCONDITIONAL direction's floor: a quarter of the cases decided by an exhibited model.
///
/// Split out because it belongs to the two ENUMERATED arms of [`Bound`] and to neither the
/// concrete one nor a caller: a property whose oracle never runs has no models to exhibit, and
/// asserting `0 * 4 >= 0` there would be a floor that reads as passing while measuring
/// nothing.
fn assert_modelled(name: &str, cases: u32, tally: &Tally) {
    assert!(
        tally.modelled * 4 >= cases,
        "{name} decided fewer than a quarter of its cases by an exhibited model, so the \
         unconditional direction is barely being asserted: {tally:?}"
    );
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
        // Three individuals against a two-element enumeration: `bounded_domain` can never
        // hold, so the over-permissive direction is structurally unavailable here.
        Bound::Impossible,
        STEP_CAP,
        // Measured 1,900 rounds, of which 350 are the one case that exhausts at any cap.
        2_100,
        // Measured 9,088,012 work units — the most of any property, because the case that
        // exhausts at any cap grows its completion graph for every round it is given.
        10_000_000,
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
        Bound::Asserted(20),
        STEP_CAP,
        // Measured 1,179 rounds.
        1_300,
        // Measured 7,116,702 work units over 1,179 rounds: this family's rounds are the
        // dearest in the suite, which is a fact only this counter states.
        7_830_000,
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
        Bound::Asserted(10),
        STEP_CAP,
        // Measured 848 rounds — the counting family is where the first-open `⊔`-rule is
        // dearest (777 under the narrowest-first selection whose own measurements, recorded
        // at `Hyper::find_branch`, retired it), and this is what that rule costs here.
        940,
        // Measured 53,946 work units.
        59_500,
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
        Bound::Asserted(250),
        STEP_CAP,
        // Measured 699 rounds.
        770,
        // Measured 13,294 work units.
        14_700,
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
        // Three individuals against a two-element enumeration, as `wide`: structurally
        // unavailable rather than merely unobserved.
        Bound::Impossible,
        STEP_CAP,
        // Measured 2,305 rounds.
        2_540,
        // Measured 113,908 work units.
        125_400,
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
        Bound::Asserted(700),
        STEP_CAP,
        // Measured 9,221 rounds — the most expensive property, and the one holding the
        // 306-round case STEP_CAP is sized for.
        10_150,
        // Measured 594,418 work units — many cheap rounds rather than few dear ones, which
        // is the opposite shape to `deep` and is what the two counters together say.
        654_000,
        &arb_axioms(axiom),
    );
}

/// THE CASE THE `⊔`-RULE'S BRANCH SELECTION WAS MEASURED ON, lifted out of the corpus above
/// and pinned by COST.
///
/// It is one knowledge base of `complement ⊗ disjunction`, written out verbatim — triple
/// negation, duplicated disjunct and all — because what it pins is a number and a number is
/// only reproducible from the exact axioms. Under the narrowest-first selection rule that
/// [`Hyper::find_branch`](crate::owl_dl::hyper) used to apply it cost 439 rounds, the most of
/// any deciding case in the whole suite and the reason [`STEP_CAP`] had to be 500; under the
/// first-open rule that replaced it, it costs 178.
///
/// # Why this needs its own pin, when the property already has a ceiling
///
/// The property ceiling is a SUM over 2,500 cases with a tenth of headroom, and 261 extra
/// rounds is under 3% of that sum: the regression passed the ceiling comfortably and showed up
/// only as a cap that had to be widened, which is a knob rather than a failure. A per-case pin
/// is what turns it back into one. The reverse-mapping boundary cannot carry this row either —
/// the same axioms written as OWL and put through
/// `purrdf_validate::regime::consistency_to_string` cost 202 rounds under BOTH rules, because
/// that path interns the concepts in a different order — so the step ledger in `purrdf-validate`
/// is not where this belongs; here, over the generator's own encoding, is.
///
/// The whole of [`check`] runs on it, so the cost is asserted without weakening anything a
/// generated case is otherwise held to: both differentials, the encoding comparison and the
/// oracle's own verdict all apply.
#[test]
fn the_case_the_branch_selection_was_measured_on_costs_exactly_what_it_is_pinned_to() {
    let a = INDIVIDUAL_NAMES[0];
    let b = INDIVIDUAL_NAMES[1];
    let c = INDIVIDUAL_NAMES[2];
    let cls = |i: usize| Concept::Named(CONCEPT_NAMES[i]);
    let not = |c: Concept| Concept::Not(Box::new(c));
    let axioms = [
        Axiom::Gci(
            not(not(not(Concept::nominal(vec![a, b])))),
            not(Concept::Or(vec![cls(0), cls(2)])),
        ),
        Axiom::Type(c, not(Concept::Or(vec![cls(1), cls(2)]))),
        Axiom::DifferentFrom(a, b),
        Axiom::Type(b, Concept::Top),
        Axiom::Gci(
            not(Concept::Or(vec![cls(1), cls(2)])),
            Concept::nominal(vec![a]),
        ),
        Axiom::Gci(
            not(Concept::Or(vec![cls(1), cls(2)])),
            Concept::Or(vec![Concept::And(vec![cls(0), not(cls(0))]), cls(0)]),
        ),
    ];
    let tally = RefCell::new(Tally::default());
    if let Err(failure) = check(BOOLEAN, &axioms, &tally, STEP_CAP) {
        panic!("the pinned case no longer passes what every generated case passes: {failure}");
    }
    let tally = tally.into_inner();
    assert_eq!(
        tally.exhausted, 0,
        "the pinned case must DECIDE inside STEP_CAP — an exhausted one pins nothing: {tally:?}"
    );
    assert_eq!(
        tally.max_case_steps, 178,
        "the case the branch selection was measured on now costs a different number of \
         rounds. It cost 439 under the narrowest-first selection rule and 178 under the \
         first-open one; if this moved back towards 439, the selection is back: {tally:?}"
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
        Bound::Asserted(300),
        STEP_CAP,
        // Measured 3,923 rounds.
        4_320,
        // Measured 149,265 work units.
        164_200,
        &arb_axioms(axiom),
    );
}

// ── The ∀-equivalence family ────────────────────────────────────────────────────

/// Knowledge bases checked by the ∀-equivalence property.
const FORALL_EQUIVALENCE_CASES: u32 = 600;

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
/// * `⊤ ⊑ ∀p.S` — an `rdfs:range`, the unguarded inclusion the equivalence-over-untyped-
///   restrictions ontology states;
/// * plain inclusions, type assertions and `owl:differentFrom`, so the family is a knowledge
///   base rather than a terminology.
#[test]
fn the_forall_equivalence_shape_agrees_with_the_oracle() {
    let sig = FORALL_EQUIVALENCE;
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
        FORALL_EQUIVALENCE_CASES,
        8,
        Bound::Asserted(12),
        STEP_CAP,
        // Measured 2,644 rounds over 959 case splits — the second most branch-heavy
        // property in the suite, which is the point of it.
        2_910,
        // Measured 806,279 work units.
        887_000,
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
        Bound::Asserted(11),
        STEP_CAP,
        // Measured 887 rounds over THREE case splits in 600 knowledge bases: a cyclic
        // equivalence absorbs on both sides, so what makes these cases hard is blocking
        // rather than branching, and the number that would move if blocking stopped
        // biting is this one.
        980,
        // Measured 56,207 work units.
        61_900,
        &arb_axiom_groups(group),
    );
}

// ── The co-typed family ─────────────────────────────────────────────────────────

/// Knowledge bases checked by the co-typed property.
const CO_TYPED_CASES: u32 = 300;

/// One equivalence-defined class, as the co-typed property states them.
///
/// The three bodies are the equivalence-over-untyped-restrictions ontology's own, reduced to
/// what a four-class, one-role signature can carry AND to what four of them at once stay
/// affordable at: the `∀`-restriction whose filler is an intersection, the exact cardinality,
/// and an existential. The intersection is two named conjuncts rather than the ∀-equivalence
/// shape's nested `∀`, because four nested-`∀` definitions on one node cost the internalized
/// reference encoding more than the suite's narrowed round cap allows — which would shrink
/// the encoding differential this family is also checked by. Each is a definition whose CONVERSE direction —
/// `body ⊑ A` — has an antecedent no faithful absorption can guard, so each contributes a
/// disjunction to the search rather than a guarded clause. That is the property being
/// checked: four such disjunctions, all on one node.
fn arb_co_typed_body(sig: Signature) -> BoxedStrategy<Concept> {
    let named = arb_named(sig);
    Union::new_weighted(vec![
        (
            2,
            (arb_role(sig), named.clone(), named.clone())
                .prop_map(|(outer, left, right)| {
                    Concept::All(outer, Box::new(Concept::And(vec![left, right])))
                })
                .boxed(),
        ),
        (
            4,
            (0u32..=1, arb_role(sig))
                .prop_map(|(n, counted)| {
                    Concept::And(vec![
                        Concept::Min(n, counted, Box::new(Concept::Top)),
                        Concept::Max(n, counted, Box::new(Concept::Top)),
                    ])
                })
                .boxed(),
        ),
        (
            3,
            (arb_role(sig), named)
                .prop_map(|(r, c)| Concept::Some(r, Box::new(c)))
                .boxed(),
        ),
    ])
    .boxed()
}

/// FOUR equivalence-defined classes, every one of them asserted of the SAME individual.
///
/// Not a random axiom list, deliberately. Every other property here samples axioms and lets
/// the interesting shapes co-occur by chance; co-typing four definitions on one individual by
/// chance would essentially never happen, and a family that reached its own subject matter
/// occasionally would report a population it never checked. So the SHAPE is fixed — four
/// definitions, four type assertions, one individual — and what varies is the four bodies.
fn arb_co_typed_axioms(sig: Signature) -> BoxedStrategy<Vec<Axiom>> {
    let subject = sig.individual_names()[0];
    let names = sig.concept_names().to_vec();
    let body = arb_co_typed_body(sig);
    (body.clone(), body.clone(), body.clone(), body)
        .prop_map(move |bodies| {
            let mut axioms: Vec<Axiom> = Vec::new();
            for (&name, body) in names.iter().zip([bodies.0, bodies.1, bodies.2, bodies.3]) {
                axioms.extend(equivalence(&Concept::Named(name), &body));
                axioms.push(Axiom::Type(subject, Concept::Named(name)));
            }
            axioms
        })
        .boxed()
}

/// FOUR equivalence-defined classes co-typed on ONE individual — the shape whose per-round
/// cost the round budget cannot see.
///
/// Every other family here checks the calculus over knowledge bases whose disjunctions are
/// spread across nodes. This one puts them all on one, which is the difference between a
/// search whose cost grows with the ontology and one whose cost grows with the CO-TYPING: the
/// converse direction of each of the four definitions reaches the search as a disjunction, and
/// four disjunctions on one node interleave.
///
/// What is asserted here is what every family asserts — the two calculi agree, the two TBox
/// encodings agree, an exhibited model is accepted and a bounded refutation is refuted — over
/// a population the others do not reach. The COST of that population is the second half: the
/// round and work ceilings below are the measured evidence that this shape is bounded, and
/// they sit an order of magnitude apart per case from the families whose disjunctions are
/// spread out.
#[test]
fn four_co_typed_definitions_on_one_individual_agree_with_the_oracle() {
    let sig = CO_TYPED;
    run_property(
        "co-typed definitions",
        sig,
        CO_TYPED_CASES,
        10,
        // ZERO, and asserted as an equality: every body this family generates carries a
        // quantifier or a counting concept, so `forces_unnamed_element` holds of all of them
        // and `bounded_domain` can never hold. The over-permissive direction is checked by
        // the OTHER families; what this one is for is the cost of the co-typed search and the
        // two differentials over it.
        Bound::Impossible,
        // A WIDER round narrowing than [`STEP_CAP`], and the only family that takes one.
        // The suite's cap is what 9,800 cases can afford EACH, and this family is 300
        // structurally deeper ones: four definitions internalized as eight disjunctions in
        // every node's label is what the ENCODING differential decides here, and at 350
        // rounds a quarter of its cases could not finish that side — which would quietly
        // shrink the population absorption's soundness claim is checked over. At 4,000 the
        // encoding comparison covers 290 of the 300 cases; the HYPERTABLEAU side never needs
        // it, spending at most 188 rounds on any case here, so what the wider cap buys is
        // entirely the reference encoding's ability to keep up.
        4_000,
        // Measured 5,350 rounds — the most branch-heavy family in the suite by a factor of
        // three, over 1,403 case splits in 300 knowledge bases.
        5_900,
        // Measured 12,292,228 work units, the most of any family, over a peak of 2,986,922
        // in ONE case. That per-case figure is what co-typing costs: the `wide` corpus's
        // dearest case spends 6.9 million over a search the round cap truncates, and this
        // one spends nearly half of that while DECIDING in 188 rounds.
        13_700_000,
        &arb_co_typed_axioms(sig),
    );
}

// ── The concrete-domain family ──────────────────────────────────────────────────

/// Knowledge bases checked by the concrete-domain property.
const DATA_CASES: u32 = 600;

/// A strategy over the family's data ranges, as concept leaves.
fn arb_data_range() -> BoxedStrategy<Concept> {
    prop::sample::select(vec![DR_INTEGER, DR_STRING, DR_SMALL, DR_ONE])
        .prop_map(Concept::Data)
        .boxed()
}

/// A strategy over the family's data properties.
fn arb_data_property() -> BoxedStrategy<u32> {
    prop::sample::select(DATA_PROPERTY_NAMES.to_vec()).boxed()
}

/// A strategy over the family's literals, by the term id each is interned under.
fn arb_literal() -> BoxedStrategy<u32> {
    prop::sample::select((0..LITERALS.len() as u32).collect::<Vec<u32>>()).boxed()
}

/// A strategy over the fillers a `≤n u.C` restriction counts: a data range, or `⊤`.
///
/// `⊤` is the unqualified `owl:maxCardinality`, and it is the one that reaches the
/// value-class rule on its own: a `≤n u.DR` counts only the neighbours some narrowing has
/// already labelled `DR`, while `≤n u.⊤` counts every literal a data property reaches — so
/// `"1"^^xsd:integer` and `"01"^^xsd:integer` are one element and `"1"` and `"2"` are two,
/// with nothing having stated an inequality either way.
fn arb_counted_filler() -> BoxedStrategy<Concept> {
    Union::new_weighted(vec![(3, arb_data_range()), (2, Just(Concept::Top).boxed())]).boxed()
}

/// `∀u.DR`, over a plain or a COMPLEMENTED data range — the second is what puts a
/// `¬Data` leaf on the successor's node, which is the negative half of the emptiness
/// question [`crate::owl_dl::data`] answers.
///
/// Not an `arb_` strategy: it is a plain constructor the strategies below call once they have
/// drawn a property, a range and a flag.
fn universal_range(property: u32, filler: Concept, complemented: bool) -> Concept {
    let filler = if complemented {
        Concept::Not(Box::new(filler))
    } else {
        filler
    };
    Concept::All(Role::Named(property), Box::new(filler))
}

/// Every axiom shape of the concrete-domain family that REACHES `Δ_D`.
///
/// Kept apart from the abstract shapes below because every generated knowledge base is
/// anchored with one of these ([`arb_data_axioms`]): a case built entirely from the abstract
/// shapes would state no data range, and the enumerator would then decide it — which is a
/// perfectly good abstract case and no coverage of the concrete domain at all.
fn arb_concrete_axiom(sig: Signature) -> BoxedStrategy<Axiom> {
    let individuals = sig.individual_names().to_vec();
    let named = arb_named(sig);
    Union::new_weighted(vec![
        // A literal-valued assertion. The object is an element of `Δ_D` carrying the
        // singleton range its VALUE is, which is what makes every `∀u.DR` above it a
        // question about the value the ontology stated rather than about a range's own
        // emptiness.
        (
            6,
            (
                prop::sample::select(individuals.clone()),
                arb_data_property(),
                arb_literal(),
            )
                .prop_map(|(a, p, l)| Axiom::DataAssertion(a, p, l))
                .boxed(),
        ),
        // `A ⊑ ∀u.DR` — range narrowing under a GUARD, so the narrowing reaches a literal's
        // node only where the search derived `A`.
        (
            5,
            (
                named.clone(),
                arb_data_property(),
                arb_data_range(),
                any::<bool>(),
            )
                .prop_map(|(a, p, dr, complemented)| {
                    Axiom::Gci(a, universal_range(p, dr, complemented))
                })
                .boxed(),
        ),
        // `⊤ ⊑ ∀u.DR` — `rdfs:range` over a data property, and the shape that clausifies to
        // an EDGE clause rather than to anything in a label. Its narrowing is therefore read
        // from the per-role index in [`Graph::data_clashes`](crate::owl_dl::graph) under one
        // encoding and off the node's own label under the other, which is the asymmetry this
        // family exists to check.
        (
            4,
            (arb_data_property(), arb_data_range(), any::<bool>())
                .prop_map(|(p, dr, complemented)| {
                    Axiom::Gci(Concept::Top, universal_range(p, dr, complemented))
                })
                .boxed(),
        ),
        // `A ⊑ ≥n u.DR` — counting over a data range. `≥4 u.xsd:integer[0…2]` is refuted by
        // the range's CARDINALITY, which no per-node emptiness check can see, and the same
        // restriction over `xsd:integer` is satisfiable — so the family reaches both sides of
        // `provably_fewer_than`.
        (
            4,
            (
                named.clone(),
                1u32..=4,
                arb_data_property(),
                arb_data_range(),
            )
                .prop_map(|(a, n, p, dr)| {
                    Axiom::Gci(a, Concept::Min(n, Role::Named(p), Box::new(dr)))
                })
                .boxed(),
        ),
        (
            3,
            (
                prop::sample::select(individuals.clone()),
                1u32..=4,
                arb_data_property(),
                arb_data_range(),
            )
                .prop_map(|(a, n, p, dr)| {
                    Axiom::Type(a, Concept::Min(n, Role::Named(p), Box::new(dr)))
                })
                .boxed(),
        ),
        // `a : ∃u.(DR ⊓ DR′)` — two ranges on ONE node, which is a clash exactly when the two
        // value spaces are disjoint (`xsd:integer` against `xsd:string`) and satisfiable when
        // they nest.
        (
            3,
            (
                prop::sample::select(individuals.clone()),
                arb_data_property(),
                arb_data_range(),
                arb_data_range(),
            )
                .prop_map(|(a, p, left, right)| {
                    Axiom::Type(
                        a,
                        Concept::Some(Role::Named(p), Box::new(Concept::And(vec![left, right]))),
                    )
                })
                .boxed(),
        ),
        // `a : ∃u.{"1"^^xsd:integer}` — `owl:hasValue` over a data property. The witness is a
        // node of the data domain from birth, and the `o`-rule identifies it with the
        // literal's own root.
        (
            3,
            (
                prop::sample::select(individuals.clone()),
                arb_data_property(),
                arb_literal(),
            )
                .prop_map(|(a, p, l)| {
                    Axiom::Type(
                        a,
                        Concept::Some(Role::Named(p), Box::new(Concept::nominal(vec![l]))),
                    )
                })
                .boxed(),
        ),
        // `A ⊑ ≤n u.DR` and `a : ≤n u.DR` — the FUNCTIONAL-data-property shape. Two literals
        // of one value may be counted once and two of different values may not, and nothing
        // states an inequality between them: what forces them apart is the value class, which
        // is the data domain's own answer to the absence of a unique name assumption. This is
        // also where the family branches — a `≤n` violation is a case split over which pair to
        // identify.
        (
            3,
            (
                named.clone(),
                0u32..=2,
                arb_data_property(),
                arb_counted_filler(),
            )
                .prop_map(|(a, n, p, dr)| {
                    Axiom::Gci(a, Concept::Max(n, Role::Named(p), Box::new(dr)))
                })
                .boxed(),
        ),
        (
            3,
            (
                prop::sample::select(individuals.clone()),
                0u32..=2,
                arb_data_property(),
                arb_counted_filler(),
            )
                .prop_map(|(a, n, p, dr)| {
                    Axiom::Type(a, Concept::Max(n, Role::Named(p), Box::new(dr)))
                })
                .boxed(),
        ),
        // `A ⊑ ∀u.DR ⊔ ∀u.DR′` — a DISJUNCTION of two narrowings, so which range a counting
        // question is narrowed against depends on the branch the search took. The two
        // encodings reach that disjunction by different routes, and a narrowing that survives
        // one branch and not the other is exactly the shape a per-role index can lose.
        (
            3,
            (
                named,
                arb_data_property(),
                arb_data_range(),
                arb_data_range(),
            )
                .prop_map(|(a, p, left, right)| {
                    Axiom::Gci(
                        a,
                        Concept::Or(vec![
                            universal_range(p, left, false),
                            universal_range(p, right, false),
                        ]),
                    )
                })
                .boxed(),
        ),
        // `a owl:sameAs "1"^^xsd:integer` — the identification that puts a named individual's
        // root into `Δ_D`, and with it the withdrawal of every concept the TBox asserted
        // unconditionally of a node that turns out to denote a value.
        (
            2,
            (prop::sample::select(individuals), arb_literal())
                .prop_map(|(a, l)| Axiom::SameAs(a, l))
                .boxed(),
        ),
    ])
    .boxed()
}

/// The concrete-domain family's ABSTRACT shapes: the terminology the shapes above are read
/// against.
///
/// `⊤ ⊑ A` is the unconditional consequent whose withdrawal a merge with a literal forces, and
/// `A ⊓ B ⊑ ⊥` is what gives a consequent that is NOT withdrawn something to clash against —
/// so the pair is how a lost or a kept withdrawal becomes a VERDICT rather than a label
/// difference nothing reads.
fn arb_abstract_axiom(sig: Signature) -> BoxedStrategy<Axiom> {
    let individuals = sig.individual_names().to_vec();
    let named = arb_named(sig);
    Union::new_weighted(vec![
        (
            3,
            (prop::sample::select(individuals), named.clone())
                .prop_map(|(a, c)| Axiom::Type(a, c))
                .boxed(),
        ),
        (
            2,
            named
                .clone()
                .prop_map(|a| Axiom::Gci(Concept::Top, a))
                .boxed(),
        ),
        (
            2,
            (named.clone(), named)
                .prop_map(|(a, b)| Axiom::Gci(Concept::And(vec![a, b]), Concept::Bottom))
                .boxed(),
        ),
    ])
    .boxed()
}

/// THE NARROWED COUNTING PAIR: `⊤ ⊑ ∀u.DR` beside `a : ≥n u.DR′`, with the narrowing chosen
/// from the two ranges small enough to bound the count.
///
/// Fixed as a PAIR rather than left to chance, for the reason the co-typed family fixes its
/// shape: the two halves have to meet on one node, and drawing them independently produces one
/// without the other almost every time. Measured, before this existed: not once in 600 cases —
/// and the consequence was that deleting the per-role range index of
/// [`Graph`](crate::owl_dl::graph) left the whole generated corpus passing, with only the
/// hand-written regression noticing.
///
/// It is the sharp case because the narrowing bounds the COUNT without emptying any single
/// node's constraint set: `∀u.{1}` beside `≥2 u.xsd:integer` gives every witness the perfectly
/// satisfiable label `xsd:integer ⊓ {1}`, and what is unsatisfiable is having TWO of them. A
/// narrowing that emptied the conjunction instead — `∀u.xsd:integer` beside `≥1 u.xsd:string` —
/// would close on the witness's own label, which the per-node check already reaches, so it
/// would say nothing about the index.
fn arb_narrowed_counting(sig: Signature) -> BoxedStrategy<Vec<Axiom>> {
    let individuals = sig.individual_names().to_vec();
    (
        arb_data_property(),
        2u32..=4,
        arb_data_range(),
        prop::sample::select(vec![DR_ONE, DR_SMALL]),
        prop::sample::select(individuals),
    )
        .prop_map(|(property, n, counted, narrowing, subject)| {
            vec![
                Axiom::Gci(
                    Concept::Top,
                    universal_range(property, Concept::Data(narrowing), false),
                ),
                Axiom::Type(
                    subject,
                    Concept::Min(n, Role::Named(property), Box::new(counted)),
                ),
            ]
        })
        .boxed()
}

/// One to seven axioms, ANCHORED by a group that reaches the concrete domain.
///
/// The anchor is what makes the family's silence about the enumerator total: every knowledge
/// base it generates states a data range, so [`Case::enumerable`] is false of all of them and
/// the property's [`Bound::Concrete`] can assert that as an equality rather than as a share.
/// Without it a case drawn entirely from the abstract shapes would be enumerated like any
/// other — sound, but a case of the `boolean` family under a different name.
fn arb_data_axioms(sig: Signature) -> BoxedStrategy<Vec<Axiom>> {
    let anchor = Union::new_weighted(vec![
        (
            3,
            arb_concrete_axiom(sig)
                .prop_map(|axiom| vec![axiom])
                .boxed(),
        ),
        (2, arb_narrowed_counting(sig)),
    ])
    .boxed();
    let rest = Union::new_weighted(vec![
        (7, arb_concrete_axiom(sig)),
        (3, arb_abstract_axiom(sig)),
    ])
    .boxed();
    (anchor, prop::collection::vec(rest, 0..=5))
        .prop_map(|(anchor, rest)| {
            let mut out = anchor;
            out.extend(rest);
            out
        })
        .boxed()
}

/// THE CONCRETE DOMAIN, over the differentials that do not need an enumeration.
///
/// Every other family here is checked three ways: two calculi against each other, two TBox
/// encodings against each other, and both against an enumeration of every interpretation over
/// a bounded domain. This one keeps the first two and gives up the third, because the third is
/// not available at any bound: an interpretation in this file fixes ONE finite domain `Δ` and
/// guesses a subset of it per class name, while a data range is a subset of a second, disjoint
/// domain `Δ_D` whose extension the DATATYPE MAP fixes — infinite for `xsd:integer` alone, and
/// decided by the very procedure ([`crate::owl_dl::data`] over `purrdf-xsd`) that is under
/// test here. An "oracle" that guessed `Δ_D` would either be that procedure again, which
/// checks nothing, or a different semantics, which checks the wrong thing. So
/// [`Case::enumerable`] is false of every case this family generates, [`check`] tallies each
/// as `concrete`, and the property's [`Bound::Concrete`] states the floors that bind instead.
///
/// # What is left, and why it is not nothing
///
/// The ENCODING differential is the sharp one here, because the concrete domain is exactly
/// where the two encodings of one axiom take different routes. `⊤ ⊑ ∀u.DR` — `rdfs:range` over
/// a data property — is a meta-concept in every abstract node's label under one encoding and
/// an EDGE CLAUSE under the other, and the counting rule at
/// [`Graph::data_clashes`](crate::owl_dl::graph) has to fold that narrowing into a `≥n u.DR′`
/// question from a per-role index in the second case and off the node's own label in the
/// first. A narrowing lost on one side and kept on the other is a verdict difference, and this
/// corpus is where it shows.
///
/// The CALCULUS differential covers the other half: the hypertableau reaches a range through a
/// clause head and the concept-tree tableau through its own `∀`-rule, while both share
/// [`crate::owl_dl::data`] verbatim — so what they can disagree about is WHICH constraints
/// reach a node, which is the question the two latent scope bugs on this shape were.
///
/// And both differentials are conditional on the corpus deciding things, which is why the
/// verdict floors are part of the [`Bound::Concrete`] below rather than an observation in a
/// comment: a corpus of consistent verdicts would establish that neither encoding nor either
/// calculus ever closes a branch, which is precisely what a withheld concrete-domain clash
/// looks like.
///
/// # The shapes
///
/// * `a u "1"^^xsd:integer` — a literal-valued assertion, whose object carries the singleton
///   range its value is and the value CLASS that decides which literals are one element;
/// * `A ⊑ ∀u.DR` and `⊤ ⊑ ∀u.DR` — range narrowing, guarded and unguarded, the second being
///   the edge-clause shape above (and both also generated with a COMPLEMENTED filler, so the
///   `¬Data` half of the emptiness question is reached);
/// * `A ⊑ ≥n u.DR`, `a : ≥n u.DR` — counting over a data range, refutable by cardinality;
/// * `A ⊑ ≤n u.DR`, `a : ≤n u.DR` — counting the other way, where two literals are one
///   element exactly when they denote one VALUE and nothing states an inequality either way;
/// * `A ⊑ ∀u.DR ⊔ ∀u.DR′` — a narrowing that depends on the branch, which is what gives this
///   family case splits at all;
/// * `a : ∃u.(DR ⊓ DR′)` — datatype disjointness on one node;
/// * `a : ∃u.{"1"^^xsd:integer}` — `owl:hasValue`, and with it the `o`-rule over a literal;
/// * `⊤ ⊑ A`, `A ⊓ B ⊑ ⊥` and `a owl:sameAs "1"^^xsd:integer` — the merge-with-a-literal
///   shape, where an unconditional TBox consequent has to be withdrawn from a node that turns
///   out to inhabit `Δ_D`.
#[test]
fn the_concrete_domain_shapes_agree_across_the_encodings_and_the_calculi() {
    let sig = DATA;
    run_property(
        "concrete domain",
        sig,
        DATA_CASES,
        11,
        Bound::Concrete {
            // Measured 315 consistent and 285 inconsistent verdicts over the 600 cases,
            // floored a fifth below each: the question is whether the corpus still reaches
            // both sides, not whether the split is exactly this one. Which clash each
            // concrete-domain rule contributes is pinned by name in the regressions below —
            // a corpus count cannot say WHICH branch closed, only that some did.
            consistent: 250,
            inconsistent: 225,
        },
        STEP_CAP,
        // Measured 1,053 rounds over 28 case splits — the cheapest family in the suite per
        // case, because a node of the data domain generates no successors of its own.
        1_160,
        // Measured 52,588 work units.
        57_850,
        &arb_data_axioms(sig),
    );
}

/// The signature the concrete-domain regressions below are stated over: ONE individual.
///
/// One, because the shape that needs it is the withdrawal: a knowledge base whose only
/// object-domain element is identified with a literal has no second element left for the
/// TBox's unconditional consequents to hold of, so the verdict turns on the withdrawal alone.
const DATA_HAND: Signature = Signature {
    concepts: 2,
    roles: 0,
    individuals: 1,
    max_domain: 1,
};

/// Check one hand-written concrete-domain case against every side that HAS an opinion: both
/// calculi, both TBox encodings, and the verdict derived in the case's own comment.
///
/// The oracle is not one of them, and the assertion says so rather than passing silently: a
/// case here MUST state a data range, because a concrete regression that reached no data range
/// would be pinning the abstract rules under a concrete-sounding name.
fn assert_concrete_verdict(axioms: &[Axiom], satisfiable: bool) {
    let case = Case::assemble(DATA_HAND, axioms);
    assert!(
        !case.enumerable(),
        "a concrete-domain regression must reach Δ_D:\n{}",
        case.axioms_text()
    );
    let cap = graph::Budget::for_kb(&case.kb);
    let decision = hyper::decide(&case.kb, &Assumptions::of_kb(), cap);
    let reference = tableau::decide(&case.kb, &Assumptions::of_kb(), cap);
    let internalized = Case::encoded(DATA_HAND, axioms, true);
    let other = hyper::decide(
        &internalized.kb,
        &Assumptions::of_kb(),
        graph::Budget::for_kb(&internalized.kb),
    );
    let legend = concrete_legend();
    for (side, verdict) in [
        ("the hypertableau", &decision),
        ("the concept-tree tableau", &reference),
        ("the all-meta encoding", &other),
    ] {
        assert!(
            !verdict.exhausted,
            "{side} must decide a regression this small:\n{}\n{legend}",
            case.axioms_text()
        );
        assert_eq!(
            verdict.consistent,
            satisfiable,
            "{side} disagrees with the derived verdict:\n{}\n{legend}",
            case.axioms_text()
        );
    }
}

/// The individual of [`DATA_HAND`].
fn data_subject() -> u32 {
    DATA_HAND.individual_names()[0]
}

/// `∀u.C` over the family's first data property.
fn all_values(filler: Concept) -> Concept {
    Concept::All(Role::Named(DATA_PROPERTY_NAMES[0]), Box::new(filler))
}

/// `≥n u.C` over the family's first data property.
fn at_least_values(n: u32, filler: Concept) -> Concept {
    Concept::Min(n, Role::Named(DATA_PROPERTY_NAMES[0]), Box::new(filler))
}

/// `a u <literal>` for the family's first data property.
fn values(literal: u32) -> Axiom {
    Axiom::DataAssertion(data_subject(), DATA_PROPERTY_NAMES[0], literal)
}

/// `a : ≥4 xsd:integer[0…2]`.
///
/// UNSATISFIABLE. `≥n u.DR` demands `n` PAIRWISE-DISTINCT values of `DR`, and the data domain
/// has no unique-name freedom to invent them from: `xsd:integer[0…2]` holds exactly three
/// values, so four distinct ones do not exist. Nothing about any single node's constraint set
/// is empty here — the refutation is arithmetic over the range, which is the half of
/// [`Graph::data_clashes`](crate::owl_dl::graph) a per-node emptiness check cannot reach.
#[test]
fn counting_more_values_than_a_data_range_holds_is_unsatisfiable() {
    assert_concrete_verdict(
        &[Axiom::Type(
            data_subject(),
            at_least_values(4, Concept::Data(DR_SMALL)),
        )],
        false,
    );
    // …and three of them DO exist, so the assertion above turns on the count rather than on
    // the range being unusable.
    assert_concrete_verdict(
        &[Axiom::Type(
            data_subject(),
            at_least_values(3, Concept::Data(DR_SMALL)),
        )],
        true,
    );
}

/// `⊤ ⊑ ∀u.{1}` with `a : ≥2 u.xsd:integer`.
///
/// UNSATISFIABLE, and reachable ONLY through the range axiom. `⊤ ⊑ ∀u.DR` — `rdfs:range` over
/// a data property — absorbs to the EDGE CLAUSE `u(x,y) → DR(y)` and enters no node's label at
/// all, so the narrowing it contributes to the counting question has to be read from the
/// per-role index of [`Graph`](crate::owl_dl::graph) rather than off the node's own label.
/// Every `u`-value is therefore in `{1}`, which holds one value, and two distinct ones are
/// demanded.
///
/// The satisfiable half widens the range axiom to three values and keeps everything else, so
/// what separates the two is the narrowing alone: a per-role index that silently found nothing
/// would report both as consistent, and the internalized encoding — where the same axiom IS in
/// every node's label — would then disagree with the absorbed one over the first.
#[test]
fn a_range_axiom_narrows_a_counting_question_over_a_data_property() {
    assert_concrete_verdict(
        &[
            Axiom::Gci(Concept::Top, all_values(Concept::Data(DR_ONE))),
            Axiom::Type(
                data_subject(),
                at_least_values(2, Concept::Data(DR_INTEGER)),
            ),
        ],
        false,
    );
    assert_concrete_verdict(
        &[
            Axiom::Gci(Concept::Top, all_values(Concept::Data(DR_SMALL))),
            Axiom::Type(
                data_subject(),
                at_least_values(2, Concept::Data(DR_INTEGER)),
            ),
        ],
        true,
    );
}

/// `a : ∃u.(xsd:integer ⊓ xsd:string)`.
///
/// UNSATISFIABLE. OWL 2's datatype map makes those two value spaces DISJOINT, so no element of
/// `Δ_D` is in both and the witness the existential demands cannot exist. The abstract rules
/// see two atomic leaves that are not complementary; what refutes it is the datatype map.
#[test]
fn a_value_in_two_disjoint_datatypes_is_unsatisfiable() {
    let some_values = |left: u32, right: u32| {
        Axiom::Type(
            data_subject(),
            Concept::Some(
                Role::Named(DATA_PROPERTY_NAMES[0]),
                Box::new(Concept::And(vec![
                    Concept::Data(left),
                    Concept::Data(right),
                ])),
            ),
        )
    };
    assert_concrete_verdict(&[some_values(DR_INTEGER, DR_STRING)], false);
    // Two ranges of ONE space, one inside the other: inhabited, so the case above turns on
    // disjointness rather than on a conjunction of ranges being refused on sight.
    assert_concrete_verdict(&[some_values(DR_INTEGER, DR_SMALL)], true);
}

/// `⊤ ⊑ ∀u.xsd:integer` with `a u "cat"^^xsd:string`.
///
/// UNSATISFIABLE. The range axiom's head lands on the LITERAL's own node — a node of `Δ_D`,
/// which the clause is allowed to conclude about even though no general concept inclusion may
/// fire FROM one — and there it meets the singleton range the literal's value is. An integer
/// range and `{"cat"}` have an empty intersection.
#[test]
fn a_literal_outside_its_data_propertys_range_is_unsatisfiable() {
    let range = Axiom::Gci(Concept::Top, all_values(Concept::Data(DR_INTEGER)));
    assert_concrete_verdict(&[range.clone(), values(3)], false);
    assert_concrete_verdict(&[range, values(0)], true);
}

/// `a : ≤1 u.⊤` with two literal values.
///
/// UNSATISFIABLE for `"1"^^xsd:integer` and `"2"^^xsd:integer`, SATISFIABLE for
/// `"1"^^xsd:integer` and `"01"^^xsd:integer` — the same two RDF TERMS in both readings, and
/// the difference is that the second pair denotes ONE element of `Δ_D`. Nothing states an
/// inequality in either case: what forces the first pair apart is the value class, which is
/// the datatype map's answer rather than a unique-name assumption.
#[test]
fn counting_literal_values_counts_values_rather_than_terms() {
    let functional = Axiom::Type(
        data_subject(),
        Concept::Max(
            1,
            Role::Named(DATA_PROPERTY_NAMES[0]),
            Box::new(Concept::Top),
        ),
    );
    assert_concrete_verdict(&[functional.clone(), values(0), values(2)], false);
    assert_concrete_verdict(&[functional, values(0), values(1)], true);
}

/// `⊤ ⊑ A`, `⊤ ⊑ B`, `A ⊓ B ⊑ ⊥` with `a owl:sameAs "1"^^xsd:integer`.
///
/// SATISFIABLE. The three inclusions refute any knowledge base with an element of `Δ_I` in it,
/// and the `owl:sameAs` says the only named element is a literal VALUE — an element of `Δ_D`,
/// which `owl:Thing` does not denote and which those inclusions therefore never quantified
/// over. The identification has to WITHDRAW the two unconditional consequents from the node it
/// produced, in whichever encoding they arrived: a seeded meta-concept under one, a derived
/// empty-guard clause head under the other. Keeping either would refute the knowledge base on
/// the strength of an axiom that never ranged over the element it closed. See
/// [`Graph::merge_nodes`](crate::owl_dl::graph).
///
/// The dual is the same terminology with the identification removed, which IS refuted — so
/// the case above turns on the withdrawal rather than on the inclusions being harmless.
#[test]
fn an_unconditional_consequent_is_withdrawn_from_a_node_that_denotes_a_value() {
    let class = |i: usize| Concept::Named(DATA_HAND.concept_names()[i]);
    let terminology = [
        Axiom::Gci(Concept::Top, class(0)),
        Axiom::Gci(Concept::Top, class(1)),
        Axiom::Gci(Concept::And(vec![class(0), class(1)]), Concept::Bottom),
    ];
    let mut identified = terminology.to_vec();
    identified.push(Axiom::SameAs(data_subject(), 0));
    assert_concrete_verdict(&identified, true);

    // The same terminology over an element the axioms DO quantify over. A data range still
    // reaches the knowledge base — the assertion's object is a literal — so this is a
    // concrete-domain case whose refutation is nonetheless abstract, which is what makes the
    // pair a difference of DOMAIN rather than of axiom.
    let mut abstract_only = terminology.to_vec();
    abstract_only.push(values(0));
    assert_concrete_verdict(&abstract_only, false);
}

/// WHAT THE CONCRETE-DOMAIN FAMILY'S RANGES ACTUALLY HOLD, stated as literals.
///
/// The family's clashes are arithmetic — `≥4 u.xsd:integer[0…2]` is refutable because that
/// range holds exactly three values, and a node in both `xsd:integer` and `xsd:string` is
/// refutable because those value spaces are disjoint — and the arithmetic is `purrdf-xsd`'s
/// rather than this file's. Two things therefore have to be pinned rather than assumed.
///
/// EXACTNESS first: the concrete-domain rules read `Undecided` as "no clash", so a range the
/// decision procedure could not pin down would generate cases whose clashes are silently
/// withheld — and a family reporting consistent verdicts it never had grounds to refute would
/// read exactly like a family that works. Then the CARDINALITIES, because they are what makes
/// the counting shapes above decidable at all: `xsd:integer` bounds no counting question and
/// the other three each bound one, which is why the generator draws from all four.
#[test]
fn the_concrete_domains_arithmetic_is_pinned() {
    use purrdf_xsd::range::{Cardinality, Satisfiability, cardinality, is_exactly_decided};

    let ranges = data_ranges();
    for (index, range) in ranges.iter().enumerate() {
        assert!(
            is_exactly_decided(range),
            "Data({index}) is not exactly decided, so a clash over it would be WITHHELD rather \
             than derived: {range:?}"
        );
        assert_eq!(
            purrdf_xsd::range::satisfiability(range),
            Satisfiability::Inhabited,
            "Data({index}) is empty, which would refute every case that mentions it: {range:?}"
        );
    }
    assert_eq!(
        cardinality(&ranges[DR_INTEGER as usize]),
        Cardinality::Unbounded,
        "xsd:integer bounds no counting question"
    );
    assert_eq!(
        cardinality(&ranges[DR_STRING as usize]),
        Cardinality::Unbounded,
        "xsd:string bounds no counting question either"
    );
    assert_eq!(
        cardinality(&ranges[DR_SMALL as usize]),
        Cardinality::Exactly(3),
        "xsd:integer[0…2] is what makes `≥4 u.DR` refutable by counting"
    );
    assert_eq!(
        cardinality(&ranges[DR_ONE as usize]),
        Cardinality::Exactly(1),
        "{{1}} is the narrowest a `∀u.DR` can narrow a counting question to"
    );
    // The disjointness shape: two value spaces the datatype map keeps apart, so a node in
    // both denotes no value at all.
    assert_eq!(
        purrdf_xsd::range::satisfiability(&DataRange::And(vec![
            ranges[DR_INTEGER as usize].clone(),
            ranges[DR_STRING as usize].clone(),
        ])),
        Satisfiability::Empty,
        "an integer is not a string, and that is the family's disjointness clash"
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
    + FORALL_EQUIVALENCE_CASES
    + CYCLE_CASES
    + CO_TYPED_CASES
    + DATA_CASES;

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
    assert_eq!(FORALL_EQUIVALENCE.search_space(), 65_568);
    assert_eq!(CYCLE.search_space(), 295_944);
    assert_eq!(CO_TYPED.search_space(), 8_224);
    assert_eq!(HAND.search_space(), 65_552);
    // [`DATA`] is deliberately absent: its knowledge bases are never enumerated, because a
    // data range is a subset of a second domain no interpretation here represents (see
    // [`Case::enumerable`]). Stating a search space for it would put a number in this table
    // that nothing spends. What that family pins instead is the enumerator's TOTAL silence and
    // its two verdict floors — see [`Bound::Concrete`].
    assert_eq!(TOTAL_CASES, 9800, "generated knowledge bases per run");
}

// ── Hand-written regressions ───────────────────────────────────────────────────

/// Check one hand-written case against ALL THREE sides: the oracle must agree with the verdict
/// derived in the case's own comment, and so must both decision cores. A regression that only
/// compared two implementations could be wrong twice over.
fn assert_verdict(axioms: &[Axiom], satisfiable: bool) {
    let case = Case::assemble(HAND, axioms);
    let cap = graph::Budget::for_kb(&case.kb);
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

// ── Proof-term corroboration ───────────────────────────────────────────────────

/// Put one hand-written case's PROOF TERM in front of the two independent resources this
/// module already is.
///
/// A proof-term checker has no external oracle: no W3C case carries a proof manifest, so its
/// adversary is the tamper-negatives in [`crate::owl_dl::proof`]. What it CAN be corroborated
/// against is the two references here, and they answer different halves:
///
/// * the BOUNDED-DOMAIN ENUMERATOR exhibits a model or says nothing. It can never rule one out,
///   so it is used in the one direction it is sound in — if it exhibits a model, the verdict
///   must be `consistent`, and the recorded completion must then MODEL CHECK. A completion
///   checker that rejected genuine completions would surface here as a proof failing to check
///   for a knowledge base the enumerator has a model of;
/// * the CONCEPT-TREE TABLEAU decides the same fragment by a different rule set, so it is what
///   says the verdict being proved is the right verdict before the proof term is looked at.
///
/// Neither is a completeness oracle and neither is treated as one.
fn assert_proof_corroborates(axioms: &[Axiom]) {
    use crate::owl_dl::proof::{DlProofContext, ProofAnswer, prove_consistency_of_kb};

    let case = Case::assemble(HAND, axioms);
    let cap = graph::Budget::for_kb(&case.kb);
    let decision = hyper::decide(&case.kb, &Assumptions::of_kb(), cap);
    let reference = tableau::decide(&case.kb, &Assumptions::of_kb(), cap);
    assert!(
        !decision.exhausted && !reference.exhausted,
        "a corroborated case must be decidable inside the step cap:\n{}",
        case.axioms_text()
    );
    assert_eq!(
        decision.consistent,
        reference.consistent,
        "the two calculi must agree before a proof term of either means anything:\n{}",
        case.axioms_text()
    );
    let (answer, proof) = prove_consistency_of_kb(&case.kb);
    // A SECOND knowledge base, assembled from the axioms again: the checking context must not
    // share the one the proof was produced from, exactly as `DlProofContext::of_ontology`
    // rebuilds from the consumer's own dataset.
    let ctx = DlProofContext::of_kb(Case::assemble(HAND, axioms).kb);
    let model = case.enumerable().then(|| case.smallest_model()).flatten();
    match answer {
        ProofAnswer::Consistent => {
            let replay = proof.replay_completion(&ctx).unwrap_or_else(|error| {
                panic!(
                    "the recorded completion of a knowledge base both calculi call consistent \
                     must model check: {error}\n{}",
                    case.axioms_text()
                )
            });
            assert!(
                replay.nodes() > 0,
                "a completion has nodes: {replay:?}\n{}",
                case.axioms_text()
            );
            assert_eq!(
                replay.clauses(),
                ctx.clause_count(),
                "the check budget must reach every clause of a case this small: {replay:?}\n{}",
                case.axioms_text()
            );
        }
        ProofAnswer::Inconsistent => {
            assert!(
                model.is_none(),
                "the hypertableau refuted a knowledge base the ORACLE exhibits a model of, so \
                 the refutation is unsound whatever its proof term says:\n{}",
                case.axioms_text()
            );
            let replay = proof.replay_refutation(&ctx).unwrap_or_else(|error| {
                panic!(
                    "the recorded refutation must walk: {error}\n{}",
                    case.axioms_text()
                )
            });
            assert!(
                replay.is_closed(),
                "every alternative of every branch point must reach a replayed closure: \
                 {replay:?}\n{}",
                case.axioms_text()
            );
        }
        ProofAnswer::Undecided => panic!(
            "a case both calculi decide must not produce an undecided proof:\n{}",
            case.axioms_text()
        ),
    }
    if model.is_some() {
        assert!(
            decision.consistent,
            "the oracle exhibits a model, so the verdict the proof is bound to must be \
             consistent:\n{}",
            case.axioms_text()
        );
    }
}

/// Corroborate a proof term for each shape this stage records, over the two independent
/// references above.
///
/// The families are chosen for what they make the RECORDER do, not for what they make the
/// calculus do: a case split whose alternatives all close (the refutation TREE), a case split
/// one alternative of which survives (a completion recorded BELOW a branch point, with `Open`
/// outcomes beside closed ones), a chain that terminates only by blocking (blocking witnesses),
/// an at-most bound that closes through the `≤`-rule's pairwise merges, an `o`-clause case
/// split, and two leaves with no case split at all.
#[test]
fn recorded_proof_terms_corroborate_against_both_references() {
    let r = || Role::Named(role(0));
    for axioms in [
        // A consistent subclass chain: one completion, no case split.
        vec![
            Axiom::Gci(class(0), class(1)),
            Axiom::Type(individual(0), class(0)),
        ],
        // A refutation with no case split: one leaf.
        vec![
            Axiom::Gci(Concept::And(vec![class(0), class(1)]), Concept::Bottom),
            Axiom::Type(individual(0), class(0)),
            Axiom::Type(individual(0), class(1)),
        ],
        // A refutation THROUGH a case split, both alternatives closing on a clause instance.
        vec![
            Axiom::Type(individual(0), Concept::Or(vec![class(0), class(1)])),
            Axiom::Gci(class(0), Concept::Bottom),
            Axiom::Gci(class(1), Concept::Bottom),
        ],
        // A case split ONE alternative of which survives: the completion is recorded below a
        // branch point, so the branch tree carries `Open` outcomes beside closed ones.
        vec![
            Axiom::Type(individual(0), Concept::Or(vec![class(0), class(1)])),
            Axiom::Gci(class(0), Concept::Bottom),
        ],
        // A chain that terminates only by BLOCKING.
        vec![Axiom::Gci(
            Concept::Top,
            Concept::Some(r(), Box::new(Concept::Top)),
        )],
        // Counting: `≥2 r.⊤` against `≤1 r.⊤`, refuted through the `≤`-rule's pairwise merges.
        vec![
            Axiom::Type(individual(0), Concept::Min(2, r(), Box::new(Concept::Top))),
            Axiom::Type(individual(0), Concept::Max(1, r(), Box::new(Concept::Top))),
        ],
        // Counting that SUCCEEDS, so the completion carries a `≥`-rule's pairwise-distinct
        // witnesses and the checker's own distinctness search has something to decide.
        vec![Axiom::Type(
            individual(0),
            Concept::Min(2, r(), Box::new(Concept::Top)),
        )],
        // The `o`-clause as a case split: `d : {a, b}` branches over the two members.
        vec![Axiom::Type(individual(3), nominal(&[0, 1]))],
        // An asymmetric role's own pair: a refutation whose body instance is ASSERTED.
        vec![
            Axiom::Asymmetric(role(0)),
            Axiom::RoleAssertion(individual(0), role(0), individual(1)),
            Axiom::RoleAssertion(individual(1), role(0), individual(0)),
        ],
        // Inverse roles and a universal, so the completion's neighbour closure — which the
        // checker recomputes from the caller's own role axioms — is not the identity.
        vec![
            Axiom::InverseOf(role(0), role(1)),
            Axiom::RoleAssertion(individual(0), role(0), individual(1)),
            Axiom::Gci(
                Concept::Top,
                Concept::All(Role::Named(role(1)), Box::new(class(0))),
            ),
        ],
        // A TRANSITIVE role, so the checker's own transitive closure of the recorded edges is
        // load-bearing rather than decoration.
        vec![
            Axiom::Transitive(role(0)),
            Axiom::RoleAssertion(individual(0), role(0), individual(1)),
            Axiom::RoleAssertion(individual(1), role(0), individual(2)),
            Axiom::Gci(
                Concept::Top,
                Concept::All(Role::Named(role(0)), Box::new(class(0))),
            ),
        ],
    ] {
        assert_proof_corroborates(&axioms);
    }
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
/// exhibit one, as `(axioms, satisfiable, has a model inside the oracle's bound)`.
///
/// These shapes were written as a deliberate hunt for a knowledge base separating pairwise
/// blocking from label-only blocking (see the blocking notes in [`crate::owl_dl::hyper`]).
/// None separated, and they are kept as a permanent fixture so that the differential's reach
/// over this corner is a decision somebody made rather than an accident of the generators —
/// which is why the list is a function: two tests read it, one deciding each chain by both
/// CALCULI and one deciding it under both BLOCKING conditions.
fn inverse_universal_chains() -> Vec<(Vec<Axiom>, bool, bool)> {
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
    cases
        .into_iter()
        .map(|(extra, satisfiable, bounded_model)| {
            let mut axioms = vec![
                Axiom::Type(i0, and(named(x), named(a))),
                Axiom::Gci(named(x), some(r, named(x))),
            ];
            axioms.extend(extra);
            (axioms, satisfiable, bounded_model)
        })
        .collect()
}

/// Both calculi decide every chain above, agree, and reach the verdict its own comment
/// derives; a satisfiable one whose loop model fits the oracle's bound is confirmed by the
/// enumeration too.
#[test]
fn inverse_universal_chains_decide_identically_in_both_cores() {
    for (k, (axioms, satisfiable, bounded_model)) in
        inverse_universal_chains().into_iter().enumerate()
    {
        let case = Case::assemble(HAND, &axioms);
        let cap = graph::Budget::for_kb(&case.kb);
        let h = hyper::decide(&case.kb, &Assumptions::of_kb(), cap);
        let t = tableau::decide(&case.kb, &Assumptions::of_kb(), cap);
        assert!(!h.exhausted && !t.exhausted, "case {k} must decide");
        assert_eq!(h.consistent, t.consistent, "case {k}: the cores diverge");
        assert_eq!(h.consistent, satisfiable, "case {k}: wrong verdict");
        if bounded_model {
            assert!(
                case.smallest_model().is_some(),
                "case {k}: this satisfiable chain has a loop model inside the bound"
            );
        }
    }
}

/// …AND EVERY ONE OF THEM DECIDES THE SAME WAY UNDER LABEL-ONLY BLOCKING.
///
/// This is the hand-targeted half of the blocking claim, beside the corpus-wide half
/// [`blocking_differential`] runs: these chains were written as a deliberate hunt for a
/// knowledge base separating the pairwise condition from label-only blocking, so the family is
/// worth nothing as evidence unless the mutation is actually applied to it. It is applied
/// here, case by case, and the verdict must be the one the shipped condition reached.
#[test]
fn label_only_blocking_decides_the_inverse_universal_chains_identically() {
    for (k, (axioms, satisfiable, _)) in inverse_universal_chains().into_iter().enumerate() {
        let mut case = Case::assemble(HAND, &axioms);
        case.kb.label_only_blocking = true;
        let cap = graph::Budget::for_kb(&case.kb);
        let mutated = hyper::decide(&case.kb, &Assumptions::of_kb(), cap);
        assert!(
            !mutated.exhausted,
            "case {k} must decide under label-only blocking too"
        );
        assert_eq!(
            mutated.consistent, satisfiable,
            "case {k}: label-only blocking reaches a different verdict, which is the \
             separating knowledge base the `hyper` module docs say is not known to exist"
        );
    }
}

/// THE MUTATION IS OBSERVED: a knowledge base label-only blocking builds a strictly smaller
/// completion graph for.
///
/// Every assertion above is about a VERDICT that does not change, and an assertion of that
/// shape has a failure mode: a switch that is never read reaches the same verdict too, and
/// the whole blocking differential would then be the hypertableau agreeing with itself over
/// 9,800 knowledge bases. So one case pins the other direction — that the two conditions
/// really are two conditions.
///
/// `A ⊑ ∃r.B`, `A ⊑ ∃s.B` and `B ⊑ ∃r.B` over an `A`-individual give the root two successors
/// with the SAME label `{B}` reached by DIFFERENT roles. Label-only blocking blocks the second
/// against the first at once and expands neither chain; the pairwise condition separates them
/// on the incoming edge, expands both, and stops one level further down where the
/// predecessor labels finally agree. The verdict is the same — the knowledge base is plainly
/// consistent — and the graph is not.
#[test]
fn label_only_blocking_builds_a_smaller_graph_than_the_pairwise_condition() {
    let a_class = HAND.concept_names()[0];
    let b_class = HAND.concept_names()[1];
    let some =
        |role: u32, filler: u32| Concept::Some(Role::Named(role), Box::new(Concept::Named(filler)));
    let axioms = [
        Axiom::Type(HAND.individual_names()[0], Concept::Named(a_class)),
        Axiom::Gci(Concept::Named(a_class), some(HAND.role_names()[0], b_class)),
        Axiom::Gci(Concept::Named(a_class), some(HAND.role_names()[1], b_class)),
        Axiom::Gci(Concept::Named(b_class), some(HAND.role_names()[0], b_class)),
    ];
    let mut case = Case::assemble(HAND, &axioms);
    let cap = graph::Budget::for_kb(&case.kb);
    let pairwise = hyper::decide(&case.kb, &Assumptions::of_kb(), cap);
    case.kb.label_only_blocking = true;
    let label_only = hyper::decide(&case.kb, &Assumptions::of_kb(), cap);
    case.kb.label_only_blocking = false;
    assert!(
        pairwise.consistent && label_only.consistent,
        "both conditions decide this knowledge base consistent"
    );
    assert!(
        label_only.peak_nodes < pairwise.peak_nodes,
        "label-only blocking must build a SMALLER graph here ({} nodes) than the pairwise \
         condition ({} nodes) — if the two are equal the mutation is not being read, and \
         every blocking comparison in this file is the calculus agreeing with itself",
        label_only.peak_nodes,
        pairwise.peak_nodes
    );
}
