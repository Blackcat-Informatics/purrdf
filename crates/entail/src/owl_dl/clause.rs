// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The `SHOIQ(D)` knowledge base as DL-CLAUSES — the input the hypertableau resolves.
//!
//! A DL-clause is
//!
//! ```text
//! U₁ ∧ … ∧ Uₙ  →  (V₁ ∧ … ∧ V_j) ∨ … ∨ (W₁ ∧ … ∧ W_k)
//! ```
//!
//! with every variable universally quantified over the completion graph, an EMPTY head
//! meaning `⊥`, and existential quantification carried by the at-least head atom
//! [`HeadAtom::AtLeast`] rather than by a quantifier prefix. That is the shape
//! [`purrdf_datalog::clause`] classifies into five head forms, and
//! [`DlClause::head_form`] answers with that crate's own
//! [`HeadForm`](purrdf_datalog::clause::HeadForm): the taxonomy is shared, so
//! [`HeadForm::Disjunctive`] names exactly the clauses
//! [`hyper`](crate::owl_dl::hyper) case-splits over.
//!
//! # Why the ATOMS are not `purrdf-datalog`'s
//!
//! `purrdf_datalog::clause::ClauseAtom` is the arity-4 quad `triple(?s, ?p, ?o, ?g)` over
//! term SURFACES. Two atoms of this fragment have no quad to be:
//!
//! * `≥n r.C(x)` — a counting head atom. There is no triple whose assertion means "x has n
//!   pairwise-distinct r-successors in C"; the number and the filler are part of the atom.
//! * `x ≈ y` — an equality head atom, which the `≤n` clauses below make the whole point of
//!   the calculus. Asserting it merges two graph nodes rather than adding a triple.
//!
//! Encoding either as a quad would need a predicate IRI to carry it, and **PurRDF mints no
//! vocabulary**; encoding a concept id as a literal surface would put every rule application
//! behind string comparison of terms that denote nothing in RDF. So the atoms here are
//! concept ids and role pairs over graph nodes — the representation the completion graph
//! already is — while the HEAD FORM taxonomy, the thing a case split is dispatched on,
//! is imported rather than re-invented.
//!
//! # Where the clauses come from
//!
//! One pass over the finalized [`ConceptTable`](crate::owl_dl::concept::ConceptTable), in
//! ascending concept-id order, plus one pass over the role axioms. Each concept id yields the
//! clauses that say what holding it FORCES, so the search never walks a concept tree: the
//! structure is compiled out in front, and a node's label is then just a set of trigger
//! atoms.
//!
//! | concept | clause | reading |
//! |---|---|---|
//! | `⊥` | `⊥(x) → ` | the empty head is `false` |
//! | `C₁ ⊓ … ⊓ Cₙ` | `c(x) → C₁(x) ∧ … ∧ Cₙ(x)` | conjunctive head |
//! | `C₁ ⊔ … ⊔ Cₙ` | `c(x) → C₁(x) ∨ … ∨ Cₙ(x)` | **disjunctive head** |
//! | `∃r.C` | `c(x) → ≥1 r.C(x)` | existential head |
//! | `≥n r.C`, `n ≥ 1` | `c(x) → ≥n r.C(x)` | existential head |
//! | `≥0 r.C` | — | `≥0 r.C` is `⊤`, which forces nothing |
//! | `∀r.C` | `c(x) ∧ r(x,y) → C(y)` | a two-variable body, matched against an EDGE |
//! | `≤n r.C` | `c(x) ∧ r(x,y) → C(y) ∨ ¬C(y)` | the filler must be DECIDED to be counted |
//! | `≤n r.C` | `c(x) ∧ ⋀ᵢ₌₀..ₙ (r(x,yᵢ) ∧ C(yᵢ)) → ⋁_{i<j} yᵢ ≈ yⱼ` | **disjunctive head of equalities** |
//! | `∃r.Self` | `c(x) → r(x,x)` | an edge head |
//! | `¬∃r.Self` | `c(x) ∧ r(x,x) → ` | `owl:IrreflexiveProperty`, verbatim |
//! | `{a₁,…,aₙ}` | `c(x) → x ≈ a₁ ∨ … ∨ x ≈ aₙ` | **disjunctive** for `n > 1`; a merge for `n = 1` |
//! | `¬{a₁,…,aₙ}` | `c(x) ∧ denotes_{aᵢ}(x) → ` | one clause per member |
//! | any `C` | `C(x) ∧ ¬C(x) → ` | the complementary-pair clash, once per pair |
//!
//! and from the knowledge base itself:
//!
//! | axiom | clause |
//! |---|---|
//! | absorbed inclusion ([`Kb::absorbed`]) | `⋀ guard → D(x_head)`, verbatim |
//! | `owl:AsymmetricProperty` `r` | `r(x,y) ∧ r(y,x) → ` |
//! | `owl:propertyDisjointWith` `r`, `s` | `r(x,y) ∧ s(x,y) → ` |
//!
//! The absorbed table is [`crate::owl_dl::absorb`]'s output and it arrives here already
//! clausified — guard, head variable, head concept — so this module TRANSLATES it and does
//! not decide it. That is deliberate: which inclusions become guarded clauses is a claim about
//! the semantics of a completion graph (the faithful-antecedent criterion), and it belongs
//! where that argument is written down rather than spread over a clause emitter. `A ⊑ D` is
//! the degenerate one-atom guard; `∃r.C ⊑ D`, `A ⊓ B ⊑ D`, `rdfs:domain` and `rdfs:range` are
//! the shapes an earlier revision could not absorb at all.
//!
//! The inclusions whose antecedent nothing can guard are NOT clauses here: they are
//! internalized as the meta-concepts [`Kb::meta`], which the completion graph seeds into every
//! abstract node's label, so the `⊔`-clause of `nnf(¬C ⊔ D)` fires on every node. That is the
//! same internalization the incumbent uses, kept so the two calculi differ in their rules and
//! not in their input.
//!
//! # The two things this table deliberately does NOT encode
//!
//! * **The concrete domain.** A `Data(r)` leaf's constraint is not a clause of bounded
//!   arity: satisfiability of `r₁ ∩ … ∩ rₘ ∩ ¬s₁ ∩ … ∩ ¬sₖ` is a question about the WHOLE set
//!   of ranges on a node, and it is answered by [`crate::owl_dl::data`] against
//!   `purrdf-xsd`'s value spaces. [`Graph::data_clashes`](crate::owl_dl::graph::Graph::data_clashes)
//!   stays the decision procedure for it, unchanged and shared by both calculi.
//! * **A role's closure.** A body atom `r(x,y)` is read as "`y` is an `r`-NEIGHBOUR of `x`",
//!   which [`Graph::neighbors`](crate::owl_dl::graph::Graph::neighbors) answers through the
//!   role hierarchy, the inverse-role declarations and the transitive closure. Compiling the
//!   closure into clauses instead would need a clause per derived pair, and transitivity has
//!   no finite clause set over an unbounded graph.
//!
//! # Determinism
//!
//! Clauses are emitted in ascending concept-id order (which is parse order), then in the
//! `BTreeMap`/`BTreeSet` order of the role axioms; the trigger index is a `BTreeMap`; and
//! every body and head is a `Vec` in the order emitted. Nothing here is read out of a hash
//! map, so the clause set — and therefore the branch order of every search over it — is a
//! pure function of the knowledge base.
//!
//! A DISJUNCTIVE head's alternatives are emitted in the order [`Kb::order_disjuncts`] gives —
//! the alternatives that mint no witnesses first, and a STABLE sort, so the canonical order the
//! members already carry survives inside each rank. It is a pure function of the concept table
//! and the absorbed clauses and it is decided ONCE, here, rather than at each case split. That
//! ordering is deliberately not the interner's: `Kb::order_disjuncts` states why the identity
//! order and the search order must stay two orders.

use std::collections::BTreeMap;

use purrdf_datalog::clause::HeadForm;

use crate::owl_dl::Kb;
use crate::owl_dl::concept::{Decomp, Role};

/// A clause variable, as an index into the matcher's binding frame.
///
/// Variable `0` is the TRIGGER variable: every clause is matched by binding it to one node
/// and joining outward, which is what makes the trigger index below a complete applicability
/// test rather than a heuristic.
pub(crate) type Var = u32;

/// One body atom of a DL-clause.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BodyAtom {
    /// `C(xᵢ)` — the concept is in the node's label.
    Concept {
        /// The variable the atom constrains.
        var: Var,
        /// The concept id.
        concept: u32,
    },
    /// `r(xᵢ, xⱼ)` — `xⱼ` is an `r`-NEIGHBOUR of `xᵢ`, read through the role hierarchy, the
    /// inverse-role declarations and the transitive closure.
    Role {
        /// The variable the edge leaves.
        from: Var,
        /// The variable the edge enters.
        to: Var,
        /// The role, named or inverse.
        role: Role,
    },
    /// `r(x₀,y₁) ∧ C(y₁) ∧ … ∧ r(x₀,y_count) ∧ C(y_count)` with the `yᵢ` pairwise different —
    /// the counted successors of a `≤n` restriction, as ONE schematic atom binding
    /// `count` consecutive variables from `first`.
    ///
    /// # Why it is schematic rather than `2·count` ordinary atoms
    ///
    /// `count` is `n + 1`, and `n` comes from the ontology: `owl:maxCardinality "300"` is
    /// ordinary OWL 2. Writing the atoms out would put `2n + 2` body atoms and — because the
    /// head is the disjunction of the tuple's PAIRS — `n(n+1)/2` head disjuncts in the clause
    /// set for a bound the completion graph may never come near, and would make the clause set
    /// quadratic in a number the ontology chose. Schematic, the clause costs one atom whatever
    /// `n` is, and the disjunction is expanded at MATCH time from the successors that actually
    /// exist — so the cost of a `≤n` restriction is bounded by the graph rather than by `n`.
    ///
    /// The matcher binds the `yᵢ` in strictly increasing node order, which enumerates the
    /// `count`-element SETS of successors rather than the `count!`-times-larger tuple space.
    /// That is sound because the head is invariant under permuting them.
    Successors {
        /// The role the successors are reached by.
        role: Role,
        /// The concept every counted successor satisfies.
        filler: u32,
        /// The first variable this atom binds.
        first: Var,
        /// How many successors it binds — `n + 1` for a `≤n` restriction.
        count: u32,
    },
    /// `xᵢ` DENOTES the individual `a` — the atom a negated nominal clashes against.
    ///
    /// Not a concept atom: OWL 2 makes no unique name assumption, so "denotes `a`" is a
    /// property of the node's identity (the names it was identified with), never a name
    /// comparison.
    Denotes {
        /// The variable the atom constrains.
        var: Var,
        /// The individual term id.
        individual: u32,
    },
}

impl BodyAtom {
    /// The variable this atom BINDS when matched — the one that may still be unbound.
    ///
    /// Only a role atom binds: it is the join that walks the graph. Every other atom is a
    /// filter over an already-bound variable, which is what makes the matcher's left-to-right
    /// body order a complete plan.
    #[cfg(test)]
    const fn binds(self) -> Option<Var> {
        match self {
            Self::Role { to, .. } => Some(to),
            Self::Successors { first, count, .. } => Some(first + count - 1),
            Self::Concept { .. } | Self::Denotes { .. } => None,
        }
    }
}

/// One head atom of a DL-clause.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HeadAtom {
    /// `C(xᵢ)` — add the concept to the node's label.
    Concept {
        /// The variable the atom asserts about.
        var: Var,
        /// The concept id.
        concept: u32,
    },
    /// `r(xᵢ, xᵢ)` — the node is its own `r`-successor (`owl:hasSelf`).
    SelfLoop {
        /// The variable the loop is on.
        var: Var,
        /// The role.
        role: Role,
    },
    /// `≥n r.C(xᵢ)` — the node has `n` pairwise-distinct `r`-neighbours satisfying `C`.
    ///
    /// The existential of the clause language: asserting it MINTS anonymous witnesses (no
    /// IRI, no blank node) up to the shortfall, and forces the whole witness set pairwise
    /// distinct.
    AtLeast {
        /// The variable the successors belong to.
        var: Var,
        /// How many pairwise-distinct successors are demanded.
        n: u32,
        /// The role they are reached by.
        role: Role,
        /// The concept they satisfy.
        filler: u32,
    },
    /// `⋁_{i<j} yᵢ ≈ yⱼ` over the `count` variables from `first` — the `≤n` restriction's own
    /// disjunction, kept schematic for the reason [`BodyAtom::Successors`] is.
    ///
    /// This is the ONLY equality between two variables the fragment derives: `≤n r.C` is the
    /// only axiom form that forces two elements together without naming either of them, and it
    /// forces them as a CHOICE among pairs rather than as one pair. So there is no bare
    /// `xᵢ ≈ xⱼ` head atom beside this one — a head that could assert one pair unconditionally
    /// would be a head no axiom of this fragment produces.
    ///
    /// It is the ONE head atom that stands for several DISJUNCTS rather than for one atom, so
    /// a clause carrying it is [`HeadForm::Disjunctive`] however many disjuncts its `head`
    /// vector happens to hold: the disjunction is in the atom. Grounding expands it against a
    /// matched frame; nothing else in this module treats it specially.
    EqualSomePair {
        /// The first of the counted variables.
        first: Var,
        /// How many of them there are; the disjunction has `count · (count − 1) / 2` disjuncts.
        count: u32,
    },
    /// `xᵢ ≈ a` — identify the node with the individual `a`'s root.
    EqualIndividual {
        /// The variable to identify.
        var: Var,
        /// The individual term id.
        individual: u32,
    },
}

/// One DL-clause: `⋀ body → ⋁ (⋀ conjuncts)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DlClause {
    /// The body conjunction, in the order the matcher joins it. Variable `0` is bound by the
    /// caller; every later atom either filters a bound variable or binds one through a role.
    pub(crate) body: Vec<BodyAtom>,
    /// The head disjunction, each disjunct a conjunction of atoms. EMPTY means `⊥`.
    ///
    /// One entry may itself stand for several disjuncts: see [`HeadAtom::EqualSomePair`].
    pub(crate) head: Vec<Vec<HeadAtom>>,
}

impl DlClause {
    /// The concept whose presence in a node's label can make this clause applicable AT that
    /// node, or `None` for a clause whose body opens with a role atom.
    ///
    /// A complete applicability filter, not a heuristic: matching always binds variable `0`
    /// first, so a clause whose first body atom is `C(x₀)` cannot match a node without `C`.
    pub(crate) fn trigger(&self) -> Option<u32> {
        match self.body.first() {
            Some(&BodyAtom::Concept { var: 0, concept }) => Some(concept),
            _ => None,
        }
    }

    /// Which of [`purrdf-datalog`](purrdf_datalog::clause)'s five head forms this clause has.
    ///
    /// The same precedence that crate documents — empty, then existential, then disjunctive,
    /// then conjunctive, then atomic — over this fragment's own atoms. It is what the
    /// hypertableau dispatches on:
    /// [`Inconsistency`](HeadForm::Inconsistency) closes the branch,
    /// [`Atomic`](HeadForm::Atomic) and [`Conjunctive`](HeadForm::Conjunctive) are
    /// deterministic assertions, [`Existential`](HeadForm::Existential) mints witnesses, and
    /// [`Disjunctive`](HeadForm::Disjunctive) is the ONE source of don't-know
    /// nondeterminism in the calculus.
    pub(crate) fn head_form(&self) -> HeadForm {
        let Some((first, rest)) = self.head.split_first() else {
            return HeadForm::Inconsistency;
        };
        if self
            .head
            .iter()
            .flatten()
            .any(|atom| matches!(atom, HeadAtom::AtLeast { .. }))
        {
            return HeadForm::Existential;
        }
        if !rest.is_empty()
            || self
                .head
                .iter()
                .flatten()
                .any(|atom| matches!(atom, HeadAtom::EqualSomePair { .. }))
        {
            return HeadForm::Disjunctive;
        }
        if first.len() > 1 {
            return HeadForm::Conjunctive;
        }
        HeadForm::Atomic
    }

    /// How many variables the clause uses.
    ///
    /// Read by this module's own well-formedness test rather than by the matcher: a binding
    /// frame GROWS as variables bind, so the matcher never needs the width up front — which is
    /// what keeps a `≤300 r.C` clause from costing three hundred slots at a node with two
    /// successors.
    #[cfg(test)]
    pub(crate) fn arity(&self) -> usize {
        let mut width = 1usize;
        let mut widen = |var: Var| width = width.max(var as usize + 1);
        for atom in &self.body {
            match *atom {
                BodyAtom::Concept { var, .. } | BodyAtom::Denotes { var, .. } => widen(var),
                BodyAtom::Role { from, to, .. } => {
                    widen(from);
                    widen(to);
                }
                BodyAtom::Successors { first, count, .. } => widen(first + count - 1),
            }
        }
        for atom in self.head.iter().flatten() {
            match *atom {
                HeadAtom::Concept { var, .. }
                | HeadAtom::SelfLoop { var, .. }
                | HeadAtom::AtLeast { var, .. }
                | HeadAtom::EqualIndividual { var, .. } => widen(var),
                HeadAtom::EqualSomePair { first, count } => widen(first + count - 1),
            }
        }
        width
    }

    /// Whether the clause is well-formed for the matcher: every variable is bound before it
    /// is used, and only variable `0` is bound from outside.
    ///
    /// Checked once per derived clause set by this module's own tests rather than asserted at
    /// each match, because a malformed clause is a bug in [`derive`] and not a data state.
    #[cfg(test)]
    fn is_matchable(&self) -> bool {
        let mut bound = vec![false; self.arity()];
        bound[0] = true;
        for atom in &self.body {
            match *atom {
                BodyAtom::Concept { var, .. } | BodyAtom::Denotes { var, .. } => {
                    if !bound[var as usize] {
                        return false;
                    }
                }
                BodyAtom::Role { from, to, .. } => {
                    if !bound[from as usize] {
                        return false;
                    }
                    bound[to as usize] = true;
                }
                BodyAtom::Successors { first, count, .. } => {
                    for var in first..first + count {
                        bound[var as usize] = true;
                    }
                }
            }
        }
        let head_vars_bound = self.head.iter().flatten().all(|atom| match *atom {
            HeadAtom::Concept { var, .. }
            | HeadAtom::SelfLoop { var, .. }
            | HeadAtom::AtLeast { var, .. }
            | HeadAtom::EqualIndividual { var, .. } => bound[var as usize],
            HeadAtom::EqualSomePair { first, count } => {
                (first..first + count).all(|var| bound[var as usize])
            }
        });
        head_vars_bound
            && self
                .body
                .iter()
                .filter_map(|atom| atom.binds())
                .all(|var| (var as usize) < self.arity())
    }
}

/// Every DL-clause of one knowledge base, indexed by the trigger concept.
#[derive(Debug, Clone)]
pub(crate) struct ClauseSet {
    /// The clauses, in derivation order.
    clauses: Vec<DlClause>,
    /// Trigger concept id → the indices of the clauses it can make applicable, ascending.
    by_trigger: BTreeMap<u32, Vec<usize>>,
    /// The clauses no CONCEPT triggers — a body that opens with a role atom, a `denotes` atom
    /// or nothing at all — so they are tried at every node of every round.
    ///
    /// # What lands here, and the bound that makes it affordable
    ///
    /// Exactly two populations, and neither scales with the ontology's CONCEPT count:
    ///
    /// * the two role axioms (`owl:AsymmetricProperty`, `owl:propertyDisjointWith`), one
    ///   clause each;
    /// * an absorbed inclusion whose faithful antecedent names no class anywhere —
    ///   `⊤ ⊑ D` (an empty body), `rdfs:range` and `rdfs:domain` (`r(x,y)`), `{a} ⊑ D`
    ///   (`denotes_a(x)`). One clause per such axiom.
    ///
    /// Everything else is re-rooted at a named class by
    /// [`crate::owl_dl::absorb`] and reaches [`Self::by_trigger`]: an `∃r.C ⊑ D` with a named
    /// filler is authored `C(x₀) ∧ r⁻(x₀,x₁) → D(x₁)`, so the common absorbed shape does NOT
    /// land here. The count is therefore bounded by the ontology's AXIOM count — but the
    /// per-node cost of one of them is NOT a free failure. A single
    /// [`Graph::neighbors`](crate::owl_dl::graph::Graph::neighbors) call resolves the role's
    /// achiever closure (memoized per role for the run, see
    /// [`Graph::achiever_cache`](crate::owl_dl::graph::Graph); a role queried for the first
    /// time still walks the role hierarchy to build it) and then scans every edge the
    /// completion graph holds, whether or not one matches — a node with no such edge pays
    /// that scan in full before `neighbors` can report it has nothing. What the bound above
    /// buys is a per-node cost independent of the ontology's CONCEPT count, not a per-node
    /// cost of zero.
    ///
    /// An edge-driven index over these would need a role's ACHIEVERS — its sub-roles and its
    /// inverse partners — resolved to find the clauses an edge could trigger, which is what
    /// `neighbors` already does (and now caches). The bound above is what makes building a
    /// second, edge-keyed index unnecessary rather than merely unmeasured.
    untriggered: Vec<usize>,
    /// Whether each clause is TBox-DERIVED, by clause index.
    ///
    /// A TBox clause is scoped to the OBJECT domain: a general concept inclusion quantifies
    /// over `owl:Thing`, and a literal value is not in it, so firing one at a node of `Δ_D`
    /// would derive a consequence the ontology does not state. The concept-structure clauses
    /// carry no such flag — `C ⊓ ¬C ⊑ ⊥` and the decomposition of a concept a node's label
    /// actually carries are statements about that concept, valid over either domain.
    tbox: Vec<bool>,
}

impl ClauseSet {
    /// The clause at `index`.
    pub(crate) fn clause(&self, index: usize) -> &DlClause {
        &self.clauses[index]
    }

    /// How many clauses the knowledge base produced.
    #[cfg(test)]
    pub(crate) fn count(&self) -> usize {
        self.clauses.len()
    }

    /// The clauses `concept` can make applicable, in derivation order.
    pub(crate) fn triggered_by(&self, concept: u32) -> &[usize] {
        self.by_trigger
            .get(&concept)
            .map_or(&[] as &[usize], Vec::as_slice)
    }

    /// The clauses no concept triggers — tried at every node.
    pub(crate) fn untriggered(&self) -> &[usize] {
        &self.untriggered
    }

    /// Every clause with the given head form, by index — the inventory a test reads to see
    /// WHICH forms an ontology actually produced.
    #[cfg(test)]
    pub(crate) fn with_head_form(&self, form: HeadForm) -> Vec<usize> {
        (0..self.clauses.len())
            .filter(|&index| self.clauses[index].head_form() == form)
            .collect()
    }

    /// Whether the clause at `index` is TBox-derived, and so scoped to the object domain.
    pub(crate) fn is_tbox(&self, index: usize) -> bool {
        self.tbox[index]
    }

    /// Record one clause, indexing it by its trigger.
    fn push(&mut self, clause: DlClause) {
        self.record(clause, false);
    }

    /// Record one TBox-derived clause — see [`ClauseSet::tbox`].
    fn push_tbox(&mut self, clause: DlClause) {
        self.record(clause, true);
    }

    /// Record one clause, indexing it by its trigger and noting its provenance.
    fn record(&mut self, clause: DlClause, tbox: bool) {
        let index = self.clauses.len();
        match clause.trigger() {
            Some(concept) => self.by_trigger.entry(concept).or_default().push(index),
            None => self.untriggered.push(index),
        }
        self.clauses.push(clause);
        self.tbox.push(tbox);
    }

    /// Record `body → head` where the body opens with the trigger concept `trigger` on
    /// variable 0.
    fn push_triggered(&mut self, trigger: u32, rest: Vec<BodyAtom>, head: Vec<Vec<HeadAtom>>) {
        let mut body = vec![BodyAtom::Concept {
            var: 0,
            concept: trigger,
        }];
        body.extend(rest);
        self.push(DlClause { body, head });
    }
}

/// Derive the DL-clause set of `kb`.
///
/// The knowledge base's concept table must be finalized ([`Kb::finalize`]): the
/// complementary-pair clauses read the negation cache, and a concept whose negation is not
/// cached simply produces no such clause rather than a panic — one fewer derived clash, never
/// an invented one.
pub(crate) fn derive(kb: &Kb) -> ClauseSet {
    let mut out = ClauseSet {
        clauses: Vec::new(),
        by_trigger: BTreeMap::new(),
        untriggered: Vec::new(),
        tbox: Vec::new(),
    };
    for id in 0..kb.table.len() {
        let id = u32::try_from(id).expect("concept count fits u32");
        derive_concept(kb, id, &mut out);
        // `C ⊓ ¬C ⊑ ⊥`, valid for every concept and emitted once per PAIR — the clash the
        // incumbent detects by scanning a label for a complementary pair, here as the clause
        // that says why it is one.
        if let Some(negation) = kb.table.negation(id)
            && id < negation
        {
            out.push_triggered(
                id,
                vec![BodyAtom::Concept {
                    var: 0,
                    concept: negation,
                }],
                Vec::new(),
            );
        }
    }
    // The absorbed TBox, verbatim: [`crate::owl_dl::absorb`] has already decided each
    // inclusion's guard, so emission is a translation and not a second place the encoding is
    // chosen. Marked as TBox-derived, which is what scopes it to the OBJECT domain — a
    // general concept inclusion quantifies over `owl:Thing`, and no literal value is in it.
    for clause in &kb.absorbed {
        out.push_tbox(DlClause {
            body: clause.body.clone(),
            head: vec![vec![HeadAtom::Concept {
                var: clause.head_var,
                concept: clause.head,
            }]],
        });
    }
    // The two role axioms that constrain EDGES rather than labels, so neither can be
    // internalized as a general concept inclusion and neither has a concept to trigger it.
    // Asymmetry with `y = x` is the self-loop case, which is how it subsumes irreflexivity.
    for &property in &kb.asymmetric {
        let role = Role::Named(property);
        out.push(DlClause {
            body: vec![
                BodyAtom::Role {
                    from: 0,
                    to: 1,
                    role,
                },
                BodyAtom::Role {
                    from: 1,
                    to: 0,
                    role,
                },
            ],
            head: Vec::new(),
        });
    }
    for &(left, right) in &kb.disjoint_roles {
        // The pair is held in both orders so a lookup needs no normalization; one clause per
        // unordered pair is enough, because the body is symmetric in the two roles.
        if left > right {
            continue;
        }
        out.push(DlClause {
            body: vec![
                BodyAtom::Role {
                    from: 0,
                    to: 1,
                    role: Role::Named(left),
                },
                BodyAtom::Role {
                    from: 0,
                    to: 1,
                    role: Role::Named(right),
                },
            ],
            head: Vec::new(),
        });
    }
    out
}

/// Emit the clauses one concept id forces, per the table in the [module docs](self).
fn derive_concept(kb: &Kb, id: u32, out: &mut ClauseSet) {
    match *kb.table.decomp(id) {
        Decomp::Bottom => out.push_triggered(id, Vec::new(), Vec::new()),
        Decomp::And(ref children) => {
            let conjuncts: Vec<HeadAtom> = children
                .iter()
                .map(|&concept| HeadAtom::Concept { var: 0, concept })
                .collect();
            if !conjuncts.is_empty() {
                out.push_triggered(id, Vec::new(), vec![conjuncts]);
            }
        }
        Decomp::Or(ref children) => {
            // AUTHORED in search order, not in the interner's: see [`Kb::order_disjuncts`] for
            // why the canonical member order and this one are different orders with different
            // jobs. The `⊔`-rule branches in the order it finds here, so this is where a
            // terminology's cheap alternatives are put first — once, in front of every search
            // over the clause set, rather than re-decided at each case split.
            let disjuncts: Vec<Vec<HeadAtom>> = kb
                .order_disjuncts(children)
                .into_iter()
                .map(|concept| vec![HeadAtom::Concept { var: 0, concept }])
                .collect();
            // An EMPTY disjunction is `⊥`, which is exactly the empty head — so the degenerate
            // `⊔` of nothing needs no special case.
            out.push_triggered(id, Vec::new(), disjuncts);
        }
        Decomp::Some(role, filler) => out.push_triggered(
            id,
            Vec::new(),
            vec![vec![HeadAtom::AtLeast {
                var: 0,
                n: 1,
                role,
                filler,
            }]],
        ),
        // `≥0 r.C` IS `⊤`: it forces nothing, so it produces no clause rather than a clause
        // with a vacuous head.
        Decomp::Min(0, _, _) => {}
        Decomp::Min(n, role, filler) => out.push_triggered(
            id,
            Vec::new(),
            vec![vec![HeadAtom::AtLeast {
                var: 0,
                n,
                role,
                filler,
            }]],
        ),
        Decomp::All(role, filler) => out.push_triggered(
            id,
            vec![BodyAtom::Role {
                from: 0,
                to: 1,
                role,
            }],
            vec![vec![HeadAtom::Concept {
                var: 1,
                concept: filler,
            }]],
        ),
        Decomp::Max(n, role, filler) => derive_at_most(kb, id, n, role, filler, out),
        Decomp::SelfRestriction(role) => {
            out.push_triggered(
                id,
                Vec::new(),
                vec![vec![HeadAtom::SelfLoop { var: 0, role }]],
            );
        }
        Decomp::NegSelfRestriction(role) => out.push_triggered(
            id,
            vec![BodyAtom::Role {
                from: 0,
                to: 0,
                role,
            }],
            Vec::new(),
        ),
        Decomp::Nominal(ref members) => {
            let disjuncts: Vec<Vec<HeadAtom>> = members
                .iter()
                .map(|&individual| vec![HeadAtom::EqualIndividual { var: 0, individual }])
                .collect();
            out.push_triggered(id, Vec::new(), disjuncts);
        }
        Decomp::NegNominal(ref members) => {
            for &individual in members {
                out.push_triggered(
                    id,
                    vec![BodyAtom::Denotes { var: 0, individual }],
                    Vec::new(),
                );
            }
        }
        // Atomic leaves force nothing on their own: a named class is opaque, and a data range
        // is decided by the concrete domain rather than by a clause. See the module docs.
        Decomp::Top | Decomp::Named | Decomp::NegNamed | Decomp::Data(_) | Decomp::NegData(_) => {}
    }
}

/// Emit the two clauses of `≤n r.C`.
///
/// The first DECIDES the filler on every `r`-neighbour, and it is not an optimization: a
/// counting clause can only count what the labels say, so without it a neighbour whose
/// `C`-membership is undetermined would be silently counted as a non-member and the
/// restriction would be satisfiable by not deciding. It is the standard structural
/// transformation of a concept occurring NEGATIVELY (a `≤` restriction is antitone in its
/// filler) specialized to this shape, and it is exactly what the incumbent's `≤`-choose rule
/// does.
///
/// The second is the restriction itself: `n + 1` pairwise-distinct `C`-successors are one too
/// many, so at least two of them must be the same element. Every pair is a branch; a state
/// where every pair is already recorded `≠` has no branch left and closes — which is how a
/// `≤n` violation becomes a clash without a second clash rule to state it.
fn derive_at_most(kb: &Kb, id: u32, n: u32, role: Role, filler: u32, out: &mut ClauseSet) {
    if let Some(negated) = kb.table.negation(filler) {
        // The two alternatives are ordered like any other authored case split: deciding a
        // neighbour's membership one way may force it to mint successors of its own, and the
        // way that does not is the one to try first.
        let decided: Vec<Vec<HeadAtom>> = kb
            .order_disjuncts(&[filler, negated])
            .into_iter()
            .map(|concept| vec![HeadAtom::Concept { var: 1, concept }])
            .collect();
        out.push_triggered(
            id,
            vec![BodyAtom::Role {
                from: 0,
                to: 1,
                role,
            }],
            decided,
        );
    }
    // The counting clause, schematic in `n + 1`: one body atom for the counted successors and
    // one head atom for the disjunction of their pairs. See `BodyAtom::Successors` for why it
    // is not written out — an `owl:maxCardinality "300"` would otherwise put 45_150 head
    // disjuncts in the clause set for a bound no completion graph in this crate will reach.
    let count = n + 1;
    out.push_triggered(
        id,
        vec![BodyAtom::Successors {
            role,
            filler,
            first: 1,
            count,
        }],
        vec![vec![HeadAtom::EqualSomePair { first: 1, count }]],
    );
}

#[cfg(test)]
mod tests {
    use purrdf_datalog::clause::HeadForm;

    use super::{BodyAtom, DlClause, HeadAtom, derive};
    use crate::owl_dl::Kb;
    use crate::owl_dl::concept::{Concept, Role};

    /// A knowledge base holding one asserted concept, finalized.
    fn kb_of(concept: Concept) -> Kb {
        let mut kb = Kb::empty();
        let id = kb.table.intern(concept);
        kb.abox_types.push((1, id));
        kb.individuals.insert(1);
        kb.finalize();
        kb
    }

    /// Every derived clause is matchable: each variable is bound before use, and only
    /// variable 0 comes from outside. A clause that failed this would make the matcher's
    /// left-to-right join plan incomplete.
    #[test]
    fn every_derived_clause_is_matchable() {
        let concepts = vec![
            Concept::And(vec![Concept::Named(10), Concept::Named(11)]),
            Concept::Or(vec![Concept::Named(10), Concept::Named(11)]),
            Concept::Some(Role::Named(20), Box::new(Concept::Named(10))),
            Concept::All(Role::Inv(20), Box::new(Concept::Named(10))),
            Concept::Min(3, Role::Named(20), Box::new(Concept::Named(10))),
            Concept::Max(2, Role::Named(20), Box::new(Concept::Named(10))),
            Concept::Nominal(vec![30, 31]),
            Concept::Not(Box::new(Concept::Nominal(vec![30]))),
            Concept::SelfRestriction(Role::Named(20)),
            Concept::Not(Box::new(Concept::SelfRestriction(Role::Named(20)))),
        ];
        for concept in concepts {
            let mut kb = kb_of(concept.clone());
            kb.asymmetric.insert(21);
            kb.disjoint_roles.insert((20, 21));
            kb.disjoint_roles.insert((21, 20));
            let clauses = derive(&kb);
            for index in 0..clauses.count() {
                let clause = clauses.clause(index);
                assert!(
                    clause.is_matchable(),
                    "{concept:?} derived an unmatchable clause: {clause:?}"
                );
            }
        }
    }

    /// A DISJUNCTION derives a clause whose head form is [`HeadForm::Disjunctive`] — the form
    /// `purrdf-datalog` classifies and refuses, and which the hypertableau case-splits over.
    #[test]
    fn a_disjunction_derives_a_disjunctive_head() {
        let kb = kb_of(Concept::Or(vec![Concept::Named(10), Concept::Named(11)]));
        let clauses = derive(&kb);
        let disjunctive = clauses.with_head_form(HeadForm::Disjunctive);
        assert_eq!(
            disjunctive.len(),
            1,
            "one disjunction, one disjunctive clause"
        );
        let clause = clauses.clause(disjunctive[0]);
        assert_eq!(clause.head.len(), 2, "two disjuncts");
        assert!(
            clause
                .head
                .iter()
                .all(|disjunct| matches!(disjunct.as_slice(), [HeadAtom::Concept { var: 0, .. }])),
            "each disjunct adds one concept to the trigger node: {clause:?}"
        );
    }

    /// `≤n r.C` derives BOTH clauses: the choose disjunction that decides the filler on each
    /// neighbour, and the counting disjunction over `n + 1` interchangeable successors whose
    /// disjuncts are equalities.
    #[test]
    fn an_at_most_restriction_derives_a_choose_clause_and_a_counting_clause() {
        let kb = kb_of(Concept::Max(
            2,
            Role::Named(20),
            Box::new(Concept::Named(10)),
        ));
        let clauses = derive(&kb);
        let counting = (0..clauses.count())
            .map(|index| clauses.clause(index))
            .find(|clause| {
                clause
                    .head
                    .iter()
                    .flatten()
                    .any(|atom| matches!(atom, HeadAtom::EqualSomePair { .. }))
            })
            .expect("a counting clause");
        // Disjunctive because of the SCHEMATIC atom, not because the head vector is long: the
        // disjunction over the tuple's pairs is expanded at match time.
        assert_eq!(counting.head_form(), HeadForm::Disjunctive);
        assert_eq!(counting.arity(), 4, "x plus three counted successors");
        assert_eq!(counting.head.len(), 1, "one schematic disjunction");
        assert!(matches!(
            counting.head[0][0],
            HeadAtom::EqualSomePair { first: 1, count: 3 }
        ));
        assert!(matches!(
            counting.body[1],
            BodyAtom::Successors {
                first: 1,
                count: 3,
                ..
            }
        ));

        let choose = (0..clauses.count())
            .map(|index| clauses.clause(index))
            .find(|clause| {
                clause.head.len() == 2
                    && clause
                        .head
                        .iter()
                        .flatten()
                        .all(|atom| matches!(atom, HeadAtom::Concept { var: 1, .. }))
            })
            .expect("a choose clause");
        assert_eq!(choose.head_form(), HeadForm::Disjunctive);
        assert_eq!(choose.arity(), 2, "x and one neighbour");
    }

    /// `≥n r.C` and `∃r.C` derive the EXISTENTIAL head form; `≥0 r.C` derives no clause at
    /// all, because it is `⊤`.
    #[test]
    fn a_min_restriction_derives_an_existential_head() {
        let kb = kb_of(Concept::Min(2, Role::Named(20), Box::new(Concept::Top)));
        let clauses = derive(&kb);
        let existential = clauses.with_head_form(HeadForm::Existential);
        assert_eq!(existential.len(), 1);
        assert!(matches!(
            clauses.clause(existential[0]).head[0][0],
            HeadAtom::AtLeast { n: 2, .. }
        ));

        let zero = kb_of(Concept::Min(0, Role::Named(20), Box::new(Concept::Top)));
        let clauses = derive(&zero);
        assert!(
            clauses.with_head_form(HeadForm::Existential).is_empty(),
            "≥0 r.C is ⊤ and forces nothing"
        );
    }

    /// A LARGE cardinality is ordinary OWL 2, and the clause set holds it in CONSTANT size.
    ///
    /// `owl:maxCardinality "300"` is a legal restriction. Written out, its counting clause
    /// would carry 602 body atoms and 45_150 head disjuncts — quadratic in a number the
    /// ontology chose, for a bound the completion graph may never come near — and an earlier
    /// revision of this module panicked on it outright, because the variable index did not fit.
    /// Schematic, it is one body atom and one head atom, whatever `n` is.
    #[test]
    fn a_large_cardinality_derives_one_schematic_clause() {
        let kb = kb_of(Concept::Max(
            300,
            Role::Named(20),
            Box::new(Concept::Named(10)),
        ));
        let clauses = derive(&kb);
        let counting = (0..clauses.count())
            .map(|index| clauses.clause(index))
            .find(|clause| {
                clause
                    .head
                    .iter()
                    .flatten()
                    .any(|atom| matches!(atom, HeadAtom::EqualSomePair { .. }))
            })
            .expect("a counting clause");
        assert_eq!(counting.head_form(), HeadForm::Disjunctive);
        assert_eq!(counting.body.len(), 2, "the trigger and one schematic atom");
        assert_eq!(counting.head.len(), 1, "one schematic disjunction");
        assert_eq!(counting.arity(), 302);
        assert!(counting.is_matchable());
    }

    /// A conjunction derives ONE conjunctive-head clause — not one clause per conjunct — so
    /// the whole conjunction is asserted in one derivation step.
    #[test]
    fn a_conjunction_derives_one_conjunctive_head() {
        let kb = kb_of(Concept::And(vec![Concept::Named(10), Concept::Named(11)]));
        let clauses = derive(&kb);
        let conjunctive = clauses.with_head_form(HeadForm::Conjunctive);
        assert_eq!(conjunctive.len(), 1);
        assert_eq!(clauses.clause(conjunctive[0]).head[0].len(), 2);
    }

    /// A complementary pair, `⊥`, a negated self restriction, a negated nominal and the two
    /// role axioms all derive the INCONSISTENCY head form — the empty head, which is `false`.
    #[test]
    fn the_clash_clauses_have_empty_heads() {
        let mut kb = kb_of(Concept::Not(Box::new(Concept::SelfRestriction(
            Role::Named(20),
        ))));
        kb.asymmetric.insert(21);
        let clauses = derive(&kb);
        let inconsistency = clauses.with_head_form(HeadForm::Inconsistency);
        assert!(
            inconsistency.len() >= 2,
            "the negated self restriction and the asymmetry axiom, at least: {inconsistency:?}"
        );
        assert!(
            inconsistency
                .iter()
                .all(|&index| clauses.clause(index).head.is_empty())
        );
        // The asymmetry clause is the untriggered one: it opens with a role atom.
        assert_eq!(clauses.untriggered().len(), 1);
        let asymmetry = clauses.clause(clauses.untriggered()[0]);
        assert_eq!(asymmetry.trigger(), None);
        assert_eq!(asymmetry.arity(), 2);
        assert!(asymmetry.head.is_empty());
    }

    /// A SINGLETON nominal derives an atomic head — a deterministic identification — while a
    /// multi-member one derives a disjunction over its members. That difference is the whole
    /// of the `o`-rule, and it never compares names.
    #[test]
    fn a_nominal_derives_an_identification_per_member() {
        let one = kb_of(Concept::Nominal(vec![30]));
        let clauses = derive(&one);
        let identifications = clauses
            .with_head_form(HeadForm::Atomic)
            .into_iter()
            .filter(|&index| {
                matches!(
                    clauses.clause(index).head[0][0],
                    HeadAtom::EqualIndividual { .. }
                )
            })
            .count();
        assert_eq!(identifications, 1, "one member, one deterministic merge");

        let two = kb_of(Concept::Nominal(vec![30, 31]));
        let clauses = derive(&two);
        let disjunctive: Vec<usize> = clauses
            .with_head_form(HeadForm::Disjunctive)
            .into_iter()
            .filter(|&index| {
                clauses
                    .clause(index)
                    .head
                    .iter()
                    .flatten()
                    .all(|atom| matches!(atom, HeadAtom::EqualIndividual { .. }))
            })
            .collect();
        assert_eq!(disjunctive.len(), 1, "two members, one branch of two");
        assert_eq!(clauses.clause(disjunctive[0]).head.len(), 2);
    }

    /// The absorbed TBox becomes an atomic-head clause triggered by the named class, which is
    /// what keeps a terminology from branching on every node.
    #[test]
    fn the_absorbed_tbox_becomes_triggered_atomic_clauses() {
        let mut kb = Kb::empty();
        kb.push_gci(Concept::Named(10), Concept::Named(11));
        kb.finalize();
        let named = kb.table.intern(Concept::Named(10));
        let clauses = derive(&kb);
        let triggered = clauses.triggered_by(named);
        assert!(
            triggered.iter().any(|&index| {
                let clause = clauses.clause(index);
                clause.head_form() == HeadForm::Atomic
                    && matches!(clause.head[0][0], HeadAtom::Concept { var: 0, .. })
            }),
            "A ⊑ D is a clause triggered by A: {triggered:?}"
        );
    }

    /// An absorbed `∃r.C ⊑ D` is TRIGGERED, not untriggered — the whole point of authoring it
    /// re-rooted at its filler. It is the shape a terminology states most often after the
    /// named-class one, and head-first it would be retried at every node of every round.
    #[test]
    fn an_absorbed_existential_antecedent_is_triggered_by_its_filler() {
        let mut kb = Kb::empty();
        kb.push_gci(
            Concept::Some(Role::Named(20), Box::new(Concept::Named(10))),
            Concept::Named(11),
        );
        kb.finalize();
        let filler = kb.table.intern(Concept::Named(10));
        let clauses = derive(&kb);
        assert!(
            clauses.untriggered().is_empty(),
            "∃r.C ⊑ D must not land in the untriggered set: {:?}",
            clauses.untriggered()
        );
        assert!(
            clauses.triggered_by(filler).iter().any(|&index| {
                matches!(
                    clauses.clause(index).body.as_slice(),
                    [
                        BodyAtom::Concept { var: 0, .. },
                        BodyAtom::Role {
                            from: 0,
                            to: 1,
                            role: Role::Inv(20)
                        },
                    ]
                )
            }),
            "the clause is triggered by the FILLER and walks back through r⁻"
        );
    }

    /// THE UNTRIGGERED INVARIANT, as a population rather than a promise.
    ///
    /// Every clause that no concept triggers is tried at every node of every round, so what
    /// may land there is a bounded, enumerable set — the two role axioms, and the absorbed
    /// inclusions whose faithful antecedent names no class anywhere. This knowledge base
    /// states one of each of the five, and the count is exactly five: nothing that could have
    /// been re-rooted onto a trigger is here, and in particular the concept table's own
    /// clauses — one per interned concept, the population that scales — are all triggered.
    #[test]
    fn only_the_role_axioms_and_the_class_free_guards_are_untriggered() {
        let mut kb = Kb::empty();
        // `⊤ ⊑ A` — an empty body.
        kb.push_gci(Concept::Top, Concept::Named(10));
        // `rdfs:range` — `r(x,y) → A(y)`.
        kb.push_gci(
            Concept::Top,
            Concept::All(Role::Named(20), Box::new(Concept::Named(10))),
        );
        // `rdfs:domain` — `r(x,y) → A(x)`.
        kb.push_gci(
            Concept::Some(Role::Named(20), Box::new(Concept::Top)),
            Concept::Named(10),
        );
        // A nominal guard — `denotes_a(x) → A(x)`.
        kb.push_gci(Concept::Nominal(vec![30]), Concept::Named(10));
        // …beside inclusions that MUST be triggered rather than untriggered.
        kb.push_gci(Concept::Named(11), Concept::Named(10));
        kb.push_gci(
            Concept::Some(Role::Named(20), Box::new(Concept::Named(11))),
            Concept::Named(10),
        );
        kb.asymmetric.insert(21);
        kb.disjoint_roles.insert((20, 21));
        kb.disjoint_roles.insert((21, 20));
        kb.finalize();
        let clauses = derive(&kb);
        assert_eq!(
            clauses.untriggered().len(),
            6,
            "four class-free guards and the two role axioms: {:?}",
            clauses
                .untriggered()
                .iter()
                .map(|&index| clauses.clause(index))
                .collect::<Vec<&DlClause>>()
        );
        assert!(
            clauses.untriggered().len() < clauses.count(),
            "the population that scales with the concept table is the TRIGGERED one"
        );
    }
}
