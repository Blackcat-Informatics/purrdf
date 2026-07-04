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

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use purrdf_core::{RdfDataset, RdfDatasetBuilder};

use crate::interner::{intern_into, Interner};
use crate::vocab::{
    OWL_EQUIVALENTCLASS, OWL_EQUIVALENTPROPERTY, OWL_INVERSEOF, OWL_SYMMETRICPROPERTY,
    OWL_TRANSITIVEPROPERTY, RDFS_CLASS, RDFS_DOMAIN, RDFS_RANGE, RDFS_RESOURCE, RDFS_SUBCLASSOF,
    RDFS_SUBPROPERTYOF, RDF_PROPERTY, RDF_TYPE,
};
use crate::EntailError;

/// A faithful copy of `ds` (the identity closure for `Simple`).
pub(crate) fn copy_of(ds: &RdfDataset) -> Result<Arc<RdfDataset>, EntailError> {
    let mut b = RdfDatasetBuilder::new();
    b.push_dataset(ds);
    b.freeze().map_err(|e| EntailError::Build(e.to_string()))
}

/// Run the forward chase and emit `original + inferred`.
pub(crate) fn close(ds: &RdfDataset, owl: bool) -> Result<Arc<RdfDataset>, EntailError> {
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
    chase(&mut facts, &base, &c, &interner, owl);

    // Emit: original quads (all graphs) + newly inferred default-graph triples.
    let mut b = RdfDatasetBuilder::new();
    b.push_dataset(ds);
    for t in &facts {
        if original.contains(t) {
            continue;
        }
        let s = intern_into(&mut b, interner.value(t[0]));
        let p = intern_into(&mut b, interner.value(t[1]));
        let o = intern_into(&mut b, interner.value(t[2]));
        b.push_quad(s, p, o, None);
    }
    b.freeze().map_err(|e| EntailError::Build(e.to_string()))
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
    /// `owl:inverseOf` partners, both directions: `p → [q]` and `q → [p]`.
    inv_map: HashMap<u32, Vec<u32>>,
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
            self.inv_map.entry(s).or_default().push(o);
            self.inv_map.entry(o).or_default().push(s);
        }
    }
}

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
        }
    }

    /// Emit `v subPropertyOf v` the first time `v` is discovered as a property
    /// vertex (predicate key, `rdf:Property` instance, or `subPropertyOf`
    /// endpoint) — the new-vertex-only form of the reflexive rule.
    fn emit_spo_refl(&mut self, v: u32, derived: &mut Vec<[u32; 3]>) {
        if self.seen_spo_refl.insert(v) {
            derived.push([v, self.c.spo, v]);
        }
    }

    /// Emit `v subClassOf v` the first time `v` is discovered as a class vertex
    /// (`rdfs:Class` instance or `subClassOf` endpoint).
    fn emit_sco_refl(&mut self, v: u32, derived: &mut Vec<[u32; 3]>) {
        if self.seen_sco_refl.insert(v) {
            derived.push([v, self.c.sco, v]);
        }
    }

    /// Fire every rule for which the single frontier fact `(s, p, o)` can supply a
    /// premise, joining the remaining premises against the full accumulated indices.
    /// Each rule with two data/schema premises is fired from *both* premise
    /// positions (via forward and reverse indices) so that a new fact in either
    /// position is caught — the standard semi-naive expansion.
    #[allow(clippy::cognitive_complexity)]
    fn fire(&mut self, s: u32, p: u32, o: u32, derived: &mut Vec<[u32; 3]>) {
        let c = self.c;
        let interner = self.interner;

        // --- The frontier fact as a *schema* premise, keyed by its predicate. ---
        if p == c.sco {
            // rdfs11 / scm-sco, first premise sco(s, o): (s ⊑ o),(o ⊑ e) ⇒ (s ⊑ e).
            if let Some(es) = self.idx.sco_by_left.get(&o) {
                for &e in es {
                    derived.push([s, c.sco, e]);
                }
            }
            // rdfs11 / scm-sco, second premise sco(s, o): (d ⊑ s),(s ⊑ o) ⇒ (d ⊑ o).
            if let Some(ds) = self.idx.sco_by_right.get(&s) {
                for &d in ds {
                    derived.push([d, c.sco, o]);
                }
            }
            // rdfs9 / cax-sco, first premise sco(s, o): instances of s become o.
            if let Some(insts) = self.idx.type_by_class.get(&s) {
                for &inst in insts {
                    derived.push([inst, c.ty, o]);
                }
            }
            self.emit_sco_refl(s, derived);
            self.emit_sco_refl(o, derived);
        } else if p == c.spo {
            // rdfs5 / scm-spo, both premise positions.
            if let Some(rs) = self.idx.spo_by_left.get(&o) {
                for &r in rs {
                    derived.push([s, c.spo, r]);
                }
            }
            if let Some(ps) = self.idx.spo_by_right.get(&s) {
                for &pp in ps {
                    derived.push([pp, c.spo, o]);
                }
            }
            // rdfs7 / prp-spo1, first premise spo(s, o): rewrite every s-triple to o.
            if let Some(pairs) = self.idx.by_pred.get(&s) {
                for &(ss, oo) in pairs {
                    derived.push([ss, o, oo]);
                }
            }
            self.emit_spo_refl(s, derived);
            self.emit_spo_refl(o, derived);
        } else if p == c.ty {
            // rdfs9 / cax-sco, second premise type(s, o): (s a o),(o ⊑ d) ⇒ (s a d).
            if let Some(ds) = self.idx.sco_by_left.get(&o) {
                for &d in ds {
                    derived.push([s, c.ty, d]);
                }
            }
            // rdfs6: (s a rdf:Property) ⇒ (s subPropertyOf s).
            if o == c.property {
                self.emit_spo_refl(s, derived);
            }
            // rdfs10 + rdfs8: (s a rdfs:Class) ⇒ (s ⊑ s) and (s ⊑ rdfs:Resource).
            if o == c.class {
                self.emit_sco_refl(s, derived);
                derived.push([s, c.sco, c.resource]);
            }
            if self.owl {
                // prp-symp, first premise type(s, Symmetric): mirror every s-triple.
                if o == c.symmetric {
                    if let Some(pairs) = self.idx.by_pred.get(&s) {
                        for &(x, y) in pairs {
                            if interner.is_subject(y) {
                                derived.push([y, s, x]);
                            }
                        }
                    }
                }
                // prp-trp, first premise type(s, Transitive): one-step join over all
                // s-edges (the fixpoint composes longer chains across rounds).
                if o == c.transitive {
                    if let Some(pairs) = self.idx.by_pred.get(&s) {
                        for &(x, y) in pairs {
                            if let Some(zs) = self.idx.by_pred_so.get(&s).and_then(|m| m.get(&y)) {
                                for &z in zs {
                                    derived.push([x, s, z]);
                                }
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
                    derived.push([ss, c.ty, o]);
                }
            }
        } else if p == c.rng {
            // rdfs3 / prp-rng, first premise rng(s, o): every s-object gets type o.
            if let Some(pairs) = self.idx.by_pred.get(&s) {
                for &(_ss, oo) in pairs {
                    if interner.is_subject(oo) {
                        derived.push([oo, c.ty, o]);
                    }
                }
            }
        }
        if self.owl {
            if p == c.eq_class {
                // scm-eqc: equivalentClass ⇒ mutual subClassOf.
                derived.push([s, c.sco, o]);
                derived.push([o, c.sco, s]);
            } else if p == c.eq_prop {
                // scm-eqp: equivalentProperty ⇒ mutual subPropertyOf.
                derived.push([s, c.spo, o]);
                derived.push([o, c.spo, s]);
            } else if p == c.inverse_of {
                // prp-inv, first premise inverseOf(s, o): join with full s/o triples.
                if let Some(pairs) = self.idx.by_pred.get(&s) {
                    for &(x, y) in pairs {
                        if interner.is_subject(y) {
                            derived.push([y, o, x]);
                        }
                    }
                }
                if let Some(pairs) = self.idx.by_pred.get(&o) {
                    for &(x, y) in pairs {
                        if interner.is_subject(y) {
                            derived.push([y, s, x]);
                        }
                    }
                }
            }
        }

        // --- The frontier fact as a *data* triple (s, p, o); schema is looked up. ---
        // rdfs7 / prp-spo1, second premise (s p o): (p ⊑ q) ⇒ (s q o).
        if let Some(qs) = self.idx.spo_by_left.get(&p) {
            for &q in qs {
                derived.push([s, q, o]);
            }
        }
        // rdfs2 / prp-dom, second premise (s p o): (p domain cc) ⇒ (s a cc).
        if let Some(cs) = self.idx.dom_by_prop.get(&p) {
            for &cc in cs {
                derived.push([s, c.ty, cc]);
            }
        }
        // rdfs3 / prp-rng, second premise (s p o): (p range cc) ⇒ (o a cc).
        if interner.is_subject(o) {
            if let Some(cs) = self.idx.rng_by_prop.get(&p) {
                for &cc in cs {
                    derived.push([o, c.ty, cc]);
                }
            }
        }
        // Every predicate is reflexively a subProperty of itself (new-vertex only).
        self.emit_spo_refl(p, derived);
        if self.owl {
            // prp-symp, second premise (s p o) with p symmetric ⇒ (o p s).
            if interner.is_subject(o) && self.idx.sym_props.contains(&p) {
                derived.push([o, p, s]);
            }
            // prp-trp, second premise (s p o) with p transitive: compose with the
            // full predecessor/successor adjacency of p.
            if self.idx.trans_props.contains(&p) {
                if let Some(zs) = self.idx.by_pred_so.get(&p).and_then(|m| m.get(&o)) {
                    for &z in zs {
                        derived.push([s, p, z]);
                    }
                }
                if let Some(ws) = self.idx.by_pred_os.get(&p).and_then(|m| m.get(&s)) {
                    for &w in ws {
                        derived.push([w, p, o]);
                    }
                }
            }
            // prp-inv, data side (s p o) with (p inverseOf q) ⇒ (o q s).
            if interner.is_subject(o) {
                if let Some(partners) = self.idx.inv_map.get(&p) {
                    for &q in partners {
                        derived.push([o, q, s]);
                    }
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
fn chase(
    facts: &mut HashSet<[u32; 3]>,
    base: &[[u32; 3]],
    c: &Consts,
    interner: &Interner,
    owl: bool,
) {
    let mut chaser = Chaser::new(c, interner, owl);
    for &t in base {
        chaser.idx.insert(t, c);
    }
    let mut delta: Vec<[u32; 3]> = base.to_vec();
    while !delta.is_empty() {
        let mut derived: Vec<[u32; 3]> = Vec::new();
        for &[s, p, o] in &delta {
            chaser.fire(s, p, o, &mut derived);
        }
        let mut next: Vec<[u32; 3]> = Vec::new();
        for t in derived {
            if facts.insert(t) {
                chaser.idx.insert(t, c);
                next.push(t);
            }
        }
        delta = next;
    }
}
