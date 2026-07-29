// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0
//
// The RDFS / OWL-RL forward-materialization rule set transliterated here derives
// from the reasoning rule tables of the sister project `gmeow-logic` (originally
// AGPL-3.0-only); the copyright holder relicenses this port under MIT OR
// Apache-2.0. The rule *semantics* (rule ids in comments: rdfs*, scm-*, prp-*)
// are the W3C RDF 1.1 Semantics / OWL 2 RL calculus — spec-derived, not novel.

//! The RDFS / OWL-RL forward-materialization ("chase") reasoner.
//!
//! A genuinely delta-driven semi-naive evaluator: a *frontier* of newly-derived
//! facts seeds each round, and every rule fires only where at least one premise is a
//! frontier fact — the remaining premises are joined against incrementally-maintained
//! indices that are never rebuilt over the whole accumulated set. Two-premise rules
//! fire from both premise positions (forward and reverse indices), so a new fact in
//! either slot is caught. The next frontier is the round's genuinely-new triples
//! (deduplicated against the accumulated set); the chase halts when the frontier is
//! empty. The reflexive rules (`p subPropertyOf p`, `c subClassOf c`) fire once per
//! *newly-seen* predicate/class/property vertex. The materialized closure is the
//! least fixpoint, identical to a naive evaluation of the same rule set.
//!
//! # Every candidate carries the rule that proposed it
//!
//! A candidate conclusion is pushed as `(triple, ChaseRule)`, and the tag is credited
//! when — and only when — that candidate turns out to be a genuinely new fact. So the
//! per-rule tally in [`ChaseStats`] is a count of triples the rule was the FIRST to add,
//! summing to exactly the number of inferred triples in the result, rather than a count of
//! how often the rule was tried. A fact two rules both conclude is credited to whichever
//! reached it first in the chase's deterministic firing order; nothing about that order
//! depends on hashing, so the tally is reproducible.
//!
//! The derivation ORDER is not observable in the closure: the emitted triples are sorted
//! by interned term id, and the chase mints no terms, so the result is a function of the
//! fact SET alone.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use purrdf_core::{RdfDataset, RdfDatasetBuilder, TermRef, TermValue};

use crate::EntailError;
use crate::calculus::ChaseRule;
use crate::interner::{Interner, intern_into, term_bytes};
use crate::report::ChaseStats;
use crate::vocab::{
    OWL_EQUIVALENTCLASS, OWL_EQUIVALENTPROPERTY, OWL_INVERSEOF, OWL_SYMMETRICPROPERTY,
    OWL_TRANSITIVEPROPERTY, RDF_PROPERTY, RDF_TYPE, RDFS_CLASS, RDFS_DOMAIN, RDFS_RANGE,
    RDFS_RESOURCE, RDFS_SUBCLASSOF, RDFS_SUBPROPERTYOF,
};

/// A faithful copy of `ds` (the identity closure for `Simple`).
pub(crate) fn copy_of(ds: &RdfDataset) -> Result<Arc<RdfDataset>, EntailError> {
    let mut b = RdfDatasetBuilder::new();
    b.push_dataset(ds);
    b.freeze().map_err(|e| EntailError::Build(e.to_string()))
}

/// The bare-`RDF` regime closure: the original graph plus the RDF axiomatic
/// predicate-typing rule.
///
/// The RDF Semantics entail that any resource used in the predicate position of a
/// triple is an `rdf:Property` (rule `rdfD2`, spelled `rdf1` in RDF 1.0 — `s p o ⇒ p
/// rdf:type rdf:Property`). The full RDF axiomatic-triple schema is infinite and is *not*
/// materialized; only this single, decidable rule is applied, which is all the bare
/// RDF entailment regime requires for BGP query answering over a finite graph.
pub(crate) fn close_rdf(ds: &RdfDataset) -> Result<(Arc<RdfDataset>, ChaseStats), EntailError> {
    let mut b = RdfDatasetBuilder::new();
    b.push_dataset(ds);
    let mut stats = ChaseStats::none();

    // Seed the store with the typings the graph ALREADY asserts. A conclusion the graph
    // already holds is not a contribution, and crediting it would break the property that
    // makes the per-rule tally checkable: that the counts sum to the inferred triples. The
    // emitted dataset is unchanged either way — the builder folds a duplicate quad — so
    // this buys honesty in the tally at no cost to the output.
    let mut seen: HashSet<TermValue> = HashSet::new();
    for q in ds.quad_refs() {
        if q.g.is_some() {
            continue;
        }
        if let (TermRef::Iri(subject), TermRef::Iri(predicate), TermRef::Iri(object)) =
            (q.s, q.p, q.o)
            && predicate == RDF_TYPE
            && object == RDF_PROPERTY
        {
            seen.insert(TermValue::Iri(subject.to_owned()));
        }
    }

    // Emit `p rdf:type rdf:Property` once per distinct default-graph predicate, in
    // first-seen order for deterministic output.
    for q in ds.quads() {
        if q.g.is_some() {
            continue; // entailment operates over the default graph
        }
        // Every default-graph quad enumerates exactly one candidate conclusion.
        stats.join_steps += 1;
        let pred = ds.term_value(q.p);
        if seen.insert(pred.clone()) {
            stats.commit(ChaseRule::PredicateProperty);
            let pid = intern_into(&mut b, &pred);
            let ty = b.intern_iri(RDF_TYPE);
            let prop = b.intern_iri(RDF_PROPERTY);
            b.push_quad(pid, ty, prop, None);
        }
    }
    // The store this lane maintains is exactly the typed predicates: the ones the graph
    // asserted plus the ones the rule concluded.
    stats.stored_facts = seen.len();
    stats.term_arena_bytes = seen.iter().map(term_bytes).sum();

    let dataset = b.freeze().map_err(|e| EntailError::Build(e.to_string()))?;
    Ok((dataset, stats))
}

/// Run the forward chase and emit `original + inferred`.
pub(crate) fn close(
    ds: &RdfDataset,
    owl: bool,
) -> Result<(Arc<RdfDataset>, ChaseStats), EntailError> {
    let mut interner = Interner::default();

    // Intern the default-graph triples as the seed fact set. `base` keeps them in
    // dataset order (deduplicated) so the semi-naive frontier starts from a
    // deterministic sequence rather than hash-iteration order.
    let mut facts: HashSet<[u32; 3]> = HashSet::new();
    let mut base: Vec<[u32; 3]> = Vec::new();
    for q in ds.quads() {
        if q.g.is_some() {
            continue; // RDFS/OWL-RL entailment operates over the default graph
        }
        let s = interner.intern(ds.term_value(q.s));
        let p = interner.intern(ds.term_value(q.p));
        let o = interner.intern(ds.term_value(q.o));
        let t = [s, p, o];
        if facts.insert(t) {
            base.push(t);
        }
    }
    let original = facts.clone();

    let c = Consts::intern(&mut interner);
    let mut stats = chase(&mut facts, &base, &c, &interner, owl);
    stats.stored_facts = facts.len();
    stats.term_arena_bytes = interner.term_bytes();

    // Emit: original quads (all graphs) + newly inferred default-graph triples.
    let mut b = RdfDatasetBuilder::new();
    b.push_dataset(ds);
    // `HashSet` iteration order is not stable across runs, so sort the accumulated
    // facts by their interned term ids to get a deterministic (not insertion-order)
    // emission order, matching the RIF path.
    let mut ordered: Vec<[u32; 3]> = facts.iter().copied().collect();
    ordered.sort_unstable();
    for t in ordered {
        if original.contains(&t) {
            continue;
        }
        let s = intern_into(&mut b, interner.value(t[0]));
        let p = intern_into(&mut b, interner.value(t[1]));
        let o = intern_into(&mut b, interner.value(t[2]));
        b.push_quad(s, p, o, None);
    }
    let dataset = b.freeze().map_err(|e| EntailError::Build(e.to_string()))?;
    Ok((dataset, stats))
}

/// Pre-interned vocabulary constant ids.
struct Consts {
    ty: u32,
    property: u32,
    sco: u32,
    spo: u32,
    dom: u32,
    rng: u32,
    class: u32,
    resource: u32,
    eq_class: u32,
    eq_prop: u32,
    inverse_of: u32,
    symmetric: u32,
    transitive: u32,
}

impl Consts {
    fn intern(i: &mut Interner) -> Self {
        Self {
            ty: i.intern_iri(RDF_TYPE),
            property: i.intern_iri(RDF_PROPERTY),
            sco: i.intern_iri(RDFS_SUBCLASSOF),
            spo: i.intern_iri(RDFS_SUBPROPERTYOF),
            dom: i.intern_iri(RDFS_DOMAIN),
            rng: i.intern_iri(RDFS_RANGE),
            class: i.intern_iri(RDFS_CLASS),
            resource: i.intern_iri(RDFS_RESOURCE),
            eq_class: i.intern_iri(OWL_EQUIVALENTCLASS),
            eq_prop: i.intern_iri(OWL_EQUIVALENTPROPERTY),
            inverse_of: i.intern_iri(OWL_INVERSEOF),
            symmetric: i.intern_iri(OWL_SYMMETRICPROPERTY),
            transitive: i.intern_iri(OWL_TRANSITIVEPROPERTY),
        }
    }
}

/// Incrementally-maintained rule indices over the interned fact ids.
///
/// Every index is grown by [`Indexes::insert`] as facts are added (the base seed,
/// then each round's frontier) so no round ever rebuilds an index over the whole
/// accumulated set. Per-key `Vec`s preserve insertion order, which — because facts
/// are inserted in the deterministic frontier order — keeps every derivation
/// deterministic without any hash-iteration leaking into results.
#[derive(Default)]
struct Indexes {
    /// Every triple keyed by predicate: `p → [(s, o)]` (ordered edge list).
    by_pred: HashMap<u32, Vec<(u32, u32)>>,
    /// Per-predicate successor adjacency `p → s → [o]` (transitive-property joins).
    by_pred_so: HashMap<u32, HashMap<u32, Vec<u32>>>,
    /// Per-predicate predecessor adjacency `p → o → [s]` (transitive-property joins).
    by_pred_os: HashMap<u32, HashMap<u32, Vec<u32>>>,
    /// `subClassOf` forward edges `c → [d]`.
    sco_by_left: HashMap<u32, Vec<u32>>,
    /// `subClassOf` reverse edges `d → [c]`.
    sco_by_right: HashMap<u32, Vec<u32>>,
    /// `subPropertyOf` forward edges `p → [q]`.
    spo_by_left: HashMap<u32, Vec<u32>>,
    /// `subPropertyOf` reverse edges `q → [p]`.
    spo_by_right: HashMap<u32, Vec<u32>>,
    /// Instances by class: `c → [s]` for `s rdf:type c`.
    type_by_class: HashMap<u32, Vec<u32>>,
    /// Domain declarations `p → [c]` for `p rdfs:domain c`.
    dom_by_prop: HashMap<u32, Vec<u32>>,
    /// Range declarations `p → [c]` for `p rdfs:range c`.
    rng_by_prop: HashMap<u32, Vec<u32>>,
    /// Properties typed `owl:SymmetricProperty`.
    sym_props: HashSet<u32>,
    /// Properties typed `owl:TransitiveProperty`.
    trans_props: HashSet<u32>,
    /// `owl:inverseOf` read left to right: `p1 → [p2]`, the premise of `prp-inv1`.
    inv_forward: HashMap<u32, Vec<u32>>,
    /// `owl:inverseOf` read right to left: `p2 → [p1]`, the premise of `prp-inv2`.
    ///
    /// Kept apart from [`Indexes::inv_forward`] rather than merged into one symmetric map
    /// because `prp-inv1` and `prp-inv2` are two rules with two ids, and a merged map
    /// loses which of them licensed a given conclusion.
    inv_backward: HashMap<u32, Vec<u32>>,
}

impl Indexes {
    /// Fold a single fact into every index it participates in.
    fn insert(&mut self, t: [u32; 3], c: &Consts) {
        let [s, p, o] = t;
        self.by_pred.entry(p).or_default().push((s, o));
        self.by_pred_so
            .entry(p)
            .or_default()
            .entry(s)
            .or_default()
            .push(o);
        self.by_pred_os
            .entry(p)
            .or_default()
            .entry(o)
            .or_default()
            .push(s);
        if p == c.sco {
            self.sco_by_left.entry(s).or_default().push(o);
            self.sco_by_right.entry(o).or_default().push(s);
        } else if p == c.spo {
            self.spo_by_left.entry(s).or_default().push(o);
            self.spo_by_right.entry(o).or_default().push(s);
        } else if p == c.ty {
            self.type_by_class.entry(o).or_default().push(s);
            if o == c.symmetric {
                self.sym_props.insert(s);
            }
            if o == c.transitive {
                self.trans_props.insert(s);
            }
        } else if p == c.dom {
            self.dom_by_prop.entry(s).or_default().push(o);
        } else if p == c.rng {
            self.rng_by_prop.entry(s).or_default().push(o);
        } else if p == c.inverse_of {
            self.inv_forward.entry(s).or_default().push(o);
            self.inv_backward.entry(o).or_default().push(s);
        }
    }
}

/// One candidate conclusion: the triple, and the rule that proposed it.
type Candidate = ([u32; 3], ChaseRule);

/// Semi-naive chase state: the incremental indices plus the vocabulary constants,
/// the term interner, the regime flag, and the reflexive "already-emitted" sets.
struct Chaser<'a> {
    idx: Indexes,
    c: &'a Consts,
    interner: &'a Interner,
    owl: bool,
    /// Vertices for which `v subPropertyOf v` has already been emitted.
    seen_spo_refl: HashSet<u32>,
    /// Vertices for which `v subClassOf v` has already been emitted.
    seen_sco_refl: HashSet<u32>,
    /// Conclusions abandoned because their subject would not be a legal RDF 1.2
    /// subject — the observation behind the generalized-RDF boundary.
    drops: u64,
}

/// Propose `[subject, predicate, object]` if RDF 1.2 can hold it, counting the abandoned
/// conclusion otherwise.
///
/// A rule that concludes into subject position (`rdfs3` / `prp-rng`, `prp-symp`,
/// `prp-inv1`, `prp-inv2`) can reach a literal or a triple term there. That triple is a
/// *generalized*-RDF triple, which the [`RdfDataset`] IR cannot represent — so the
/// conclusion is dropped rather than a term being fabricated for it, and the drop is
/// counted so the report can name the boundary instead of silently narrowing.
///
/// A free function over the two fields it needs rather than a `&mut self` method, so a
/// caller may hold a borrow of the indices across the call and no lookup has to be cloned
/// out of them to satisfy the borrow checker.
fn propose_into_subject(
    interner: &Interner,
    drops: &mut u64,
    triple: [u32; 3],
    rule: ChaseRule,
    derived: &mut Vec<Candidate>,
) {
    if interner.is_subject(triple[0]) {
        derived.push((triple, rule));
    } else {
        *drops += 1;
    }
}

impl<'a> Chaser<'a> {
    fn new(c: &'a Consts, interner: &'a Interner, owl: bool) -> Self {
        Self {
            idx: Indexes::default(),
            c,
            interner,
            owl,
            seen_spo_refl: HashSet::new(),
            seen_sco_refl: HashSet::new(),
            drops: 0,
        }
    }

    /// Emit `v subPropertyOf v` the first time `v` is discovered as a property
    /// vertex (predicate key, `rdf:Property` instance, or `subPropertyOf`
    /// endpoint) — the new-vertex-only form of the reflexive rule.
    fn emit_spo_refl(&mut self, v: u32, derived: &mut Vec<Candidate>) {
        if self.seen_spo_refl.insert(v) {
            derived.push(([v, self.c.spo, v], ChaseRule::SubPropertyReflexive));
        }
    }

    /// Emit `v subClassOf v` the first time `v` is discovered as a class vertex
    /// (`rdfs:Class` instance or `subClassOf` endpoint).
    fn emit_sco_refl(&mut self, v: u32, derived: &mut Vec<Candidate>) {
        if self.seen_sco_refl.insert(v) {
            derived.push(([v, self.c.sco, v], ChaseRule::SubClassReflexive));
        }
    }

    /// Fire every rule for which the single frontier fact `(s, p, o)` can supply a
    /// premise, joining the remaining premises against the full accumulated indices.
    /// Each rule with two data/schema premises is fired from *both* premise
    /// positions (via forward and reverse indices) so that a new fact in either
    /// position is caught — the standard semi-naive expansion.
    #[allow(clippy::cognitive_complexity)]
    fn fire(&mut self, s: u32, p: u32, o: u32, derived: &mut Vec<Candidate>) {
        let c = self.c;
        let interner = self.interner;

        // --- The frontier fact as a *schema* premise, keyed by its predicate. ---
        if p == c.sco {
            // rdfs11 / scm-sco, first premise sco(s, o): (s ⊑ o),(o ⊑ e) ⇒ (s ⊑ e).
            if let Some(es) = self.idx.sco_by_left.get(&o) {
                for &e in es {
                    derived.push(([s, c.sco, e], ChaseRule::SubClassTransitive));
                }
            }
            // rdfs11 / scm-sco, second premise sco(s, o): (d ⊑ s),(s ⊑ o) ⇒ (d ⊑ o).
            if let Some(ds) = self.idx.sco_by_right.get(&s) {
                for &d in ds {
                    derived.push(([d, c.sco, o], ChaseRule::SubClassTransitive));
                }
            }
            // rdfs9 / cax-sco, first premise sco(s, o): instances of s become o.
            if let Some(insts) = self.idx.type_by_class.get(&s) {
                for &inst in insts {
                    derived.push(([inst, c.ty, o], ChaseRule::SubClassInstance));
                }
            }
            self.emit_sco_refl(s, derived);
            self.emit_sco_refl(o, derived);
        } else if p == c.spo {
            // rdfs5 / scm-spo, both premise positions.
            if let Some(rs) = self.idx.spo_by_left.get(&o) {
                for &r in rs {
                    derived.push(([s, c.spo, r], ChaseRule::SubPropertyTransitive));
                }
            }
            if let Some(ps) = self.idx.spo_by_right.get(&s) {
                for &pp in ps {
                    derived.push(([pp, c.spo, o], ChaseRule::SubPropertyTransitive));
                }
            }
            // rdfs7 / prp-spo1, first premise spo(s, o): rewrite every s-triple to o.
            if let Some(pairs) = self.idx.by_pred.get(&s) {
                for &(ss, oo) in pairs {
                    derived.push(([ss, o, oo], ChaseRule::SubPropertyRewrite));
                }
            }
            self.emit_spo_refl(s, derived);
            self.emit_spo_refl(o, derived);
        } else if p == c.ty {
            // rdfs9 / cax-sco, second premise type(s, o): (s a o),(o ⊑ d) ⇒ (s a d).
            if let Some(ds) = self.idx.sco_by_left.get(&o) {
                for &d in ds {
                    derived.push(([s, c.ty, d], ChaseRule::SubClassInstance));
                }
            }
            // rdfs6: (s a rdf:Property) ⇒ (s subPropertyOf s).
            if o == c.property {
                self.emit_spo_refl(s, derived);
            }
            // rdfs10 + rdfs8: (s a rdfs:Class) ⇒ (s ⊑ s) and (s ⊑ rdfs:Resource).
            if o == c.class {
                self.emit_sco_refl(s, derived);
                derived.push(([s, c.sco, c.resource], ChaseRule::ClassResource));
            }
            if self.owl {
                // prp-symp, first premise type(s, Symmetric): mirror every s-triple.
                if o == c.symmetric
                    && let Some(pairs) = self.idx.by_pred.get(&s)
                {
                    for &(x, y) in pairs {
                        propose_into_subject(
                            interner,
                            &mut self.drops,
                            [y, s, x],
                            ChaseRule::Symmetric,
                            derived,
                        );
                    }
                }
                // prp-trp, first premise type(s, Transitive): one-step join over all
                // s-edges (the fixpoint composes longer chains across rounds).
                if o == c.transitive
                    && let Some(pairs) = self.idx.by_pred.get(&s)
                {
                    for &(x, y) in pairs {
                        if let Some(zs) = self.idx.by_pred_so.get(&s).and_then(|m| m.get(&y)) {
                            for &z in zs {
                                derived.push(([x, s, z], ChaseRule::Transitive));
                            }
                        }
                    }
                }
            }
        }
        if p == c.dom {
            // rdfs2 / prp-dom, first premise dom(s, o): every s-subject gets type o.
            if let Some(pairs) = self.idx.by_pred.get(&s) {
                for &(ss, _oo) in pairs {
                    derived.push(([ss, c.ty, o], ChaseRule::Domain));
                }
            }
        } else if p == c.rng {
            // rdfs3 / prp-rng, first premise rng(s, o): every s-object gets type o.
            if let Some(pairs) = self.idx.by_pred.get(&s) {
                for &(_ss, oo) in pairs {
                    propose_into_subject(
                        interner,
                        &mut self.drops,
                        [oo, c.ty, o],
                        ChaseRule::Range,
                        derived,
                    );
                }
            }
        }
        if self.owl {
            if p == c.eq_class {
                // scm-eqc1: equivalentClass ⇒ mutual subClassOf.
                derived.push(([s, c.sco, o], ChaseRule::EquivalentClass));
                derived.push(([o, c.sco, s], ChaseRule::EquivalentClass));
            } else if p == c.eq_prop {
                // scm-eqp1: equivalentProperty ⇒ mutual subPropertyOf.
                derived.push(([s, c.spo, o], ChaseRule::EquivalentProperty));
                derived.push(([o, c.spo, s], ChaseRule::EquivalentProperty));
            } else if p == c.inverse_of {
                // prp-inv1, first premise inverseOf(s, o): (x s y) ⇒ (y o x).
                if let Some(pairs) = self.idx.by_pred.get(&s) {
                    for &(x, y) in pairs {
                        propose_into_subject(
                            interner,
                            &mut self.drops,
                            [y, o, x],
                            ChaseRule::Inverse1,
                            derived,
                        );
                    }
                }
                // prp-inv2, first premise inverseOf(s, o): (x o y) ⇒ (y s x).
                if let Some(pairs) = self.idx.by_pred.get(&o) {
                    for &(x, y) in pairs {
                        propose_into_subject(
                            interner,
                            &mut self.drops,
                            [y, s, x],
                            ChaseRule::Inverse2,
                            derived,
                        );
                    }
                }
            }
        }

        // --- The frontier fact as a *data* triple (s, p, o); schema is looked up. ---
        // rdfs7 / prp-spo1, second premise (s p o): (p ⊑ q) ⇒ (s q o).
        if let Some(qs) = self.idx.spo_by_left.get(&p) {
            for &q in qs {
                derived.push(([s, q, o], ChaseRule::SubPropertyRewrite));
            }
        }
        // rdfs2 / prp-dom, second premise (s p o): (p domain cc) ⇒ (s a cc).
        if let Some(cs) = self.idx.dom_by_prop.get(&p) {
            for &cc in cs {
                derived.push(([s, c.ty, cc], ChaseRule::Domain));
            }
        }
        // rdfs3 / prp-rng, second premise (s p o): (p range cc) ⇒ (o a cc).
        if let Some(cs) = self.idx.rng_by_prop.get(&p) {
            for &cc in cs {
                propose_into_subject(
                    interner,
                    &mut self.drops,
                    [o, c.ty, cc],
                    ChaseRule::Range,
                    derived,
                );
            }
        }
        // Every predicate is reflexively a subProperty of itself (new-vertex only).
        self.emit_spo_refl(p, derived);
        if self.owl {
            // prp-symp, second premise (s p o) with p symmetric ⇒ (o p s).
            if self.idx.sym_props.contains(&p) {
                propose_into_subject(
                    interner,
                    &mut self.drops,
                    [o, p, s],
                    ChaseRule::Symmetric,
                    derived,
                );
            }
            // prp-trp, second premise (s p o) with p transitive: compose with the
            // full predecessor/successor adjacency of p.
            if self.idx.trans_props.contains(&p) {
                if let Some(zs) = self.idx.by_pred_so.get(&p).and_then(|m| m.get(&o)) {
                    for &z in zs {
                        derived.push(([s, p, z], ChaseRule::Transitive));
                    }
                }
                if let Some(ws) = self.idx.by_pred_os.get(&p).and_then(|m| m.get(&s)) {
                    for &w in ws {
                        derived.push(([w, p, o], ChaseRule::Transitive));
                    }
                }
            }
            // prp-inv1, data side (s p o) with (p inverseOf q) ⇒ (o q s).
            if let Some(partners) = self.idx.inv_forward.get(&p) {
                for &q in partners {
                    propose_into_subject(
                        interner,
                        &mut self.drops,
                        [o, q, s],
                        ChaseRule::Inverse1,
                        derived,
                    );
                }
            }
            // prp-inv2, data side (s p o) with (q inverseOf p) ⇒ (o q s).
            if let Some(partners) = self.idx.inv_backward.get(&p) {
                for &q in partners {
                    propose_into_subject(
                        interner,
                        &mut self.drops,
                        [o, q, s],
                        ChaseRule::Inverse2,
                        derived,
                    );
                }
            }
        }
    }
}

/// Genuine semi-naive (delta/frontier) forward chase to a fixpoint.
///
/// The frontier `delta` starts as the base facts. Each round fires every rule
/// once per frontier fact, joining that fact's premise against the full
/// incrementally-maintained [`Indexes`]; rules with two premises fire from both
/// positions (forward and reverse indices) so a new fact in either slot is caught.
/// Newly-derived triples (deduplicated against the accumulated `facts`) become the
/// next round's frontier; the chase stops when the frontier is empty.
///
/// Returns the run's measurements: the per-rule tally of COMMITTED conclusions, the total
/// candidates enumerated, and the generalized-RDF conclusions abandoned. The caller fills
/// in the two store-shaped coordinates (`stored_facts`, `term_arena_bytes`), which it
/// alone can see.
fn chase(
    facts: &mut HashSet<[u32; 3]>,
    base: &[[u32; 3]],
    c: &Consts,
    interner: &Interner,
    owl: bool,
) -> ChaseStats {
    let mut stats = ChaseStats::none();
    let mut chaser = Chaser::new(c, interner, owl);
    for &t in base {
        chaser.idx.insert(t, c);
    }
    let mut delta: Vec<[u32; 3]> = base.to_vec();
    let mut derived: Vec<Candidate> = Vec::new();
    let mut next: Vec<[u32; 3]> = Vec::new();
    while !delta.is_empty() {
        derived.clear();
        next.clear();
        for &[s, p, o] in &delta {
            chaser.fire(s, p, o, &mut derived);
        }
        stats.join_steps = stats.join_steps.saturating_add(derived.len() as u64);
        for &(t, rule) in &derived {
            if facts.insert(t) {
                // The FIRST rule to reach a fact is the one credited; a later
                // re-derivation of the same triple commits nothing and counts nothing.
                stats.commit(rule);
                chaser.idx.insert(t, c);
                next.push(t);
            }
        }
        std::mem::swap(&mut delta, &mut next);
    }
    stats.generalized_rdf_drops = chaser.drops;
    // An abandoned conclusion was still enumerated, so it is still a join step; it just
    // never reached `derived`.
    stats.join_steps = stats.join_steps.saturating_add(chaser.drops);
    stats
}
