// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The consequence-based classifier: the WHOLE subsumption relation in ONE saturation.
//!
//! # Why this is not a loop over the tableau
//!
//! `KB ⊨ C ⊑ D` can always be decided by refutation — negate the superclass, run a
//! completion, see whether every branch clashes — and a classifier built that way costs one
//! tableau run per ORDERED pair of named classes. That is the wrong algorithm being paid for
//! as if it were a slow one: a consequence-based (saturation) calculus derives every
//! entailed subsumption at once by closing a rule table to its least fixpoint, which is why
//! the published consequence-based classifiers handle terminologies with hundreds of
//! thousands of axioms while a subsumption-test-per-pair procedure cannot.
//!
//! So this module derives, and [`crate::reasoner::classify`] only refutes what the
//! derivation did not reach — and only when the ontology lies outside the fragment this
//! calculus is complete for.
//!
//! # The rule table
//!
//! The derived relations are read as ENTAILMENTS, which is what makes each rule's soundness
//! checkable by eye:
//!
//! * `S(X) ∋ C` means `KB ⊨ X ⊑ C`. `X` is a *context*: a concept id treated as a name.
//! * `R(r) ∋ (X, Y)` means `KB ⊨ X ⊑ ∃r.Y`.
//!
//! The normalized axiom forms are `C ⊑ D`, `C₁ ⊓ C₂ ⊑ D`, `C ⊑ ∃r.D` and `∃r.C ⊑ D`, and
//! the closure rules over them are:
//!
//! | rule | premises | conclusion | why it is sound |
//! |---|---|---|---|
//! | `R0` | a context `X` exists | `X, ⊤ ∈ S(X)` | `X ⊑ X` and `X ⊑ ⊤` hold in every interpretation |
//! | `R1` | `C ∈ S(X)`, `C ⊑ D` | `D ∈ S(X)` | `X ⊑ C ⊑ D` |
//! | `R2` | `C₁, C₂ ∈ S(X)`, `C₁ ⊓ C₂ ⊑ D` | `D ∈ S(X)` | `X ⊑ C₁ ⊓ C₂ ⊑ D` |
//! | `R3` | `C ∈ S(X)`, `C ⊑ ∃r.D` | `(X, D) ∈ R(r)` | `X ⊑ C ⊑ ∃r.D` |
//! | `R4` | `(X, Y) ∈ R(r)`, `C ∈ S(Y)`, `∃r.C ⊑ D` | `D ∈ S(X)` | `X ⊑ ∃r.Y ⊑ ∃r.C ⊑ D` |
//! | `R5` | `(X, Y) ∈ R(r)`, `r ⊑ s` | `(X, Y) ∈ R(s)` | `∃r.Y ⊑ ∃s.Y` |
//! | `R6` | `(X, Y), (Y, Z) ∈ R(r)`, `r` transitive | `(X, Z) ∈ R(r)` | `∃r.∃r.Z ⊑ ∃r.Z` |
//! | `R7` | `(X, Y) ∈ R(r)`, `⊥ ∈ S(Y)` | `⊥ ∈ S(X)` | `∃r.⊥ ⊑ ⊥` |
//!
//! `R5` is folded into edge insertion — an edge is recorded for the role AND for every
//! super-role, so the role hierarchy is closed once rather than re-derived per edge — and
//! `R0` is folded into context creation. Everything else is a distinct arm of
//! [`Engine::on_concept`] / [`Engine::on_edge`], named after its row above.
//!
//! # Every rule is sound, so every DERIVED pair is entailed
//!
//! That is the load-bearing asymmetry. A derivation is a proof, so a pair this module
//! reports is a subsumption the tableau would also confirm; the calculus may therefore
//! carry deliberately WEAKENING readings of non-fragment constructs (`≥n r.C ⊑ ∃r.C`,
//! `Cᵢ ⊑ C₁ ⊔ … ⊔ Cₙ`, `C ⊓ ¬C ⊑ ⊥`) to derive more without ever risking a wrong answer.
//! What those readings cost is COMPLETENESS, and completeness is reported separately by
//! [`Taxonomy::is_complete`] rather than assumed.
//!
//! # The fragment this calculus is COMPLETE for
//!
//! `EL⁺⁺`-shaped Horn terminologies. Precisely, [`Taxonomy::is_complete`] answers `true`
//! exactly when all of the following hold of the knowledge base:
//!
//! 1. **Every general concept inclusion is `EL`-shaped in its polarity.** For each
//!    `sub ⊑ sup`, `sub` is built only from `⊤`, `⊥`, named classes, `⊓`, `∃r.C` and
//!    `≥0/≥1 r.C` over NAMED roles, and `sup` is built from those plus a negated named
//!    class `¬A` at any positive position. `¬A` is admissible there because a name `N`
//!    constrained only by `N ⊓ A ⊑ ⊥` and occurring only positively can be enlarged to `¬A`
//!    in any model without breaking an axiom, so the translation preserves every
//!    subsumption between named classes — which is exactly how a disjointness axiom stays
//!    inside the fragment.
//! 2. **No inverse role, no disjoint role pair, no asymmetric role.** Each of the three
//!    invalidates the canonical-model argument the completeness of the calculus rests on:
//!    an inverse turns `∀`-like propagation back along an edge (the `ELI` jump the one
//!    -context-per-concept shape cannot follow), and a disjointness or asymmetry constraint
//!    can be violated by the canonical model's own edges without any subsumption changing.
//! 3. **No nominal in the terminology** — implied by (1), since `Nominal` is not an `EL`
//!    constructor. This is also what makes a TBox-only saturation legitimate at all: with
//!    nominals an ASSERTION changes the class hierarchy (`Only ≡ {alice}` with
//!    `alice : Female` entails `Only ⊑ Female`), while without them a model of the TBox and
//!    a model of the whole knowledge base can be joined by disjoint union — every axiom
//!    form left in the fragment is a local statement that survives that union — so the ABox
//!    constrains no subsumption between named classes.
//!
//! Outside the fragment the derivation is still sound, so a caller may report every derived
//! pair as entailed; what it may NOT do is read "not derived" as "refuted". See
//! [`crate::reasoner::classify`] for the residual tableau runs that settle those pairs, and
//! [`crate::reasoner::DlCertificate::decisions`] for the count, which is what makes the
//! saving a measurement rather than a claim.
//!
//! # A consistent knowledge base is a precondition
//!
//! An inconsistent knowledge base entails every subsumption, and this saturation would not
//! derive them: it reads the TBox and never sees the ABox clash. Both callers therefore
//! establish consistency with the tableau BEFORE saturating —
//! [`crate::reasoner::Reasoner::classify`] through its session, and
//! [`crate::materialize_dl_reported`] through its own up-front check — and an inconsistent
//! ontology is refused there rather than misclassified here.
//!
//! # Determinism
//!
//! Concept ids are assigned in parse order; every axiom index is a `BTreeMap` over those
//! ids; the work queue is a FIFO seeded in the caller's context order; and the answer is a
//! least fixpoint, so it does not depend on the queue order at all. Nothing is read out of
//! a hash map and nothing consults a clock.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::owl_dl::Kb;
use crate::owl_dl::concept::{Decomp, Role};

/// The subsumptions one saturation derived, plus whether it was COMPLETE for the ontology.
pub(crate) struct Taxonomy {
    /// Context concept id → its slot in [`Self::supers`].
    slot_of: BTreeMap<u32, usize>,
    /// Per context slot, the concept ids it was derived to be subsumed by — `S(X)`.
    supers: Vec<BTreeSet<u32>>,
    /// The `⊤` concept id, which subsumes everything.
    top: u32,
    /// The `⊥` concept id, which is subsumed by everything.
    bottom: u32,
    /// Whether the knowledge base lies in the fragment this calculus is complete for.
    complete: bool,
}

impl Taxonomy {
    /// Whether the calculus is COMPLETE for this knowledge base — i.e. whether a pair it did
    /// not derive is genuinely not entailed.
    ///
    /// See the [module docs](self) for the exact fragment condition. A `false` here does not
    /// weaken any derived answer; it says only that the underivable pairs still need
    /// deciding some other way.
    pub(crate) const fn is_complete(&self) -> bool {
        self.complete
    }

    /// Whether `KB ⊨ sub ⊑ sup` was DERIVED.
    ///
    /// Sound in both the trivial and the derived cases, so a `true` may be reported as an
    /// entailment without a refutation. A `false` means "this saturation did not derive it",
    /// which is a refutation only when [`Self::is_complete`] holds.
    pub(crate) fn derives(&self, sub: u32, sup: u32) -> bool {
        // `X ⊑ X` and `X ⊑ ⊤` are theorems of the logic rather than facts about an
        // ontology, so they are answered without consulting the saturation — which also
        // means a class the caller never seeded still gets them right.
        if sub == sup || sup == self.top {
            return true;
        }
        self.slot_of.get(&sub).is_some_and(|&slot| {
            let derived = &self.supers[slot];
            // An empty class is subsumed by everything, so `⊥ ∈ S(sub)` settles every
            // superclass at once instead of being derived pair by pair.
            derived.contains(&self.bottom) || derived.contains(&sup)
        })
    }
}

/// Derive the subsumption relation over `seeds` (context concept ids) in one saturation.
///
/// `seeds` fixes the contexts the answer is READ from; the saturation creates whatever
/// further contexts the existential rule needs, so a filler concept is reasoned about even
/// when the caller never named it.
pub(crate) fn saturate(kb: &Kb, seeds: &[u32]) -> Taxonomy {
    let normalized = Normalized::of_kb(kb);
    let mut engine = Engine::new(&normalized, kb.top, kb.bottom);
    for &seed in seeds {
        engine.context(seed);
    }
    engine.run();
    Taxonomy {
        slot_of: engine.slot_of,
        supers: engine.supers,
        top: kb.top,
        bottom: kb.bottom,
        complete: normalized.complete,
    }
}

/// The knowledge base as the four normalized axiom forms the rule table joins over.
struct Normalized {
    /// `C ⊑ D`, indexed by `C`.
    sub: BTreeMap<u32, Vec<u32>>,
    /// `C₁ ⊓ C₂ ⊑ D`, indexed by EACH conjunct as `conjunct → [(other conjunct, D)]`, so
    /// `R2` fires from whichever of the two entered `S(X)` last.
    conj: BTreeMap<u32, Vec<(u32, u32)>>,
    /// `C ⊑ ∃r.D`, indexed by `C` as `C → [(r, D)]`.
    ex_right: BTreeMap<u32, Vec<(u32, u32)>>,
    /// `∃r.C ⊑ D`, indexed by `(r, C)`.
    ex_left: BTreeMap<(u32, u32), Vec<u32>>,
    /// Role term id → itself and every super-role, reflexive-transitively closed, so `R5`
    /// is one lookup at edge-insertion time.
    role_supers: BTreeMap<u32, Vec<u32>>,
    /// Role term ids whose composition with themselves is subsumed by themselves.
    transitive: BTreeSet<u32>,
    /// Whether the rule table captures the knowledge base exactly — see
    /// [`Taxonomy::is_complete`].
    complete: bool,
}

impl Normalized {
    /// Normalize `kb` into the rule table's axiom forms, and decide the fragment question
    /// on the way past.
    fn of_kb(kb: &Kb) -> Self {
        let count = kb.table.len();
        let mut out = Self {
            sub: BTreeMap::new(),
            conj: BTreeMap::new(),
            ex_right: BTreeMap::new(),
            ex_left: BTreeMap::new(),
            role_supers: role_closure(kb),
            transitive: kb.transitive.clone(),
            complete: true,
        };

        // Names for the intermediate conjunctions an n-ary `⊓` binarizes into. They live
        // ABOVE the concept table's own id space rather than inside it, so normalization
        // needs no mutable access to the table and cannot perturb the ids the tableau reads.
        let mut next_name = u32::try_from(count).expect("concept count fits u32");

        for id in 0..count {
            let id = id as u32;
            match kb.table.decomp(id) {
                // `C₁ ⊓ … ⊓ Cₙ` in both directions: it is subsumed by each conjunct, and
                // holding all of them entails it. The fold is binarized left-associatively
                // because the rule table joins two conjuncts at a time.
                Decomp::And(children) => {
                    for &child in children {
                        out.push_sub(id, child);
                    }
                    match children.len() {
                        0 => out.push_sub(kb.top, id),
                        1 => out.push_sub(children[0], id),
                        _ => {
                            let mut left = children[0];
                            for (position, &right) in children.iter().enumerate().skip(1) {
                                let result = if position + 1 == children.len() {
                                    id
                                } else {
                                    let name = next_name;
                                    next_name += 1;
                                    name
                                };
                                out.push_conj(left, right, result);
                                left = result;
                            }
                        }
                    }
                }
                // A WEAKENING: each disjunct is subsumed by the disjunction. Sound, and the
                // only half of `⊔` a consequence-based calculus gets for free — which is
                // why disjunction is outside the fragment even though it derives something.
                Decomp::Or(children) => {
                    for &child in children {
                        out.push_sub(child, id);
                    }
                }
                Decomp::Some(Role::Named(role), filler) => {
                    out.push_ex_right(id, *role, *filler);
                    out.push_ex_left(*role, *filler, id);
                }
                // `≥0 r.C` IS `⊤`; `≥1 r.C` IS `∃r.C`; `≥n r.C` for `n ≥ 2` is only
                // WEAKENED to `∃r.C`, so the fold direction is deliberately absent.
                Decomp::Min(0, _, _) => {
                    out.push_sub(id, kb.top);
                    out.push_sub(kb.top, id);
                }
                Decomp::Min(1, Role::Named(role), filler) => {
                    out.push_ex_right(id, *role, *filler);
                    out.push_ex_left(*role, *filler, id);
                }
                Decomp::Min(_, Role::Named(role), filler) => {
                    out.push_ex_right(id, *role, *filler);
                }
                // A data range whose value set is PROVABLY empty is `⊥`, so a class the
                // terminology forces into one is derived empty rather than only refuted. The
                // converse — a non-empty range is not `⊤` — is deliberately absent: it says
                // nothing about the object domain at all.
                Decomp::Data(range) if kb.data_ranges.is_range_empty(*range) => {
                    out.push_sub(id, kb.bottom);
                }
                // Everything else is an opaque atomic name to this calculus: an inverse
                // role, a `∀`, a `≤n`, a nominal, a self restriction, a data range that is
                // not empty. Recording NO axiom for it loses derivations and invents none,
                // which is the sound direction.
                _ => {}
            }
            // `C ⊓ ¬C ⊑ ⊥` is valid for every concept, and it is what lets a disjointness
            // axiom written as `A ⊑ ¬B` close a class rather than sit inert.
            if let Some(negation) = kb.table.negation(id) {
                out.push_conj(id, negation, kb.bottom);
            }
        }

        for &(sub, sup) in &kb.tbox {
            out.push_sub(sub, sup);
        }
        out.complete = out.fragment_holds(kb);
        out
    }

    /// Record `sub ⊑ sup`.
    fn push_sub(&mut self, sub: u32, sup: u32) {
        self.sub.entry(sub).or_default().push(sup);
    }

    /// Record `left ⊓ right ⊑ result`, indexed by both conjuncts.
    fn push_conj(&mut self, left: u32, right: u32, result: u32) {
        self.conj.entry(left).or_default().push((right, result));
        if left != right {
            self.conj.entry(right).or_default().push((left, result));
        }
    }

    /// Record `sub ⊑ ∃role.filler`.
    fn push_ex_right(&mut self, sub: u32, role: u32, filler: u32) {
        self.ex_right.entry(sub).or_default().push((role, filler));
    }

    /// Record `∃role.filler ⊑ sup`.
    fn push_ex_left(&mut self, role: u32, filler: u32, sup: u32) {
        self.ex_left.entry((role, filler)).or_default().push(sup);
    }

    /// Whether `kb` lies in the fragment this calculus is complete for.
    ///
    /// See the [module docs](self) for the three conditions and the argument for each.
    fn fragment_holds(&self, kb: &Kb) -> bool {
        if !kb.inverses.is_empty() || !kb.disjoint_roles.is_empty() || !kb.asymmetric.is_empty() {
            return false;
        }
        let (as_sub, as_sup) = el_shapes(kb);
        kb.tbox
            .iter()
            .all(|&(sub, sup)| as_sub[sub as usize] && as_sup[sup as usize])
    }
}

/// Whether each concept id is `EL`-shaped as a SUBCLASS and as a SUPERCLASS.
///
/// Two vectors rather than one because the two positions admit different grammars: a
/// negated named class is admissible in a superclass and not in a subclass (see the
/// [module docs](self)). Computed in ascending id order, which is a valid bottom-up
/// traversal because [`crate::owl_dl::concept::ConceptTable`] interns a concept's children
/// before the concept itself, so every child id is strictly smaller than its parent's.
fn el_shapes(kb: &Kb) -> (Vec<bool>, Vec<bool>) {
    let count = kb.table.len();
    let mut as_sub = vec![false; count];
    let mut as_sup = vec![false; count];
    for id in 0..count {
        let (sub_ok, sup_ok) = match kb.table.decomp(id as u32) {
            Decomp::Top | Decomp::Bottom | Decomp::Named => (true, true),
            Decomp::NegNamed => (false, true),
            Decomp::And(children) => (
                children.iter().all(|&c| as_sub[c as usize]),
                children.iter().all(|&c| as_sup[c as usize]),
            ),
            // `≥0 r.C` is `⊤` whatever `C` is, and the rule table says exactly that, so it
            // is in the fragment without any condition on the filler.
            Decomp::Min(0, _, _) => (true, true),
            Decomp::Some(Role::Named(_), filler) | Decomp::Min(1, Role::Named(_), filler) => {
                (as_sub[*filler as usize], as_sup[*filler as usize])
            }
            _ => (false, false),
        };
        as_sub[id] = sub_ok;
        as_sup[id] = sup_ok;
    }
    (as_sub, as_sup)
}

/// Each role term id mapped to itself and every super-role, reflexive-transitively closed.
///
/// [`Kb::role_sub`] is indexed the other way round (super-role → sub-roles) because the
/// tableau propagates downward; the saturation propagates an edge UPWARD to every
/// super-role, so the closure is inverted here once rather than searched per edge.
fn role_closure(kb: &Kb) -> BTreeMap<u32, Vec<u32>> {
    let mut direct: BTreeMap<u32, BTreeSet<u32>> = BTreeMap::new();
    for (&sup, subs) in &kb.role_sub {
        direct.entry(sup).or_default();
        for &sub in subs {
            direct.entry(sub).or_default().insert(sup);
        }
    }
    let roles: Vec<u32> = direct.keys().copied().collect();
    let mut out: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    for role in roles {
        let mut reached: BTreeSet<u32> = BTreeSet::new();
        let mut stack = vec![role];
        while let Some(next) = stack.pop() {
            if !reached.insert(next) {
                continue;
            }
            if let Some(supers) = direct.get(&next) {
                stack.extend(supers.iter().copied());
            }
        }
        out.insert(role, reached.into_iter().collect());
    }
    out
}

/// One unit of derived state waiting to have the rules applied to it.
enum Work {
    /// `concept` was added to `S(context)`.
    Concept {
        /// The context slot whose superclass set grew.
        context: usize,
        /// The concept id that entered it.
        concept: u32,
    },
    /// `(subject, role, object)` was added to `R(role)`.
    Edge {
        /// The context slot the edge leaves.
        subject: usize,
        /// The role term id.
        role: u32,
        /// The context slot the edge enters.
        object: usize,
    },
}

/// The fixpoint over one [`Normalized`] axiom set.
struct Engine<'a> {
    /// The axiom table every rule joins against.
    normalized: &'a Normalized,
    /// The `⊤` concept id.
    top: u32,
    /// The `⊥` concept id.
    bottom: u32,
    /// Context concept id → slot.
    slot_of: BTreeMap<u32, usize>,
    /// Slot → derived superclasses, `S(X)`.
    supers: Vec<BTreeSet<u32>>,
    /// Slot → outgoing `(role, slot)` edges, `R(r)` read forwards.
    outgoing: Vec<BTreeSet<(u32, usize)>>,
    /// Slot → incoming `(role, slot)` edges. The reverse index is what makes `R4` and `R7`
    /// incremental: when `S(Y)` grows, only `Y`'s predecessors can gain anything, and
    /// finding them by scanning every context's outgoing set would make each addition cost
    /// the whole graph.
    incoming: Vec<BTreeSet<(u32, usize)>>,
    /// The FIFO of derived state still to be closed over.
    queue: VecDeque<Work>,
}

impl<'a> Engine<'a> {
    /// An engine over `normalized` with no contexts yet.
    fn new(normalized: &'a Normalized, top: u32, bottom: u32) -> Self {
        Self {
            normalized,
            top,
            bottom,
            slot_of: BTreeMap::new(),
            supers: Vec::new(),
            outgoing: Vec::new(),
            incoming: Vec::new(),
            queue: VecDeque::new(),
        }
    }

    /// The slot of `concept`'s context, creating it — and applying `R0` — if it is new.
    fn context(&mut self, concept: u32) -> usize {
        if let Some(&slot) = self.slot_of.get(&concept) {
            return slot;
        }
        let slot = self.supers.len();
        self.supers.push(BTreeSet::new());
        self.outgoing.push(BTreeSet::new());
        self.incoming.push(BTreeSet::new());
        self.slot_of.insert(concept, slot);
        // R0.
        self.add(slot, concept);
        self.add(slot, self.top);
        slot
    }

    /// Add `concept` to `S(context)`, queueing it when it is new.
    fn add(&mut self, context: usize, concept: u32) {
        if self.supers[context].insert(concept) {
            self.queue.push_back(Work::Concept { context, concept });
        }
    }

    /// Add `(subject, role, object)` to `R(role)` AND to every super-role of `role` — the
    /// `R5` closure, applied once at insertion.
    ///
    /// A role the hierarchy never mentions has no closure entry, and then the edge is its
    /// own only conclusion: `R5` over an empty hierarchy is the identity, not a no-op that
    /// drops the edge.
    fn add_edge(&mut self, subject: usize, role: u32, object: usize) {
        let normalized = self.normalized;
        match normalized.role_supers.get(&role) {
            Some(supers) => {
                for &super_role in supers {
                    self.insert_edge(subject, super_role, object);
                }
            }
            None => self.insert_edge(subject, role, object),
        }
    }

    /// Record one already-role-closed edge, queueing it when it is new.
    fn insert_edge(&mut self, subject: usize, role: u32, object: usize) {
        if self.outgoing[subject].insert((role, object)) {
            self.incoming[object].insert((role, subject));
            self.queue.push_back(Work::Edge {
                subject,
                role,
                object,
            });
        }
    }

    /// Close the queue to its least fixpoint.
    fn run(&mut self) {
        while let Some(work) = self.queue.pop_front() {
            match work {
                Work::Concept { context, concept } => self.on_concept(context, concept),
                Work::Edge {
                    subject,
                    role,
                    object,
                } => self.on_edge(subject, role, object),
            }
        }
    }

    /// Apply every rule triggered by `concept ∈ S(context)`.
    fn on_concept(&mut self, context: usize, concept: u32) {
        let normalized = self.normalized;
        // R1.
        if let Some(sups) = normalized.sub.get(&concept) {
            for &sup in sups {
                self.add(context, sup);
            }
        }
        // R2 — the other conjunct must ALREADY be derived; if it arrives later this same
        // arm fires from that side, which is why `conj` is indexed by both.
        if let Some(pairs) = normalized.conj.get(&concept) {
            for &(other, result) in pairs {
                if self.supers[context].contains(&other) {
                    self.add(context, result);
                }
            }
        }
        // R3.
        if let Some(existentials) = normalized.ex_right.get(&concept) {
            for &(role, filler) in existentials {
                let object = self.context(filler);
                self.add_edge(context, role, object);
            }
        }
        // R4 and R7 read from the successor side: `S(context)` grew, so every predecessor
        // of `context` may now satisfy an `∃r.C ⊑ D` premise, or inherit emptiness.
        let predecessors: Vec<(u32, usize)> = self.incoming[context].iter().copied().collect();
        let empty = concept == self.bottom;
        for (role, subject) in predecessors {
            if let Some(sups) = normalized.ex_left.get(&(role, concept)) {
                for &sup in sups {
                    self.add(subject, sup);
                }
            }
            if empty {
                self.add(subject, self.bottom);
            }
        }
    }

    /// Apply every rule triggered by a new `(subject, role, object)` edge.
    fn on_edge(&mut self, subject: usize, role: u32, object: usize) {
        let normalized = self.normalized;
        // R4 — this time from the edge side, over everything already in `S(object)`.
        let derived: Vec<u32> = self.supers[object].iter().copied().collect();
        for concept in derived {
            if let Some(sups) = normalized.ex_left.get(&(role, concept)) {
                for &sup in sups {
                    self.add(subject, sup);
                }
            }
        }
        // R7.
        if self.supers[object].contains(&self.bottom) {
            self.add(subject, self.bottom);
        }
        // R6 — both directions of the composition, because the new edge may be either half.
        if normalized.transitive.contains(&role) {
            let onward: Vec<usize> = self.outgoing[object]
                .iter()
                .filter(|&&(other, _)| other == role)
                .map(|&(_, target)| target)
                .collect();
            for target in onward {
                self.add_edge(subject, role, target);
            }
            let inbound: Vec<usize> = self.incoming[subject]
                .iter()
                .filter(|&&(other, _)| other == role)
                .map(|&(_, source)| source)
                .collect();
            for source in inbound {
                self.add_edge(source, role, object);
            }
        }
    }
}
