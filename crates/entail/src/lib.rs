// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0
//
// The RDFS / OWL-RL forward-materialization rule set transliterated here derives
// from the reasoning rule tables of the sister project `gmeow-logic` (originally
// AGPL-3.0-only); the copyright holder relicenses this port under MIT OR
// Apache-2.0. The rule *semantics* (rule ids in comments: rdfs*, scm-*, prp-*)
// are the W3C RDF 1.1 Semantics / OWL 2 RL calculus — spec-derived, not novel.
#![forbid(unsafe_code)]

//! Native, wasm-clean entailment for the PurRDF [`RdfDataset`] IR.
//!
//! A forward-materialization ("chase") reasoner: it closes a dataset's default
//! graph under a fixed RDFS (and OWL-RL-shaped) rule set to a fixpoint, then emits
//! a derived dataset (original quads + inferred triples). It **subsumes** the
//! external AGPL/Nemo-backed reasoner — the rule tables are ported and relicensed,
//! the chase is a native semi-naive evaluator over `RdfDataset` terms (no Nemo, no
//! `tokio`, no string round-trip), so this crate stays `wasm32`-clean and
//! MIT/Apache. It mints **no** vocabulary IRIs: every constant below is a standard
//! `rdf:`/`rdfs:`/`owl:` IRI from the entailment spec itself.
//!
//! Scope: `Simple`/`RDF` are the identity closure; `RDFS` and `OWL-RL` run the
//! chase. `OWL-Direct` (full DL) and `D` (datatype) entailment are **not** a
//! materialize-and-match affair and return [`EntailError::Unsupported`], which the
//! caller records as a typed, spec-inherent gap.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use purrdf_core::{RdfDataset, RdfDatasetBuilder, RdfLiteral, TermValue};

/// A SPARQL entailment regime (`sparql:entailmentRegime`), by its W3C IRI's local
/// name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Regime {
    /// `entailment/Simple` — no entailment; the graph is its own closure.
    Simple,
    /// `entailment/RDF` — RDF entailment (treated as the identity closure here;
    /// the RDF axiomatic triples are not materialized).
    Rdf,
    /// `entailment/RDFS` — RDFS entailment via the native chase.
    Rdfs,
    /// `entailment/OWL-RL` (a.k.a. OWL 2 RL) — RDFS + the OWL-RL-shaped rules.
    OwlRl,
    /// `entailment/OWL-Direct` — full OWL DL; out of reach for forward chaining.
    OwlDirect,
    /// `entailment/D` — datatype entailment; not materialize-and-match.
    D,
}

impl Regime {
    /// Parse a regime IRI (e.g. `http://www.w3.org/ns/entailment/RDFS`).
    #[must_use]
    pub fn from_iri(iri: &str) -> Option<Self> {
        match iri.rsplit('/').next()? {
            "Simple" => Some(Self::Simple),
            "RDF" => Some(Self::Rdf),
            "RDFS" => Some(Self::Rdfs),
            "OWL-RL" | "OWL-RDF-Based" => Some(Self::OwlRl),
            "OWL-Direct" => Some(Self::OwlDirect),
            "D" => Some(Self::D),
            _ => None,
        }
    }
}

/// Why a closure could not be produced.
#[derive(Debug, Clone)]
pub enum EntailError {
    /// The regime is a spec-inherent boundary for a forward-materialization
    /// reasoner (OWL-Direct DL, D-entailment): not implementable as
    /// materialize-and-match.
    Unsupported(Regime),
    /// Building the derived dataset failed.
    Build(String),
}

impl std::fmt::Display for EntailError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported(r) => write!(f, "entailment regime {r:?} is not materializable"),
            Self::Build(msg) => write!(f, "entailment build error: {msg}"),
        }
    }
}

impl std::error::Error for EntailError {}

// Standard vocabulary IRIs (spec-supplied; PurRDF mints none of its own).
const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
const RDF_PROPERTY: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#Property";
const RDFS_SUBCLASSOF: &str = "http://www.w3.org/2000/01/rdf-schema#subClassOf";
const RDFS_SUBPROPERTYOF: &str = "http://www.w3.org/2000/01/rdf-schema#subPropertyOf";
const RDFS_DOMAIN: &str = "http://www.w3.org/2000/01/rdf-schema#domain";
const RDFS_RANGE: &str = "http://www.w3.org/2000/01/rdf-schema#range";
const RDFS_CLASS: &str = "http://www.w3.org/2000/01/rdf-schema#Class";
const RDFS_RESOURCE: &str = "http://www.w3.org/2000/01/rdf-schema#Resource";
const OWL_EQUIVALENTCLASS: &str = "http://www.w3.org/2002/07/owl#equivalentClass";
const OWL_EQUIVALENTPROPERTY: &str = "http://www.w3.org/2002/07/owl#equivalentProperty";
const OWL_INVERSEOF: &str = "http://www.w3.org/2002/07/owl#inverseOf";
const OWL_SYMMETRICPROPERTY: &str = "http://www.w3.org/2002/07/owl#SymmetricProperty";
const OWL_TRANSITIVEPROPERTY: &str = "http://www.w3.org/2002/07/owl#TransitiveProperty";

/// Compute the entailment closure of `ds` under `regime`.
///
/// Returns a new dataset holding every original quad plus the inferred triples
/// (in the default graph). `Simple`/`RDF` return a faithful copy.
///
/// # Errors
///
/// [`EntailError::Unsupported`] for `OWL-Direct`/`D`; [`EntailError::Build`] if the
/// derived dataset cannot be frozen.
pub fn materialize(ds: &RdfDataset, regime: Regime) -> Result<Arc<RdfDataset>, EntailError> {
    match regime {
        Regime::Simple | Regime::Rdf => copy_of(ds),
        Regime::Rdfs => close(ds, false),
        Regime::OwlRl => close(ds, true),
        Regime::OwlDirect | Regime::D => Err(EntailError::Unsupported(regime)),
    }
}

/// A faithful copy of `ds` (the identity closure).
fn copy_of(ds: &RdfDataset) -> Result<Arc<RdfDataset>, EntailError> {
    let mut b = RdfDatasetBuilder::new();
    b.push_dataset(ds);
    b.freeze().map_err(|e| EntailError::Build(e.to_string()))
}

/// Run the forward chase and emit `original + inferred`.
fn close(ds: &RdfDataset, owl: bool) -> Result<Arc<RdfDataset>, EntailError> {
    let mut interner = Interner::default();

    // Intern the default-graph triples as the seed fact set.
    let mut facts: HashSet<[u32; 3]> = HashSet::new();
    for q in ds.quads() {
        if q.g.is_some() {
            continue; // RDFS/OWL-RL entailment operates over the default graph
        }
        let s = interner.intern(ds.term_value(q.s));
        let p = interner.intern(ds.term_value(q.p));
        let o = interner.intern(ds.term_value(q.o));
        facts.insert([s, p, o]);
    }
    let original = facts.clone();

    let c = Consts::intern(&mut interner);
    chase(&mut facts, &c, &interner, owl);

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

/// Local string→id interner over dataset-independent [`TermValue`]s.
#[derive(Default)]
struct Interner {
    map: HashMap<TermValue, u32>,
    values: Vec<TermValue>,
}

impl Interner {
    fn intern(&mut self, v: TermValue) -> u32 {
        if let Some(&id) = self.map.get(&v) {
            return id;
        }
        let id = u32::try_from(self.values.len()).expect("term count fits u32");
        self.values.push(v.clone());
        self.map.insert(v, id);
        id
    }

    fn intern_iri(&mut self, iri: &str) -> u32 {
        self.intern(TermValue::Iri(iri.to_owned()))
    }

    fn value(&self, id: u32) -> &TermValue {
        &self.values[id as usize]
    }

    /// Whether `id` may occupy a triple *subject* position (an IRI or blank node —
    /// never a literal or triple term reached by an inverse/range rule).
    fn is_subject(&self, id: u32) -> bool {
        matches!(
            self.values[id as usize],
            TermValue::Iri(_) | TermValue::Blank { .. }
        )
    }
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

/// Semi-naive forward chase to a fixpoint. Each round rebuilds small predicate
/// indices over the current facts and fires every rule, adding derived triples;
/// it repeats until a round adds nothing new.
fn chase(facts: &mut HashSet<[u32; 3]>, c: &Consts, interner: &Interner, owl: bool) {
    loop {
        // Per-round indices.
        let mut by_pred: HashMap<u32, Vec<(u32, u32)>> = HashMap::new();
        let mut sco_by_left: HashMap<u32, Vec<u32>> = HashMap::new();
        let mut spo_by_left: HashMap<u32, Vec<u32>> = HashMap::new();
        let mut type_by_class: HashMap<u32, Vec<u32>> = HashMap::new();
        for &[s, p, o] in facts.iter() {
            by_pred.entry(p).or_default().push((s, o));
            if p == c.sco {
                sco_by_left.entry(s).or_default().push(o);
            } else if p == c.spo {
                spo_by_left.entry(s).or_default().push(o);
            } else if p == c.ty {
                type_by_class.entry(o).or_default().push(s);
            }
        }

        let mut derived: Vec<[u32; 3]> = Vec::new();
        let mut add = |t: [u32; 3]| derived.push(t);

        // rdfs11 / scm-sco: subClassOf transitivity.
        for (&cc, ds) in &sco_by_left {
            for &d in ds {
                if let Some(es) = sco_by_left.get(&d) {
                    for &e in es {
                        add([cc, c.sco, e]);
                    }
                }
            }
        }
        // rdfs9 / cax-sco: (c subClassOf d), (s a c) ⇒ (s a d).
        for (&cc, ds) in &sco_by_left {
            if let Some(instances) = type_by_class.get(&cc) {
                for &d in ds {
                    for &s in instances {
                        add([s, c.ty, d]);
                    }
                }
            }
        }
        // rdfs5 / scm-spo: subPropertyOf transitivity.
        for (&p, qs) in &spo_by_left {
            for &q in qs {
                if let Some(rs) = spo_by_left.get(&q) {
                    for &r in rs {
                        add([p, c.spo, r]);
                    }
                }
            }
        }
        // rdfs7 / prp-spo1: (p subPropertyOf q), (s p o) ⇒ (s q o).
        for (&p, qs) in &spo_by_left {
            if let Some(pairs) = by_pred.get(&p) {
                for &q in qs {
                    for &(s, o) in pairs {
                        add([s, q, o]);
                    }
                }
            }
        }
        // rdfs2 / prp-dom: (p domain c), (s p o) ⇒ (s a c).
        if let Some(doms) = by_pred.get(&c.dom) {
            for &(p, cc) in doms {
                if let Some(pairs) = by_pred.get(&p) {
                    for &(s, _o) in pairs {
                        add([s, c.ty, cc]);
                    }
                }
            }
        }
        // rdfs3 / prp-rng: (p range c), (s p o) ⇒ (o a c) — only when o can be a subject.
        if let Some(rngs) = by_pred.get(&c.rng) {
            for &(p, cc) in rngs {
                if let Some(pairs) = by_pred.get(&p) {
                    for &(_s, o) in pairs {
                        if interner.is_subject(o) {
                            add([o, c.ty, cc]);
                        }
                    }
                }
            }
        }
        // rdfs6: (p a rdf:Property) ⇒ (p subPropertyOf p).
        // rdfs10: (c a rdfs:Class) ⇒ (c subClassOf c); rdfs8: (c a Class) ⇒ (c sco Resource).
        if let Some(props) = type_by_class.get(&c.property) {
            for &p in props {
                add([p, c.spo, p]);
            }
        }
        if let Some(classes) = type_by_class.get(&c.class) {
            for &cc in classes {
                add([cc, c.sco, cc]);
                add([cc, c.sco, c.resource]);
            }
        }
        // rdf1 + rdfs6: every predicate is a property, reflexively a subProperty of
        // itself. scm-sco / scm-spo: the endpoints of subClassOf / subPropertyOf are
        // reflexively related to themselves (their domain/range is axiomatically
        // rdfs:Class / rdf:Property, so rdfs10/rdfs6 make them reflexive).
        for &p in by_pred.keys() {
            add([p, c.spo, p]);
        }
        for (&cc, ds) in &sco_by_left {
            add([cc, c.sco, cc]);
            for &d in ds {
                add([d, c.sco, d]);
            }
        }
        for (&p, qs) in &spo_by_left {
            add([p, c.spo, p]);
            for &q in qs {
                add([q, c.spo, q]);
            }
        }

        if owl {
            // scm-eqc: equivalentClass ⇒ mutual subClassOf.
            if let Some(pairs) = by_pred.get(&c.eq_class) {
                for &(a, b) in pairs {
                    add([a, c.sco, b]);
                    add([b, c.sco, a]);
                }
            }
            // scm-eqp: equivalentProperty ⇒ mutual subPropertyOf.
            if let Some(pairs) = by_pred.get(&c.eq_prop) {
                for &(a, b) in pairs {
                    add([a, c.spo, b]);
                    add([b, c.spo, a]);
                }
            }
            // prp-symp: (p a SymmetricProperty), (x p y) ⇒ (y p x).
            if let Some(syms) = type_by_class.get(&c.symmetric) {
                for &p in syms {
                    if let Some(pairs) = by_pred.get(&p) {
                        for &(x, y) in pairs {
                            if interner.is_subject(y) {
                                add([y, p, x]);
                            }
                        }
                    }
                }
            }
            // prp-trp: (p a TransitiveProperty), (x p y),(y p z) ⇒ (x p z).
            if let Some(trs) = type_by_class.get(&c.transitive) {
                for &p in trs {
                    if let Some(pairs) = by_pred.get(&p) {
                        let mut by_left: HashMap<u32, Vec<u32>> = HashMap::new();
                        for &(x, y) in pairs {
                            by_left.entry(x).or_default().push(y);
                        }
                        for &(x, y) in pairs {
                            if let Some(zs) = by_left.get(&y) {
                                for &z in zs {
                                    add([x, p, z]);
                                }
                            }
                        }
                    }
                }
            }
            // prp-inv: (p inverseOf q) ⇒ ((x p y) ⇒ (y q x)) and the converse.
            if let Some(pairs) = by_pred.get(&c.inverse_of) {
                for &(p, q) in pairs {
                    if let Some(ps) = by_pred.get(&p) {
                        for &(x, y) in ps {
                            if interner.is_subject(y) {
                                add([y, q, x]);
                            }
                        }
                    }
                    if let Some(qs) = by_pred.get(&q) {
                        for &(x, y) in qs {
                            if interner.is_subject(y) {
                                add([y, p, x]);
                            }
                        }
                    }
                }
            }
        }

        let mut changed = false;
        for t in derived {
            if facts.insert(t) {
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
}

/// Intern a [`TermValue`] into `b`, returning its dataset-local id.
fn intern_into(b: &mut RdfDatasetBuilder, v: &TermValue) -> purrdf_core::TermId {
    match v {
        TermValue::Iri(iri) => b.intern_iri(iri),
        TermValue::Blank { label, scope } => b.intern_blank(label, *scope),
        TermValue::Literal {
            lexical_form,
            datatype,
            language,
            ..
        } => {
            let lit = if let Some(lang) = language {
                RdfLiteral::language_tagged(lexical_form, lang)
            } else {
                RdfLiteral::typed(lexical_form, datatype)
            };
            b.intern_literal(lit)
        }
        // RDFS/OWL-RL rules never derive a triple term in a subject/object slot.
        TermValue::Triple { .. } => b.intern_iri(RDFS_RESOURCE),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use purrdf_core::TermRef;

    fn iri(b: &mut RdfDatasetBuilder, s: &str) -> purrdf_core::TermId {
        b.intern_iri(s)
    }

    /// Build a dataset from `(s, p, o)` IRI triples in the default graph.
    fn dataset(triples: &[(&str, &str, &str)]) -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        for (s, p, o) in triples {
            let s = iri(&mut b, s);
            let p = iri(&mut b, p);
            let o = iri(&mut b, o);
            b.push_quad(s, p, o, None);
        }
        b.freeze().expect("freeze")
    }

    fn has(ds: &RdfDataset, s: &str, p: &str, o: &str) -> bool {
        ds.quad_refs().any(|q| {
            matches!(q.s, TermRef::Iri(si) if si == s)
                && matches!(q.p, TermRef::Iri(pi) if pi == p)
                && matches!(q.o, TermRef::Iri(oi) if oi == o)
        })
    }

    const A: &str = "http://example.org/A";
    const B: &str = "http://example.org/B";
    const C: &str = "http://example.org/C";
    const X: &str = "http://example.org/x";

    #[test]
    fn rdfs_subclass_is_transitive_and_types_instances() {
        // A ⊑ B ⊑ C, x a A  ⇒  A ⊑ C, x a B, x a C.
        let ds = dataset(&[
            (A, RDFS_SUBCLASSOF, B),
            (B, RDFS_SUBCLASSOF, C),
            (X, RDF_TYPE, A),
        ]);
        let closed = materialize(&ds, Regime::Rdfs).expect("rdfs");
        assert!(
            has(&closed, A, RDFS_SUBCLASSOF, C),
            "subClassOf transitivity"
        );
        assert!(has(&closed, X, RDF_TYPE, B), "rdfs9 one hop");
        assert!(has(&closed, X, RDF_TYPE, C), "rdfs9 transitive typing");
    }

    #[test]
    fn rdfs_domain_and_range_type_endpoints() {
        // (p domain A),(p range B),(x p y) ⇒ (x a A),(y a B).
        let p = "http://example.org/p";
        let y = "http://example.org/y";
        let ds = dataset(&[(p, RDFS_DOMAIN, A), (p, RDFS_RANGE, B), (X, p, y)]);
        let closed = materialize(&ds, Regime::Rdfs).expect("rdfs");
        assert!(has(&closed, X, RDF_TYPE, A), "domain types subject");
        assert!(has(&closed, y, RDF_TYPE, B), "range types object");
    }

    #[test]
    fn owl_transitive_and_symmetric() {
        let p = "http://example.org/rel";
        let y = "http://example.org/y";
        let z = "http://example.org/z";
        let ds = dataset(&[
            (p, RDF_TYPE, OWL_TRANSITIVEPROPERTY),
            (p, RDF_TYPE, OWL_SYMMETRICPROPERTY),
            (X, p, y),
            (y, p, z),
        ]);
        let closed = materialize(&ds, Regime::OwlRl).expect("owl-rl");
        assert!(has(&closed, X, p, z), "transitive closure");
        assert!(has(&closed, y, p, X), "symmetric mirror");
        // RDFS-only must NOT apply the OWL rules.
        let rdfs = materialize(&ds, Regime::Rdfs).expect("rdfs");
        assert!(!has(&rdfs, X, p, z), "no transitive under RDFS regime");
    }

    #[test]
    fn owl_direct_and_d_are_unsupported() {
        let ds = dataset(&[(X, RDF_TYPE, A)]);
        assert!(matches!(
            materialize(&ds, Regime::OwlDirect),
            Err(EntailError::Unsupported(Regime::OwlDirect))
        ));
        assert!(matches!(
            materialize(&ds, Regime::D),
            Err(EntailError::Unsupported(Regime::D))
        ));
    }

    #[test]
    fn simple_regime_is_identity() {
        let ds = dataset(&[(A, RDFS_SUBCLASSOF, B), (X, RDF_TYPE, A)]);
        let closed = materialize(&ds, Regime::Simple).expect("simple");
        // No inference: x is not typed B.
        assert!(!has(&closed, X, RDF_TYPE, B));
        assert!(has(&closed, X, RDF_TYPE, A));
    }
}
