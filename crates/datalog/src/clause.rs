// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The DL-clause IR: the crate's ONE rule representation.
//!
//! # The shape
//!
//! Every rule this crate can hold is a **DL-clause**
//!
//! ```text
//! U₁ ∧ … ∧ Uₙ  →  ∃ȳ. (C₁ ∨ … ∨ Cₘ)        where each Cᵢ = V₁ ∧ … ∧ V_k, k ≥ 1
//! ```
//!
//! — a conjunction of body literals implying an existentially closed disjunction of
//! **conjunctions** of head atoms. One shape, five head forms:
//!
//! | form | shape | what consumes it IN THIS WORKSPACE |
//! |---|---|---|
//! | [`Atomic`](HeadForm::Atomic) | `m = 1`, `C₁` is one atom, `ȳ = ∅` | this crate's semi-naive evaluator ([`crate::seminaive`]) |
//! | [`Existential`](HeadForm::Existential) | `ȳ ≠ ∅` | [`crate::chase`], a restricted chase with frontier-addressed Skolem witnesses |
//! | [`Conjunctive`](HeadForm::Conjunctive) | `m = 1`, `C₁` is several atoms, `ȳ = ∅` | [`crate::chase`], which asserts a conjunction atomically |
//! | [`Inconsistency`](HeadForm::Inconsistency) | `m = 0` (the head is `false`) | a hard error carrying a witness |
//! | [`Disjunctive`](HeadForm::Disjunctive) | `m > 1`, `ȳ = ∅` | **refused by name here**; case-split by `purrdf-entail`'s `SHOIQ(D)` hypertableau |
//!
//! The last row is the load-bearing one. A disjunctive head needs a case split, and no
//! evaluator in this crate performs one, so [`crate::chase`] returns
//! [`ChaseError::DisjunctiveHead`](crate::chase::ChaseError::DisjunctiveHead) rather than
//! picking a disjunct — which would assert something the program does not entail. The form
//! is CLASSIFIED here so that refusal can name it, which is the difference between a
//! rejected input and a silently dropped one.
//!
//! The case split itself has a consumer, and it is named: `purrdf-entail`'s OWL-Direct
//! decision core is a HYPERTABLEAU over DL-clauses, and [`HeadForm`] — this taxonomy, this
//! type — is what it dispatches on. Its own clauses carry two atoms no arity-4 quad can be
//! (`≥n r.C(x)`, whose number and filler are part of the atom, and the equality `x ≈ y`,
//! whose assertion merges two nodes of a completion graph), so its ATOMS are concept ids over
//! graph nodes rather than [`ClauseAtom`]s — encoding either atom as a triple would mean
//! minting a predicate IRI, and PurRDF mints no vocabulary. What is shared is the
//! classification: [`Disjunctive`](HeadForm::Disjunctive) is exactly the form that calculus
//! branches over, [`Inconsistency`](HeadForm::Inconsistency) is exactly the form that closes
//! a branch, and the precedence documented on [`HeadForm`] is the precedence it reads. So
//! this row records a deliberate boundary of THIS crate's evaluators — a case split is not a
//! least fixpoint — rather than a gap waiting on a component.
//!
//! # Every atom is an arity-4 quad, and the predicate is DATA
//!
//! A [`ClauseAtom`] is `triple(?s, ?p, ?o, ?g)`: four ordinary [`ClauseTerm`] positions,
//! every one of which may be a variable. The predicate is **not** a relation symbol here —
//! it is a term carried in the second position, exactly like the subject and the object.
//!
//! That is not generality kept for symmetry; it is the difference between expressing OWL 2
//! RL and not expressing it. `prp-dom` is
//!
//! ```text
//! T(?p, rdfs:domain, ?c) ∧ T(?x, ?p, ?y) → T(?x, rdf:type, ?c)
//! ```
//!
//! and `?p` stands in PREDICATE position of the second body atom. An IR that addressed a
//! relation *by* its predicate could not write that atom at all, because a relation symbol
//! can never be a variable — and roughly a quarter of the OWL 2 RL rule set (`prp-rng`,
//! `prp-fp`, `prp-ifp`, `prp-irp`, `prp-symp`, `prp-asyp`, `prp-trp`, `prp-spo1`,
//! `prp-spo2`, `prp-eqp1`, `prp-eqp2`, `prp-pdw`, `prp-inv1`, `prp-inv2`, `prp-npa1`,
//! `prp-npa2` and several `scm-*`) quantifies over exactly that position.
//!
//! # The graph position, and how the default graph is denoted
//!
//! The fourth position is the graph the atom is asserted in, and it is a `ClauseTerm` like
//! any other: a constant graph name, or a **variable**, which is what makes a per-graph
//! rule expressible (`T(?s, ?p, ?o, ?g) → T(?s, rdf:type, ?c, ?g)` reasons inside each
//! graph and writes its conclusion back into the graph it came from).
//!
//! PurRDF mints no vocabulary, so it does not mint a name for the default graph either.
//! RDF says the default graph HAS no name, and that is what
//! [`ClauseTerm::DefaultGraph`] denotes: a distinct term kind whose lexical surface is the
//! EMPTY surface — "no name" stated as no name, rather than as a fabricated IRI a caller
//! would then have to agree with. It is legal in the graph position only;
//! [`ClauseAtom::quad`] refuses it anywhere else.
//!
//! One mapping contract for consumers arriving from a WORLD-scoped encoding (the sister
//! project's evaluator uses the same arity-4 shape with its fourth position denoting a
//! *world* under its own semantics): the fourth position here is the **RDF graph name**,
//! with the entailment layer's documented dataset semantics — each named graph closes
//! against the union of itself and the default graph. A port states its world→graph
//! mapping explicitly (which world becomes the default graph, which become named graphs)
//! or flattens to the default graph before closing; leaving the mapping implicit silently
//! changes which premises can meet which.
//!
//! An atom that does not mention a graph — the [`ClauseAtom::positive`] /
//! [`ClauseAtom::negated`] convenience, which also fixes the predicate to a constant IRI —
//! is an atom **in the default graph**. It is not a wildcard and not an implicit variable:
//! a graph-less program is a program about one graph, which is exactly what a caller who
//! never mentions graphs means, and it keeps such a program's meaning identical to what it
//! was before the position existed.
//!
//! # Why a disjunct is a CONJUNCTION and not a single atom
//!
//! The most common existential axiom in any description logic — `A ⊑ ∃r.C` — lowers to
//! `A(x) → ∃y. (r(x, y) ∧ C(y))`: two head atoms sharing one Skolem witness `y`. A head
//! whose disjuncts were single atoms cannot express it at all, because splitting it into
//! `A(x) → ∃y. r(x, y)` and `A(x) → ∃y. C(y)` mints two UNRELATED witnesses and so is a
//! strictly weaker formula. The nesting level is therefore not a generalisation kept for
//! symmetry; it is the level at which a witness's scope is expressed, and a restricted
//! chase over this IR needs `∃y. p(x, y) ∧ D(y)` to be ONE rule.
//!
//! # Why all five are representable now
//!
//! Only the atomic form has evaluation semantics in this crate. The other four are
//! nonetheless *first-class values*: a Datalog-only IR that had to grow an existential
//! quantifier or a head disjunction later would force every consumer — planner, evaluator,
//! plan cache, content digest — to be rewritten around the new shape. Representing the
//! whole target now costs one enum and three validation rules; retrofitting it costs a
//! redesign.
//!
//! The evaluator does NOT silently ignore the other four and does not accept them as if
//! they were atomic. [`Parsed::new`](crate::plan::Parsed::new) — the sole entrance to the
//! `Parsed → Stratified → Planned → Executable` pipeline, and therefore the sole route to
//! an executable program — refuses them with [`NonDatalogClause`], and
//! [`compile`](crate::seminaive::compile) reports that refusal as a named error. That
//! refusal is the correct and permanent behaviour of a Datalog evaluator handed a
//! non-Datalog clause, not a marker standing in for one.
//!
//! # Negation is a body-only property
//!
//! [`ClauseAtom`] carries a negation flag because a body literal may be negated
//! (negation as failure). A HEAD atom may not: a negated head atom is not a rule, and
//! [`HeadDisjunct::new`] refuses one, so the head's element type carries that guarantee
//! rather than restating it at every consumer. Keeping one atom type — rather than a head
//! atom type and a separate body-literal type — means a rewritten atom is the same value
//! wherever it appears, and the head/body asymmetry is enforced once, at construction,
//! instead of by two nearly-identical types kept in sync by hand.
//!
//! # Order is authored order, everywhere
//!
//! Every sequence here is a `Vec` in authored order, and every one of them is observable:
//!
//! - **clause order** — a [`Derivation`](crate::seminaive::Derivation) names its producing
//!   clause by authored index, and the round's winner tiebreak uses that index;
//! - **body order** — a derivation's sources are reported in authored body order, and the
//!   planner's sideways-information-passing order breaks ties on authored position;
//! - **head-disjunct order** — the case split a disjunctive head licenses is a
//!   deterministic branch order, which can only be the authored one;
//! - **conjunct order** — the atoms within one disjunct are asserted in authored order,
//!   and they are already observable at plan time: [`RulePlan`](crate::plan::RulePlan)
//!   assigns variable slots in first-occurrence order over the body and then the head
//!   atoms, so permuting a disjunct's conjuncts permutes the binding frame;
//! - **existential order** — a Skolem witness is addressed by the frontier and by the
//!   quantifier's position in `ȳ`.
//!
//! Nothing in this IR is a set whose order is unobservable, so
//! [`canonical_rule_hash`](crate::cache::canonical_rule_hash) is order-sensitive in all
//! five respects. See that function for the proof obligations that go with it.

use std::collections::BTreeSet;
use std::fmt;

/// One argument position of a clause atom.
///
/// A constant carries the term's **lexical surface**, the same identity the relation
/// store interns on ([`crate::store::TermInterner`]), so a planned constant is compared
/// against stored data without a second rendering convention. An IRI is kept distinct
/// from a literal because only an IRI is bracketed when rendered.
///
/// The variants are ordered variable, IRI, literal, default graph, and the derived [`Ord`]
/// follows that declaration order.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ClauseTerm {
    /// A clause variable, named as authored (the name is a plan-time key only).
    Var(String),
    /// A constant IRI, held UNBRACKETED; its surface is `<iri>`.
    Iri(String),
    /// A constant literal, held as its already-rendered lexical surface.
    Literal(String),
    /// The default graph — the graph that HAS no name.
    ///
    /// Legal in an atom's graph position only ([`ClauseAtom::quad`] refuses it elsewhere).
    /// Its lexical surface is the EMPTY surface, which is what
    /// [`RelationStore::DEFAULT_GRAPH`](crate::store::RelationStore::DEFAULT_GRAPH) keys
    /// the default partition by: PurRDF mints no vocabulary, so it does not mint a name
    /// for the graph RDF says has none. No IRI surface (`<…>`) and no literal surface
    /// (`"…"`) is empty, so the denotation cannot collide with a caller's term.
    DefaultGraph,
}

impl ClauseTerm {
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

    /// The default graph — see [`ClauseTerm::DefaultGraph`].
    pub fn default_graph() -> Self {
        Self::DefaultGraph
    }

    /// The variable name, if this is a variable.
    pub fn variable(&self) -> Option<&str> {
        match self {
            Self::Var(name) => Some(name),
            Self::Iri(_) | Self::Literal(_) | Self::DefaultGraph => None,
        }
    }

    /// Whether this term is a variable.
    pub fn is_var(&self) -> bool {
        matches!(self, Self::Var(_))
    }

    /// Whether this term is the default graph.
    pub fn is_default_graph(&self) -> bool {
        matches!(self, Self::DefaultGraph)
    }

    /// The unbracketed IRI, if this term is a constant IRI.
    pub fn iri_value(&self) -> Option<&str> {
        match self {
            Self::Iri(iri) => Some(iri),
            Self::Var(_) | Self::Literal(_) | Self::DefaultGraph => None,
        }
    }

    /// The lexical surface of a CONSTANT term — the exact bytes the store interns — or
    /// `None` for a variable, whose surface is a runtime binding rather than a plan-time
    /// property.
    ///
    /// This is the single rendering convention: an IRI is bracketed, a literal is already
    /// its own surface, and the default graph is the empty surface. An executor grounding
    /// a head, addressing a relation partition or probing a negated atom renders through
    /// here, so clause constants and stored data are always compared as the same bytes —
    /// which is precisely what lets one variable bind a predicate in one atom and a
    /// subject in another.
    pub fn surface(&self) -> Option<String> {
        match self {
            Self::Iri(iri) => Some(format!("<{iri}>")),
            Self::Literal(surface) => Some(surface.clone()),
            Self::DefaultGraph => Some(String::new()),
            Self::Var(_) => None,
        }
    }
}

/// One arity-4 atom `triple(subject, predicate, object, graph)`, optionally negated.
///
/// All four positions are ordinary [`ClauseTerm`]s: the predicate is carried as DATA, not
/// as the relation symbol, so a variable predicate — the shape OWL 2 RL's property
/// meta-rules need — is an ordinary variable the join binds. See the [module docs](self)
/// for why that is load-bearing and for how the graph position and the default graph are
/// denoted.
///
/// The negation flag is meaningful in a clause BODY only: it marks a negation-as-failure
/// filter, never a join driver. [`HeadDisjunct::new`] refuses a negated atom, so an atom
/// reached through a clause head always has `is_negated() == false`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClauseAtom {
    /// The subject argument.
    subject: ClauseTerm,
    /// The predicate argument — a term, which may be a variable.
    predicate: ClauseTerm,
    /// The object argument.
    object: ClauseTerm,
    /// The graph argument — a term, which may be a variable or the default graph.
    graph: ClauseTerm,
    /// Whether the atom is negated (a body-only negation-as-failure filter).
    negated: bool,
}

impl ClauseAtom {
    /// A positive quad atom over four explicit terms — the general constructor.
    ///
    /// # Panics
    ///
    /// Panics if [`ClauseTerm::DefaultGraph`] appears in the subject, predicate or object
    /// position. It denotes the graph that has no name, so it is meaningful in the graph
    /// position alone; admitting it elsewhere would give the empty surface a second,
    /// unrelated meaning. This is a construction bug, never a data state.
    pub fn quad(
        subject: ClauseTerm,
        predicate: ClauseTerm,
        object: ClauseTerm,
        graph: ClauseTerm,
    ) -> Self {
        assert!(
            !subject.is_default_graph()
                && !predicate.is_default_graph()
                && !object.is_default_graph(),
            "the default graph is a graph name, so it is legal in the graph position only"
        );
        Self {
            subject,
            predicate,
            object,
            graph,
            negated: false,
        }
    }

    /// A negated quad atom (a negation-as-failure filter).
    ///
    /// # Panics
    ///
    /// See [`Self::quad`].
    pub fn negated_quad(
        subject: ClauseTerm,
        predicate: ClauseTerm,
        object: ClauseTerm,
        graph: ClauseTerm,
    ) -> Self {
        Self {
            negated: true,
            ..Self::quad(subject, predicate, object, graph)
        }
    }

    /// A positive atom with a CONSTANT predicate IRI, in the DEFAULT GRAPH — the
    /// overwhelmingly common shape, kept short because most rules are written that way.
    ///
    /// Equivalent to `quad(subject, ClauseTerm::iri(predicate), object,
    /// ClauseTerm::DefaultGraph)`. Not mentioning a graph means the default graph, not
    /// "any graph"; see the [module docs](self).
    pub fn positive(subject: ClauseTerm, predicate: impl Into<String>, object: ClauseTerm) -> Self {
        Self::quad(
            subject,
            ClauseTerm::Iri(predicate.into()),
            object,
            ClauseTerm::DefaultGraph,
        )
    }

    /// The negated sibling of [`Self::positive`].
    pub fn negated(subject: ClauseTerm, predicate: impl Into<String>, object: ClauseTerm) -> Self {
        Self {
            negated: true,
            ..Self::positive(subject, predicate, object)
        }
    }

    /// The subject argument.
    pub fn subject(&self) -> &ClauseTerm {
        &self.subject
    }

    /// The object argument.
    pub fn object(&self) -> &ClauseTerm {
        &self.object
    }

    /// The predicate argument, as a term — which may be a variable.
    pub fn predicate(&self) -> &ClauseTerm {
        &self.predicate
    }

    /// The graph argument, as a term — which may be a variable or the default graph.
    pub fn graph(&self) -> &ClauseTerm {
        &self.graph
    }

    /// The unbracketed predicate IRI, or `None` when the predicate is not a constant IRI.
    ///
    /// A diagnostic or a rule inventory that wants to *name* an atom's predicate asks
    /// here, and gets `None` exactly when there is no single name to give.
    pub fn predicate_iri(&self) -> Option<&str> {
        self.predicate.iri_value()
    }

    /// Whether the atom is negated.
    pub fn is_negated(&self) -> bool {
        self.negated
    }

    /// This atom's four positions in order: subject, predicate, object, graph.
    ///
    /// One accessor for the arity-generic passes (variable collection, adornment, slot
    /// assignment) so none of them can quietly visit three positions and skip the fourth.
    pub fn terms(&self) -> [&ClauseTerm; 4] {
        [&self.subject, &self.predicate, &self.object, &self.graph]
    }

    /// Record every variable of this atom into `into`.
    fn collect_variables(&self, into: &mut BTreeSet<String>) {
        for term in self.terms() {
            if let ClauseTerm::Var(name) = term {
                into.insert(name.clone());
            }
        }
    }
}

/// One disjunct `Cᵢ` of a clause head: a NON-EMPTY conjunction of positive atoms.
///
/// # Why the head's element type is not a bare `Vec<ClauseAtom>`
///
/// Two of the head's well-formedness rules are properties of a single disjunct — it has
/// at least one atom, and none of its atoms is negated — and neither depends on anything
/// else in the clause. Enforcing them in this constructor makes an ill-formed disjunct
/// *unrepresentable*, so every consumer that walks a head (the digest, the stratifier,
/// the planner, a future chase) may rely on `atoms()` being non-empty and positive
/// without re-deriving that from [`DlClause`]'s asserts. A bare nested `Vec` would leave
/// both rules restated at each site, or checked once and assumed everywhere else.
///
/// The distinction this type enforces is load-bearing, because the two empties mean
/// opposite things: an EMPTY HEAD (`m = 0`) is the well-formed inconsistency clause
/// `body → false`, while an empty DISJUNCT is the empty conjunction — `true` — which
/// would make the whole head trivially satisfiable and the clause silently inert.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeadDisjunct {
    /// The conjuncts `V₁ ∧ … ∧ V_k`, in authored order; never empty, never negated.
    atoms: Vec<ClauseAtom>,
}

impl HeadDisjunct {
    /// A disjunct from its conjuncts, in authored order.
    ///
    /// # Panics
    ///
    /// Panics if the disjunct is malformed, which is a construction bug rather than a
    /// data state:
    ///
    /// * `atoms` is empty — the empty conjunction is `true`, so an empty disjunct would
    ///   make the whole head trivially satisfiable; the empty HEAD (`m = 0`) is the
    ///   separate, well-formed inconsistency clause, built by [`DlClause::inconsistency`];
    /// * an atom is negated — a negated head atom is not a rule, and admitting one would
    ///   silently produce an unsound program.
    pub fn new(atoms: Vec<ClauseAtom>) -> Self {
        assert!(
            !atoms.is_empty(),
            "a head disjunct may not be empty (an empty HEAD is the inconsistency clause)"
        );
        assert!(
            atoms.iter().all(|atom| !atom.negated),
            "a rule head may not be negated"
        );
        Self { atoms }
    }

    /// The single-atom disjunct — the shape every Datalog rule's head has.
    ///
    /// # Panics
    ///
    /// Panics if `atom` is negated — see [`Self::new`].
    pub fn atom(atom: ClauseAtom) -> Self {
        Self::new(vec![atom])
    }

    /// The conjuncts `V₁ ∧ … ∧ V_k`, in authored order. Never empty.
    pub fn atoms(&self) -> &[ClauseAtom] {
        &self.atoms
    }

    /// The one conjunct of a SINGLE-atom disjunct, or `None` if the disjunct is a proper
    /// conjunction.
    pub fn single_atom(&self) -> Option<&ClauseAtom> {
        match self.atoms.as_slice() {
            [only] => Some(only),
            _ => None,
        }
    }
}

/// Which of the five head forms a [`DlClause`] has.
///
/// The forms are decided in a fixed precedence — empty, then existential, then
/// disjunctive, then conjunctive, then atomic — so a clause that is both existential and
/// disjunctive classifies as [`Existential`](Self::Existential). The precedence is total
/// (a disjunct is never empty, so the last two cases exhaust `m = 1`) and
/// content-derived, so the classification is a pure function of the clause.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HeadForm {
    /// `m = 1`, that disjunct is exactly one atom, and `ȳ = ∅`. This IS a Datalog rule,
    /// and the only form [`crate::seminaive`] evaluates.
    Atomic,
    /// `ȳ ≠ ∅` — the head existentially quantifies at least one variable. A restricted
    /// chase with frontier-addressed Skolem witnesses consumes this form.
    Existential,
    /// `m > 1` with `ȳ = ∅` — a head disjunction. A hypertableau case split consumes this
    /// form: `purrdf-entail`'s OWL-Direct core classifies its own `SHOIQ(D)` DL-clauses
    /// through this very enum and branches, depth-first in authored disjunct order, on
    /// exactly the clauses that land here. No evaluator in THIS crate case-splits, so both
    /// [`crate::seminaive`] and [`crate::chase`] refuse the form by this name.
    Disjunctive,
    /// `m = 1` with `ȳ = ∅`, that one disjunct being a conjunction of two or more atoms —
    /// `→ p(x) ∧ q(x)`. There is no disjunction here and no witness to mint, so naming it
    /// [`Disjunctive`](Self::Disjunctive) would be a false diagnostic; it is its own form.
    ///
    /// It is not a Datalog rule either: a Datalog clause is a DEFINITE clause, with
    /// exactly one head atom. The conjunction is equivalent to one Datalog rule per
    /// conjunct over the same body, but this crate does not perform that expansion,
    /// because it would renumber the program — a
    /// [`Derivation`](crate::seminaive::Derivation) names its producing clause by authored
    /// index, so splitting one clause into several would silently move an observable. A
    /// caller who wants the split performs it before construction; the evaluator refuses
    /// the unsplit clause by this name.
    Conjunctive,
    /// `m = 0` — the head is `false`. The clause asserts that its body is unsatisfiable,
    /// so a body match is an inconsistency witness rather than a derivation.
    Inconsistency,
}

impl HeadForm {
    /// Whether this is the Datalog form, and therefore evaluable by [`crate::seminaive`].
    pub fn is_datalog(self) -> bool {
        matches!(self, Self::Atomic)
    }

    /// The form's name, for a diagnostic.
    fn name(self) -> &'static str {
        match self {
            Self::Atomic => "atomic",
            Self::Existential => "existential",
            Self::Disjunctive => "disjunctive",
            Self::Conjunctive => "conjunctive",
            Self::Inconsistency => "empty (false)",
        }
    }

    /// The indefinite article that precedes [`Self::name`] in a sentence, so a diagnostic
    /// reads "a conjunctive head" and "an existential head" rather than picking one
    /// article and being wrong half the time.
    pub(crate) fn article(self) -> &'static str {
        match self {
            Self::Atomic | Self::Existential | Self::Inconsistency => "an",
            Self::Disjunctive | Self::Conjunctive => "a",
        }
    }
}

impl fmt::Display for HeadForm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// A clause whose head form the Datalog evaluator has no semantics for.
///
/// This is a REFUSAL, not a deferral: a semi-naive least-fixpoint evaluator computes the
/// least model of a set of DEFINITE clauses — exactly one head atom, no quantifier — and
/// an existential, a disjunction, a conjunction and `false` each fall outside that class
/// by definition rather than by omission. Accepting one — or dropping it — would return
/// an answer that is not the model of the program it was given.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NonDatalogClause {
    /// The clause's index in authored program order.
    clause: usize,
    /// The head form that has no Datalog semantics.
    form: HeadForm,
}

impl NonDatalogClause {
    /// A refusal naming the offending clause and its head form.
    pub fn new(clause: usize, form: HeadForm) -> Self {
        Self { clause, form }
    }

    /// The clause's index in authored program order.
    pub fn clause(&self) -> usize {
        self.clause
    }

    /// The head form that has no Datalog semantics.
    pub fn form(&self) -> HeadForm {
        self.form
    }
}

impl fmt::Display for NonDatalogClause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "clause {} has {} {} head, which the Datalog evaluator has no semantics for",
            self.clause,
            self.form.article(),
            self.form
        )
    }
}

impl std::error::Error for NonDatalogClause {}

/// One DL-clause: `U₁ ∧ … ∧ Uₙ → ∃ȳ. (C₁ ∨ … ∨ Cₘ)`, each `Cᵢ` a conjunction of atoms.
///
/// See the [module docs](self) for the five head forms and for why all five are
/// representable while only the atomic one is evaluable here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DlClause {
    /// The body conjunction `U₁ ∧ … ∧ Uₙ`, in authored order — the order every plan
    /// coordinate and every provenance restoration is expressed against.
    body: Vec<ClauseAtom>,
    /// The existentially quantified head variables `ȳ`, in authored order. Empty for the
    /// atomic, disjunctive, conjunctive and inconsistency forms.
    existentials: Vec<String>,
    /// The head disjunction `C₁ ∨ … ∨ Cₘ`, in authored order. Empty means the head is
    /// `false`; each disjunct is itself a non-empty conjunction.
    head: Vec<HeadDisjunct>,
}

impl DlClause {
    /// A DL-clause with the general head `∃existentials. (C₁ ∨ … ∨ Cₘ)`.
    ///
    /// A disjunct's own well-formedness — non-empty, and no negated atom — is carried by
    /// [`HeadDisjunct`], so what is left to check here is exactly the rules that relate
    /// the quantifier list to the rest of the clause.
    ///
    /// # Panics
    ///
    /// Panics if the clause is malformed, which is a construction bug rather than a data
    /// state:
    ///
    /// * an existential name is repeated — `ȳ` is a list of distinct quantified variables;
    /// * an existential name occurs in NO head atom of any disjunct — quantifying a
    ///   variable the head never mentions is vacuous, and hides a typo in the name. One
    ///   occurrence anywhere in the head is enough: `ȳ` scopes over the whole disjunction,
    ///   so `∃y. (r(x, y) ∨ s(x))` is a well-formed disjunctive TGD and demanding an
    ///   occurrence in EVERY disjunct would reject it;
    /// * an existential name also occurs in the body — a body-occurring head variable is
    ///   universally quantified (it is a frontier variable), so the same name cannot also
    ///   be existential.
    pub fn new(head: Vec<HeadDisjunct>, existentials: Vec<String>, body: Vec<ClauseAtom>) -> Self {
        let mut head_variables = BTreeSet::new();
        for atom in head.iter().flat_map(HeadDisjunct::atoms) {
            atom.collect_variables(&mut head_variables);
        }
        let mut body_variables = BTreeSet::new();
        for atom in &body {
            atom.collect_variables(&mut body_variables);
        }
        let mut seen: BTreeSet<&str> = BTreeSet::new();
        for name in &existentials {
            assert!(
                seen.insert(name.as_str()),
                "existential variable {name} is quantified twice"
            );
            assert!(
                head_variables.contains(name),
                "existential variable {name} does not occur in the head"
            );
            assert!(
                !body_variables.contains(name),
                "existential variable {name} also occurs in the body, so it is a \
                 universally quantified frontier variable"
            );
        }

        Self {
            body,
            existentials,
            head,
        }
    }

    /// A Datalog clause: one head atom, no existential.
    ///
    /// # Panics
    ///
    /// Panics if `head` is negated — see [`HeadDisjunct::new`].
    pub fn datalog(head: ClauseAtom, body: Vec<ClauseAtom>) -> Self {
        Self::new(vec![HeadDisjunct::atom(head)], Vec::new(), body)
    }

    /// An inconsistency clause: the body implies `false`.
    ///
    /// A body match is an inconsistency witness, never a derivation.
    pub fn inconsistency(body: Vec<ClauseAtom>) -> Self {
        Self::new(Vec::new(), Vec::new(), body)
    }

    /// The body conjunction, in authored order.
    pub fn body(&self) -> &[ClauseAtom] {
        &self.body
    }

    /// The head disjunction, in authored order; empty means the head is `false`.
    pub fn head_disjuncts(&self) -> &[HeadDisjunct] {
        &self.head
    }

    /// Every head atom, disjunct by disjunct and, within a disjunct, conjunct by
    /// conjunct — both in authored order.
    ///
    /// This is the flattening the head-form-agnostic passes want: the stratifier's
    /// dependency edges, the planner's variable frame and the range-restriction check all
    /// ask "which atoms does this head mention", never "how are they grouped". For the
    /// atomic form the sequence is exactly the one head atom, so those passes are
    /// unchanged by the nesting.
    pub fn head_atoms(&self) -> impl Iterator<Item = &ClauseAtom> {
        self.head.iter().flat_map(HeadDisjunct::atoms)
    }

    /// The existentially quantified head variables `ȳ`, in authored order.
    pub fn existentials(&self) -> &[String] {
        &self.existentials
    }

    /// Which of the five head forms this clause has.
    ///
    /// The tests read exactly as the documented precedence does: empty, then existential,
    /// then disjunctive, then conjunctive, then atomic. Every clause matches one of them,
    /// because a [`HeadDisjunct`] is never empty.
    pub fn head_form(&self) -> HeadForm {
        let Some((first, rest)) = self.head.split_first() else {
            return HeadForm::Inconsistency;
        };
        if !self.existentials.is_empty() {
            return HeadForm::Existential;
        }
        if !rest.is_empty() {
            return HeadForm::Disjunctive;
        }
        if first.single_atom().is_none() {
            return HeadForm::Conjunctive;
        }
        HeadForm::Atomic
    }

    /// The single head atom of a DATALOG clause, or `None` for any other head form.
    ///
    /// This is the one accessor the semi-naive pipeline reads, so a non-Datalog clause
    /// cannot be mistaken for an atomic one by a caller that simply forgot to check: there
    /// is no "the head" to take.
    pub fn datalog_head(&self) -> Option<&ClauseAtom> {
        match self.head_form() {
            HeadForm::Atomic => self.head.first().and_then(HeadDisjunct::single_atom),
            HeadForm::Existential
            | HeadForm::Disjunctive
            | HeadForm::Conjunctive
            | HeadForm::Inconsistency => None,
        }
    }

    /// The frontier: the head variables the body also binds, in lexical order.
    ///
    /// For the atomic form this is exactly the range-restriction obligation
    /// [`compile`](crate::seminaive::compile) checks; for the existential form it is the
    /// address a Skolem witness is minted against.
    pub fn frontier_variables(&self) -> BTreeSet<String> {
        let mut head_variables = BTreeSet::new();
        for atom in self.head_atoms() {
            atom.collect_variables(&mut head_variables);
        }
        let mut body_variables = BTreeSet::new();
        for atom in &self.body {
            atom.collect_variables(&mut body_variables);
        }
        &head_variables & &body_variables
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const P: &str = "https://example.org/p";
    const Q: &str = "https://example.org/q";
    const R: &str = "https://example.org/r";
    /// The `rdf:type`-shaped predicate a DL concept name lowers through. PurRDF mints no
    /// vocabulary, so the fixture names its own.
    const TYPE: &str = "https://example.org/type";
    /// A DL concept name, as a constant IRI in the object position of [`TYPE`].
    const CONCEPT_C: &str = "https://example.org/C";

    fn v(name: &str) -> ClauseTerm {
        ClauseTerm::var(name)
    }

    fn atom(subject: &str, predicate: &str, object: &str) -> ClauseAtom {
        ClauseAtom::positive(v(subject), predicate, v(object))
    }

    /// The constant predicate IRI of an atom the fixture built with one.
    fn predicate_iri(atom: &ClauseAtom) -> &str {
        atom.predicate_iri()
            .expect("the fixture atoms carry constant predicate IRIs")
    }

    /// `C(?var)` — a concept assertion, as the binary atom `?var TYPE <C>`.
    fn concept(var: &str, concept_iri: &str) -> ClauseAtom {
        ClauseAtom::positive(v(var), TYPE, ClauseTerm::iri(concept_iri))
    }

    /// Form 1 — an atomic head with no existential IS a Datalog rule: it round trips
    /// through the IR and is the only form that yields a `datalog_head`.
    #[test]
    fn an_atomic_head_round_trips() {
        let clause = DlClause::datalog(
            atom("?X", R, "?Y"),
            vec![
                atom("?X", P, "?Z"),
                ClauseAtom::negated(v("?Z"), Q, v("?Y")),
            ],
        );
        assert_eq!(clause.head_form(), HeadForm::Atomic);
        assert!(clause.head_form().is_datalog());
        assert_eq!(clause.head_disjuncts().len(), 1);
        assert_eq!(clause.head_disjuncts()[0].atoms(), [atom("?X", R, "?Y")]);
        assert_eq!(clause.datalog_head(), Some(&atom("?X", R, "?Y")));
        assert_eq!(
            clause.head_atoms().collect::<Vec<_>>(),
            [&atom("?X", R, "?Y")]
        );
        assert!(clause.existentials().is_empty());
        assert_eq!(clause.body().len(), 2);
        assert!(!clause.body()[0].is_negated());
        assert!(clause.body()[1].is_negated());
        assert_eq!(clause.body()[1].predicate_iri(), Some(Q));
        assert_eq!(clause.body()[1].subject(), &v("?Z"));
        assert_eq!(clause.body()[1].object(), &v("?Y"));
    }

    /// Form 2 — an existential head round trips with its quantifier list intact, and the
    /// frontier is exactly the head variables the body also binds.
    #[test]
    fn an_existential_head_round_trips() {
        let clause = DlClause::new(
            vec![HeadDisjunct::atom(atom("?X", R, "?Y"))],
            vec!["?Y".to_owned()],
            vec![atom("?X", P, "?W")],
        );
        assert_eq!(clause.head_form(), HeadForm::Existential);
        assert!(!clause.head_form().is_datalog());
        assert_eq!(clause.existentials(), ["?Y"]);
        assert_eq!(clause.head_disjuncts().len(), 1);
        assert_eq!(clause.datalog_head(), None);
        assert_eq!(
            clause.frontier_variables(),
            BTreeSet::from(["?X".to_owned()])
        );
    }

    /// Form 3 — a disjunctive head round trips with every disjunct, in authored order.
    #[test]
    fn a_disjunctive_head_round_trips() {
        let clause = DlClause::new(
            vec![
                HeadDisjunct::atom(atom("?X", Q, "?Y")),
                HeadDisjunct::atom(atom("?X", R, "?Y")),
            ],
            Vec::new(),
            vec![atom("?X", P, "?Y")],
        );
        assert_eq!(clause.head_form(), HeadForm::Disjunctive);
        assert!(!clause.head_form().is_datalog());
        assert_eq!(clause.datalog_head(), None);
        assert_eq!(
            clause.head_atoms().map(predicate_iri).collect::<Vec<_>>(),
            [Q, R]
        );
    }

    /// Form 4 — a CONJUNCTIVE head (`m = 1`, several conjuncts, no existential) is its own
    /// form, because it is neither a disjunction nor a Datalog rule: calling it either
    /// would be a false diagnostic.
    #[test]
    fn a_conjunctive_head_classifies_as_conjunctive() {
        let clause = DlClause::new(
            vec![HeadDisjunct::new(vec![
                atom("?X", Q, "?Y"),
                atom("?X", R, "?Y"),
            ])],
            Vec::new(),
            vec![atom("?X", P, "?Y")],
        );
        assert_eq!(clause.head_form(), HeadForm::Conjunctive);
        assert!(!clause.head_form().is_datalog());
        assert_eq!(clause.datalog_head(), None, "there is no single head atom");
        assert_eq!(clause.head_disjuncts().len(), 1, "no disjunction at all");
        assert_eq!(clause.head_disjuncts()[0].atoms().len(), 2);
        assert_eq!(clause.head_disjuncts()[0].single_atom(), None);
        assert_eq!(
            clause.head_atoms().map(predicate_iri).collect::<Vec<_>>(),
            [Q, R]
        );
        assert_eq!(HeadForm::Conjunctive.to_string(), "conjunctive");
    }

    /// Form 5 — an empty head is the inconsistency clause `body → false`.
    #[test]
    fn an_empty_head_round_trips() {
        let clause = DlClause::inconsistency(vec![
            atom("?X", P, "?Y"),
            ClauseAtom::negated(v("?X"), Q, v("?Y")),
        ]);
        assert_eq!(clause.head_form(), HeadForm::Inconsistency);
        assert!(!clause.head_form().is_datalog());
        assert!(clause.head_disjuncts().is_empty());
        assert_eq!(clause.head_atoms().count(), 0);
        assert_eq!(clause.datalog_head(), None);
        assert_eq!(clause.body().len(), 2);
        assert!(clause.frontier_variables().is_empty());
    }

    /// A clause that is BOTH existential and disjunctive classifies as existential — the
    /// documented precedence, so the classification is total.
    #[test]
    fn an_existential_disjunction_classifies_as_existential() {
        let clause = DlClause::new(
            vec![
                HeadDisjunct::atom(atom("?X", Q, "?Y")),
                HeadDisjunct::atom(atom("?Y", R, "?X")),
            ],
            vec!["?Y".to_owned()],
            vec![atom("?X", P, "?W")],
        );
        assert_eq!(clause.head_form(), HeadForm::Existential);
    }

    /// `A ⊑ ∃r.C` — the reason a disjunct is a CONJUNCTION.
    ///
    /// The axiom lowers to `A(?X) → ∃?Y. (r(?X, ?Y) ∧ C(?Y))`: one disjunct, two atoms,
    /// one shared Skolem witness `?Y`. Splitting it into two single-atom clauses would
    /// mint two unrelated witnesses and so state something strictly weaker, which is
    /// exactly why the head carries the nesting level.
    #[test]
    fn a_conjunctive_existential_head_round_trips() {
        let clause = DlClause::new(
            vec![HeadDisjunct::new(vec![
                atom("?X", R, "?Y"),
                concept("?Y", CONCEPT_C),
            ])],
            vec!["?Y".to_owned()],
            vec![concept("?X", "https://example.org/A")],
        );
        assert_eq!(clause.head_form(), HeadForm::Existential);
        assert!(!clause.head_form().is_datalog());
        assert_eq!(clause.datalog_head(), None);
        assert_eq!(clause.existentials(), ["?Y"]);

        // ONE disjunct holding BOTH atoms: the witness `?Y` is shared, not duplicated.
        assert_eq!(clause.head_disjuncts().len(), 1);
        let conjuncts = clause.head_disjuncts()[0].atoms();
        assert_eq!(conjuncts.len(), 2);
        assert_eq!(conjuncts[0], atom("?X", R, "?Y"));
        assert_eq!(conjuncts[1], concept("?Y", CONCEPT_C));
        assert_eq!(conjuncts[1].object(), &ClauseTerm::iri(CONCEPT_C));
        assert!(conjuncts.iter().all(|a| !a.is_negated()));

        // The frontier is the body-bound head variable, and `?Y` is NOT in it.
        assert_eq!(
            clause.frontier_variables(),
            BTreeSet::from(["?X".to_owned()])
        );
    }

    /// A disjunction OF CONJUNCTIONS round trips with both nesting levels in authored
    /// order — `→ (p(?X,?Y) ∧ q(?X,?Y)) ∨ (r(?X,?Y) ∧ C(?Y))`.
    #[test]
    fn a_disjunction_of_conjunctions_round_trips() {
        let clause = DlClause::new(
            vec![
                HeadDisjunct::new(vec![atom("?X", P, "?Y"), atom("?X", Q, "?Y")]),
                HeadDisjunct::new(vec![atom("?X", R, "?Y"), concept("?Y", CONCEPT_C)]),
            ],
            Vec::new(),
            vec![atom("?X", P, "?Y")],
        );
        assert_eq!(clause.head_form(), HeadForm::Disjunctive);
        assert_eq!(clause.datalog_head(), None);
        assert_eq!(
            clause
                .head_disjuncts()
                .iter()
                .map(|d| d.atoms().iter().map(predicate_iri).collect())
                .collect::<Vec<Vec<_>>>(),
            [vec![P, Q], vec![R, TYPE]]
        );
        // The flattening a head-form-agnostic pass sees: disjunct order, then conjunct
        // order.
        assert_eq!(
            clause.head_atoms().map(predicate_iri).collect::<Vec<_>>(),
            [P, Q, R, TYPE]
        );
    }

    /// An existential need occur in only ONE disjunct, not in every one.
    ///
    /// `ȳ` scopes over the whole disjunction, so `∃?Y. (r(?X, ?Y) ∨ s(?X, ?X))` is a
    /// well-formed disjunctive TGD — the second branch simply leaves `?Y` unconstrained.
    /// Demanding an occurrence in every disjunct would reject a legal formula; demanding
    /// one SOMEWHERE is what catches the typo.
    #[test]
    fn an_existential_need_not_occur_in_every_disjunct() {
        let clause = DlClause::new(
            vec![
                HeadDisjunct::atom(atom("?X", R, "?Y")),
                HeadDisjunct::atom(atom("?X", Q, "?X")),
            ],
            vec!["?Y".to_owned()],
            vec![atom("?X", P, "?W")],
        );
        assert_eq!(clause.head_form(), HeadForm::Existential);
        assert_eq!(clause.existentials(), ["?Y"]);
        assert_eq!(
            clause.frontier_variables(),
            BTreeSet::from(["?X".to_owned()])
        );
    }

    /// A constant term renders to the exact bytes the store interns; a variable has no
    /// plan-time surface at all, because its surface is a runtime binding.
    #[test]
    fn clause_term_surface_is_the_stored_bytes_of_a_constant_only() {
        assert_eq!(
            ClauseTerm::iri("https://example.org/a")
                .surface()
                .as_deref(),
            Some("<https://example.org/a>")
        );
        assert_eq!(
            ClauseTerm::literal("\"7\"^^<http://www.w3.org/2001/XMLSchema#integer>")
                .surface()
                .as_deref(),
            Some("\"7\"^^<http://www.w3.org/2001/XMLSchema#integer>")
        );
        assert_eq!(ClauseTerm::var("?x").surface(), None);
        assert!(ClauseTerm::var("?x").is_var());
        assert_eq!(ClauseTerm::var("?x").variable(), Some("?x"));
        assert_eq!(ClauseTerm::iri("https://example.org/a").variable(), None);
    }

    /// The default graph's surface is the EMPTY surface — "no name" stated as no name.
    /// No IRI surface and no literal surface is empty, so the denotation is unambiguous,
    /// and no vocabulary IRI was minted to express it.
    #[test]
    fn the_default_graph_is_the_empty_surface() {
        let default = ClauseTerm::default_graph();
        assert_eq!(default, ClauseTerm::DefaultGraph);
        assert_eq!(default.surface().as_deref(), Some(""));
        assert!(default.is_default_graph());
        assert!(!default.is_var());
        assert_eq!(default.variable(), None);
        assert_eq!(default.iri_value(), None);
        assert!(!ClauseTerm::iri("https://example.org/g").is_default_graph());
        // Every other constant's surface is non-empty, so nothing can alias it.
        assert!(
            !ClauseTerm::iri("")
                .surface()
                .expect("an IRI has a surface")
                .is_empty()
        );
    }

    /// A VARIABLE PREDICATE round-trips through the IR: `prp-dom`'s second body atom is
    /// `T(?x, ?p, ?y)`, and `?p` is an ordinary clause variable there — collected into the
    /// frontier, and reported as having no constant predicate name.
    #[test]
    fn a_variable_predicate_round_trips() {
        let domain = ClauseTerm::iri("https://example.org/domain");
        let clause = DlClause::datalog(
            ClauseAtom::positive(v("?x"), TYPE, v("?c")),
            vec![
                ClauseAtom::positive(v("?p"), "https://example.org/domain", v("?c")),
                ClauseAtom::quad(v("?x"), v("?p"), v("?y"), ClauseTerm::var("?g")),
            ],
        );
        let second = &clause.body()[1];
        assert_eq!(second.predicate(), &v("?p"));
        assert!(second.predicate().is_var());
        assert_eq!(
            second.predicate_iri(),
            None,
            "a variable predicate has no constant name to report"
        );
        assert_eq!(second.graph(), &v("?g"));
        assert_eq!(
            second.terms(),
            [&v("?x"), &v("?p"), &v("?y"), &ClauseTerm::var("?g")]
        );
        // `?p` and `?g` are ordinary variables: both reach the clause's variable sets.
        assert_eq!(
            clause.frontier_variables(),
            BTreeSet::from(["?c".to_owned(), "?x".to_owned()])
        );
        let mut body_variables = BTreeSet::new();
        for atom in clause.body() {
            atom.collect_variables(&mut body_variables);
        }
        assert!(body_variables.contains("?p"), "{body_variables:?}");
        assert!(body_variables.contains("?g"), "{body_variables:?}");
        assert_eq!(clause.body()[0].predicate(), &domain);
    }

    /// An atom built without a graph is an atom in the DEFAULT GRAPH — not a wildcard —
    /// and its predicate is the constant IRI it was given.
    #[test]
    fn a_graphless_atom_is_in_the_default_graph() {
        let atom = ClauseAtom::positive(v("?s"), P, v("?o"));
        assert_eq!(atom.graph(), &ClauseTerm::DefaultGraph);
        assert_eq!(atom.predicate(), &ClauseTerm::iri(P));
        assert_eq!(atom.predicate_iri(), Some(P));
        assert!(!atom.is_negated());
        let negated = ClauseAtom::negated(v("?s"), P, v("?o"));
        assert!(negated.is_negated());
        assert_eq!(negated.graph(), &ClauseTerm::DefaultGraph);
        // A named graph is an ordinary constant term in the same position.
        let named = ClauseAtom::quad(
            v("?s"),
            ClauseTerm::iri(P),
            v("?o"),
            ClauseTerm::iri("https://example.org/g1"),
        );
        assert_eq!(named.graph(), &ClauseTerm::iri("https://example.org/g1"));
        assert!(
            ClauseAtom::negated_quad(
                v("?s"),
                ClauseTerm::iri(P),
                v("?o"),
                ClauseTerm::DefaultGraph
            )
            .is_negated()
        );
    }

    #[test]
    #[should_panic(expected = "legal in the graph position only")]
    fn the_default_graph_is_refused_in_a_term_position() {
        let _ = ClauseAtom::quad(
            ClauseTerm::DefaultGraph,
            ClauseTerm::iri(P),
            v("?o"),
            ClauseTerm::DefaultGraph,
        );
    }

    #[test]
    #[should_panic(expected = "a rule head may not be negated")]
    fn a_negated_head_atom_is_not_a_rule() {
        let _ = DlClause::datalog(ClauseAtom::negated(v("?X"), P, v("?Y")), Vec::new());
    }

    /// A negated atom is refused wherever in a disjunct it sits, not only in first
    /// position.
    #[test]
    #[should_panic(expected = "a rule head may not be negated")]
    fn a_negated_conjunct_is_not_a_rule_either() {
        let _ = HeadDisjunct::new(vec![
            atom("?X", R, "?Y"),
            ClauseAtom::negated(v("?Y"), Q, v("?X")),
        ]);
    }

    /// An EMPTY DISJUNCT is refused, and it is a different thing from an empty HEAD: the
    /// empty head is the well-formed inconsistency clause, built first here to show the
    /// two are not confused.
    #[test]
    #[should_panic(expected = "a head disjunct may not be empty")]
    fn an_empty_disjunct_is_refused_though_an_empty_head_is_not() {
        let empty_head = DlClause::inconsistency(vec![atom("?X", P, "?Y")]);
        assert_eq!(empty_head.head_form(), HeadForm::Inconsistency);
        let _ = HeadDisjunct::new(Vec::new());
    }

    #[test]
    #[should_panic(expected = "does not occur in the head")]
    fn a_vacuous_existential_is_refused() {
        let _ = DlClause::new(
            vec![HeadDisjunct::atom(atom("?X", R, "?Y"))],
            vec!["?Z".to_owned()],
            vec![atom("?X", P, "?Y")],
        );
    }

    /// Vacuity is judged over EVERY disjunct's atoms: a name absent from all of them is
    /// still a typo, even when the head has several disjuncts to hide in.
    #[test]
    #[should_panic(expected = "does not occur in the head")]
    fn an_existential_in_no_disjunct_is_still_vacuous() {
        let _ = DlClause::new(
            vec![
                HeadDisjunct::new(vec![atom("?X", R, "?Y"), concept("?Y", CONCEPT_C)]),
                HeadDisjunct::atom(atom("?X", Q, "?X")),
            ],
            vec!["?Z".to_owned()],
            vec![atom("?X", P, "?W")],
        );
    }

    #[test]
    #[should_panic(expected = "also occurs in the body")]
    fn a_frontier_variable_may_not_be_existential() {
        let _ = DlClause::new(
            vec![HeadDisjunct::atom(atom("?X", R, "?Y"))],
            vec!["?Y".to_owned()],
            vec![atom("?X", P, "?Y")],
        );
    }

    #[test]
    #[should_panic(expected = "quantified twice")]
    fn a_repeated_existential_is_refused() {
        let _ = DlClause::new(
            vec![HeadDisjunct::atom(atom("?X", R, "?Y"))],
            vec!["?Y".to_owned(), "?Y".to_owned()],
            vec![atom("?X", P, "?W")],
        );
    }

    /// The refusal names both the clause and the form, and is a `std::error::Error`.
    #[test]
    fn the_non_datalog_refusal_names_the_clause_and_the_form() {
        let refusal = NonDatalogClause::new(3, HeadForm::Disjunctive);
        assert_eq!(refusal.clause(), 3);
        assert_eq!(refusal.form(), HeadForm::Disjunctive);
        let rendered = refusal.to_string();
        assert!(rendered.contains("clause 3"), "{rendered}");
        assert!(rendered.contains("a disjunctive head"), "{rendered}");
        let _: &dyn std::error::Error = &refusal;
        assert_eq!(HeadForm::Inconsistency.to_string(), "empty (false)");
        assert_eq!(HeadForm::Existential.to_string(), "existential");
        assert_eq!(HeadForm::Atomic.to_string(), "atomic");
        assert_eq!(HeadForm::Conjunctive.to_string(), "conjunctive");

        // The article is chosen per form, so no diagnostic reads "an conjunctive".
        let conjunctive = NonDatalogClause::new(0, HeadForm::Conjunctive).to_string();
        assert!(conjunctive.contains("a conjunctive head"), "{conjunctive}");
        let existential = NonDatalogClause::new(0, HeadForm::Existential).to_string();
        assert!(existential.contains("an existential head"), "{existential}");
    }
}
