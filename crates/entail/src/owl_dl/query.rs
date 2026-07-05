// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Query-directed OWL-Direct materialization.
//!
//! `OWL-Direct` is open-world: unlike the RDFS / OWL-RL chase there is no finite
//! "closure graph" whose simple-entailment matching answers every query. Instead the
//! reasoner is handed the query's basic graph pattern and produces an *augmented*
//! dataset whose simple-entailment answers coincide with the OWL Direct-Semantics
//! answers **for that query** — so the unmodified SPARQL evaluator, run over the
//! augmentation, yields the certain answers.
//!
//! Three augmentations are injected, each an entailed fact (never a fabricated one):
//!
//! 1. **Classification + realization** of the data's named vocabulary — every entailed
//!    `C rdfs:subClassOf D` between named classes (reflexive and `owl:Nothing`/`owl:Thing`
//!    included) and every entailed `i rdf:type C` — so `?c`/`?x`-quantified type and
//!    subclass patterns range over the reasoned vocabulary.
//! 2. **Query class-expression retrieval** — for each `(_, rdf:type, R)` /
//!    `(?c, rdfs:subClassOf, R)` / `(R, rdfs:subClassOf, ?c)` whose `R` is an (anonymous)
//!    class expression written in the query, the class expression is parsed with the
//!    shared [`CeExtractor`], its instances (or sub/super named classes) are computed by
//!    the tableau, and `R`'s defining sub-graph is re-materialized under a fresh blank
//!    `X` with the entailed `i rdf:type X` / `C rdfs:subClassOf X` edges — so the query's
//!    own bnode class expression binds to `X`.
//! 3. **`owl:sameAs`** equality closure over individuals (reflexive, and every asserted
//!    triple re-stated over equal individuals), plus `rdfs:domain`/`rdfs:range` answers
//!    for a queried property.
//!
//! Determinism: named classes and individuals are visited in interned-id order, tasks in
//! query order, and every fresh blank is numbered from a single counter, so the augmented
//! dataset is byte-for-byte reproducible.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use purrdf_core::{RdfDataset, RdfDatasetBuilder, TermId, TermValue};

use crate::EntailError;
use crate::interner::{Interner, intern_into};
use crate::owl_dl::Kb;
use crate::owl_dl::concept::{Concept, Role};
use crate::owl_dl::parser::{CeExtractor, TripleIndex, Vocab, index_insert};
use crate::vocab::{OWL_SAMEAS, RDF_TYPE, RDFS_DOMAIN, RDFS_RANGE, RDFS_SUBCLASSOF};

/// A node of a query basic-graph-pattern triple: a variable (by name) or a concrete
/// RDF term. Blank nodes in the query are concrete terms ([`QNode::Term`] wrapping a
/// [`TermValue::Blank`]); the evaluator treats them as non-distinguished variables, but
/// here they are the ground scaffold of a class expression.
#[derive(Debug, Clone)]
pub enum QNode {
    /// A query variable, by its name (the part after `?`/`$`).
    Var(String),
    /// A concrete term (IRI, blank node, or literal).
    Term(TermValue),
}

/// One query triple pattern in the neutral representation the DL layer consumes (so the
/// entailment crate needs no dependency on the SPARQL algebra).
#[derive(Debug, Clone)]
pub struct QTriple {
    /// The subject node.
    pub s: QNode,
    /// The predicate node.
    pub p: QNode,
    /// The object node.
    pub o: QNode,
}

/// A generator of fresh, collision-resistant blank-node labels for re-materialized
/// class expressions.
struct Fresh {
    next: u64,
}

impl Fresh {
    fn new() -> Self {
        Self { next: 0 }
    }

    fn blank(&mut self, b: &mut RdfDatasetBuilder) -> TermId {
        let label = format!("purrdfDLq{}", self.next);
        self.next += 1;
        b.intern_blank(&label, purrdf_core::BlankScope::DEFAULT)
    }
}

/// A query-directed injection task discovered by scanning the query BGP.
enum Task {
    /// `(_, rdf:type, R)` — retrieve `instances_of(concept)` and type them under a
    /// fresh re-materialization of the class expression rooted at `ce_node`.
    TypeCe { ce_node: u32, concept: u32 },
    /// `(?c, rdfs:subClassOf, R)` — every named class `⊑ concept`, as a subclass of a
    /// fresh re-materialization of `ce_node`.
    SubOfCe { ce_node: u32, concept: u32 },
    /// `(R, rdfs:subClassOf, ?c)` — every named class `⊒ concept`, as a superclass of a
    /// fresh re-materialization of `ce_node`.
    SuperOfCe { ce_node: u32, concept: u32 },
    /// `(P, rdfs:domain, ?c)` — every named class entailed as a domain of property `P`.
    Domain { prop: u32, exists: u32 },
    /// `(P, rdfs:range, ?c)` — every named class entailed as a range of property `P`;
    /// `ranges` pairs each candidate class IRI with the interned `∀P.C` concept.
    Range { prop: u32, ranges: Vec<(u32, u32)> },
}

/// Compute the query-directed OWL-Direct augmentation of `ds` for the basic graph
/// pattern `query_bgp`, returning a dataset whose simple-entailment answers to that
/// query are the OWL Direct-Semantics certain answers.
///
/// # Errors
///
/// [`EntailError::Inconsistent`] if the data is inconsistent (every query would then be
/// entailed, so there is no meaningful answer set); [`EntailError::Parse`] on a
/// malformed class-expression graph; [`EntailError::Build`] on tableau step-cap
/// exhaustion.
pub fn materialize_dl(
    ds: &RdfDataset,
    query_bgp: &[QTriple],
) -> Result<Arc<RdfDataset>, EntailError> {
    let mut kb = Kb::from_dataset(ds)?;
    if !kb.is_consistent()? {
        return Err(EntailError::Inconsistent);
    }
    let v = Vocab::intern(&mut kb.interner);

    // A `subject → predicate → objects` index over the data (for named-class discovery)
    // and the ground scaffold of the query's class expressions.
    let data_index = build_data_index(ds, &mut kb.interner);
    let named_classes = collect_named_classes(&kb.interner, &data_index, &v);

    // Intern every ground query term and index the all-ground query triples.
    let mut q_index: TripleIndex = BTreeMap::new();
    let resolved: Vec<(Option<u32>, Option<u32>, Option<u32>)> = query_bgp
        .iter()
        .map(|t| {
            let s = resolve_node(&mut kb.interner, &t.s);
            let p = resolve_node(&mut kb.interner, &t.p);
            let o = resolve_node(&mut kb.interner, &t.o);
            if let (Some(s), Some(p), Some(o)) = (s, p, o) {
                index_insert(&mut q_index, s, p, o);
            }
            (s, p, o)
        })
        .collect();

    // Extract query class expressions (borrows the interner immutably; the concept
    // table is a disjoint field, interned below).
    let raw_tasks = extract_tasks(&kb.interner, &q_index, &v, &resolved)?;

    // Intern all concepts we must reason about, then finalize the negation cache once.
    // `owl:Thing`/`owl:Nothing` reason as `⊤`/`⊥`, never as opaque atomic classes.
    let named_cid: BTreeMap<u32, u32> = named_classes
        .iter()
        .map(|&c| (c, kb.table.intern(class_concept(&v, c))))
        .collect();
    let tasks = intern_tasks(&mut kb.table, &v, &named_classes, raw_tasks);
    kb.finalize();

    // Build the output: the data verbatim, plus every entailed augmentation.
    let mut b = RdfDatasetBuilder::new();
    b.push_dataset(ds);
    let mut fresh = Fresh::new();

    inject_classification(&mut b, &kb, &named_cid)?;
    inject_realization(&mut b, &kb, &named_cid)?;
    inject_same_as(&mut b, &kb, &data_index);
    inject_tasks(&mut b, &kb, &q_index, &named_cid, &tasks, &mut fresh)?;

    b.freeze()
        .map_err(|e| EntailError::Build(format!("freeze augmented dataset: {e}")))
}

/// Resolve a query node to an interned id (a variable yields `None`).
fn resolve_node(interner: &mut Interner, node: &QNode) -> Option<u32> {
    match node {
        QNode::Var(_) => None,
        QNode::Term(tv) => Some(interner.intern(tv.clone())),
    }
}

/// Index the data's default-graph triples over the (already-populated) interner.
fn build_data_index(ds: &RdfDataset, interner: &mut Interner) -> TripleIndex {
    let mut index: TripleIndex = BTreeMap::new();
    for q in ds.quads() {
        if q.g.is_some() {
            continue;
        }
        let s = interner.intern(ds.term_value(q.s));
        let p = interner.intern(ds.term_value(q.p));
        let o = interner.intern(ds.term_value(q.o));
        index_insert(&mut index, s, p, o);
    }
    index
}

/// The set of named (IRI) classes in the data — every IRI that appears in a
/// class-denoting position — plus `owl:Thing` and `owl:Nothing`, in id order.
fn collect_named_classes(interner: &Interner, index: &TripleIndex, v: &Vocab) -> BTreeSet<u32> {
    let mut out = BTreeSet::new();
    out.insert(v.thing);
    out.insert(v.nothing);
    let is_iri = |id: u32| matches!(interner.value(id), TermValue::Iri(_));
    for (&s, preds) in index {
        for (&p, objs) in preds {
            for &o in objs {
                if p == v.ty {
                    if o == v.class {
                        // `s a owl:Class` — a named class declaration.
                        if is_iri(s) {
                            out.insert(s);
                        }
                    } else if !v.structural_types.contains(&o) && is_iri(o) {
                        // The object of a non-structural rdf:type is a named class.
                        out.insert(o);
                    }
                } else if p == v.sub_class || p == v.equiv_class || p == v.disjoint {
                    if is_iri(s) {
                        out.insert(s);
                    }
                    if is_iri(o) {
                        out.insert(o);
                    }
                } else if (p == v.domain || p == v.range) && is_iri(o) {
                    out.insert(o);
                }
            }
        }
    }
    out
}

/// A task before its concepts are interned into the concept table.
enum RawTask {
    TypeCe { ce_node: u32, concept: Concept },
    SubOfCe { ce_node: u32, concept: Concept },
    SuperOfCe { ce_node: u32, concept: Concept },
    Domain { prop: u32 },
    Range { prop: u32 },
}

/// Scan the resolved query triples for class-expression / domain / range patterns,
/// returning the raw tasks and the concepts they reference (in query order).
fn extract_tasks(
    interner: &Interner,
    q_index: &TripleIndex,
    v: &Vocab,
    resolved: &[(Option<u32>, Option<u32>, Option<u32>)],
) -> Result<Vec<RawTask>, EntailError> {
    let mut ce = CeExtractor::new(q_index, interner, v);
    let mut tasks = Vec::new();
    let mut seen: BTreeSet<(u8, u32)> = BTreeSet::new();
    for &(s, p, o) in resolved {
        if p == Some(v.ty) {
            if let Some(oid) = o
                && ce.is_class_expression(oid)
                && seen.insert((0, oid))
            {
                tasks.push(RawTask::TypeCe {
                    ce_node: oid,
                    concept: ce.expr(oid)?,
                });
            }
        } else if p == Some(v.sub_class) {
            if let Some(oid) = o
                && ce.is_class_expression(oid)
                && seen.insert((1, oid))
            {
                tasks.push(RawTask::SubOfCe {
                    ce_node: oid,
                    concept: ce.expr(oid)?,
                });
                continue;
            }
            if let Some(sid) = s
                && ce.is_class_expression(sid)
                && seen.insert((2, sid))
            {
                tasks.push(RawTask::SuperOfCe {
                    ce_node: sid,
                    concept: ce.expr(sid)?,
                });
            }
        } else if p == Some(v.domain) {
            if let (Some(sid), None) = (s, o)
                && seen.insert((3, sid))
            {
                tasks.push(RawTask::Domain { prop: sid });
            }
        } else if p == Some(v.range)
            && let (Some(sid), None) = (s, o)
            && seen.insert((4, sid))
        {
            tasks.push(RawTask::Range { prop: sid });
        }
    }
    Ok(tasks)
}

/// The concept a named class IRI denotes: `⊤` for `owl:Thing`, `⊥` for `owl:Nothing`,
/// else the atomic named class.
fn class_concept(v: &Vocab, c: u32) -> Concept {
    if c == v.thing {
        Concept::Top
    } else if c == v.nothing {
        Concept::Bottom
    } else {
        Concept::Named(c)
    }
}

/// Intern each raw task's concepts into the concept table, yielding concept-id tasks.
fn intern_tasks(
    table: &mut crate::owl_dl::concept::ConceptTable,
    v: &Vocab,
    named_classes: &BTreeSet<u32>,
    raw: Vec<RawTask>,
) -> Vec<Task> {
    raw.into_iter()
        .map(|t| match t {
            RawTask::TypeCe { ce_node, concept } => Task::TypeCe {
                ce_node,
                concept: table.intern(concept),
            },
            RawTask::SubOfCe { ce_node, concept } => Task::SubOfCe {
                ce_node,
                concept: table.intern(concept),
            },
            RawTask::SuperOfCe { ce_node, concept } => Task::SuperOfCe {
                ce_node,
                concept: table.intern(concept),
            },
            RawTask::Domain { prop } => {
                let exists = table.intern(Concept::Some(Role::Named(prop), Box::new(Concept::Top)));
                Task::Domain { prop, exists }
            }
            RawTask::Range { prop } => {
                let ranges = named_classes
                    .iter()
                    .map(|&c| {
                        let all = table.intern(Concept::All(
                            Role::Named(prop),
                            Box::new(class_concept(v, c)),
                        ));
                        (c, all)
                    })
                    .collect();
                Task::Range { prop, ranges }
            }
        })
        .collect()
}

/// Inject every entailed `C rdfs:subClassOf D` between named classes.
fn inject_classification(
    b: &mut RdfDatasetBuilder,
    kb: &Kb,
    named_cid: &BTreeMap<u32, u32>,
) -> Result<(), EntailError> {
    let sub_class = b.intern_iri(RDFS_SUBCLASSOF);
    for (&c_iri, &c_cid) in named_cid {
        for (&d_iri, &d_cid) in named_cid {
            if kb.entails_subclass(c_cid, d_cid)? {
                let s = intern_into(b, kb.interner.value(c_iri));
                let o = intern_into(b, kb.interner.value(d_iri));
                b.push_quad(s, sub_class, o, None);
            }
        }
    }
    Ok(())
}

/// Inject every entailed `i rdf:type C` for a named class `C`.
fn inject_realization(
    b: &mut RdfDatasetBuilder,
    kb: &Kb,
    named_cid: &BTreeMap<u32, u32>,
) -> Result<(), EntailError> {
    let ty = b.intern_iri(RDF_TYPE);
    for &ind in &kb.individuals {
        for (&c_iri, &c_cid) in named_cid {
            if kb.entails_instance(ind, c_cid)? {
                let s = intern_into(b, kb.interner.value(ind));
                let o = intern_into(b, kb.interner.value(c_iri));
                b.push_quad(s, ty, o, None);
            }
        }
    }
    Ok(())
}

/// Inject the `owl:sameAs` equality closure over individuals: reflexive `i sameAs i`,
/// every equal pair, and every asserted data triple re-stated over equal endpoints.
fn inject_same_as(b: &mut RdfDatasetBuilder, kb: &Kb, data_index: &TripleIndex) {
    let same_as = b.intern_iri(OWL_SAMEAS);
    let uf = EqClasses::build(&kb.individuals, &kb.same_as);

    // Reflexive + symmetric-transitive closure as explicit sameAs triples.
    for &i in &kb.individuals {
        for &j in &uf.members(i) {
            let s = intern_into(b, kb.interner.value(i));
            let o = intern_into(b, kb.interner.value(j));
            b.push_quad(s, same_as, o, None);
        }
    }

    // Re-state every asserted data triple over each combination of equal endpoints.
    for (&s, preds) in data_index {
        for (&p, objs) in preds {
            for &o in objs {
                let s_class = uf.members(s);
                let o_class = uf.members(o);
                if s_class.len() == 1 && o_class.len() == 1 {
                    continue; // nothing new to state
                }
                let p_id = intern_into(b, kb.interner.value(p));
                for &s2 in &s_class {
                    for &o2 in &o_class {
                        if s2 == s && o2 == o {
                            continue;
                        }
                        let s_id = intern_into(b, kb.interner.value(s2));
                        let o_id = intern_into(b, kb.interner.value(o2));
                        b.push_quad(s_id, p_id, o_id, None);
                    }
                }
            }
        }
    }
}

/// Inject every query-directed class-expression / domain / range task.
fn inject_tasks(
    b: &mut RdfDatasetBuilder,
    kb: &Kb,
    q_index: &TripleIndex,
    named_cid: &BTreeMap<u32, u32>,
    tasks: &[Task],
    fresh: &mut Fresh,
) -> Result<(), EntailError> {
    let ty = b.intern_iri(RDF_TYPE);
    let sub_class = b.intern_iri(RDFS_SUBCLASSOF);
    let domain = b.intern_iri(RDFS_DOMAIN);
    let range = b.intern_iri(RDFS_RANGE);
    for task in tasks {
        match *task {
            Task::TypeCe { ce_node, concept } => {
                let instances = kb.instances_of(concept)?;
                if instances.is_empty() {
                    continue;
                }
                let x = reconstruct(b, &kb.interner, q_index, ce_node, fresh);
                for i in instances {
                    let s = intern_into(b, kb.interner.value(i));
                    b.push_quad(s, ty, x, None);
                }
            }
            Task::SubOfCe { ce_node, concept } => {
                let subs = subclass_matches(kb, named_cid, concept, true)?;
                if subs.is_empty() {
                    continue;
                }
                let x = reconstruct(b, &kb.interner, q_index, ce_node, fresh);
                for c_iri in subs {
                    let s = intern_into(b, kb.interner.value(c_iri));
                    b.push_quad(s, sub_class, x, None);
                }
            }
            Task::SuperOfCe { ce_node, concept } => {
                let sups = subclass_matches(kb, named_cid, concept, false)?;
                if sups.is_empty() {
                    continue;
                }
                let x = reconstruct(b, &kb.interner, q_index, ce_node, fresh);
                for c_iri in sups {
                    let o = intern_into(b, kb.interner.value(c_iri));
                    b.push_quad(x, sub_class, o, None);
                }
            }
            Task::Domain { prop, exists } => {
                let prop_id = intern_into(b, kb.interner.value(prop));
                for (&c_iri, &c_cid) in named_cid {
                    if kb.entails_subclass(exists, c_cid)? {
                        let o = intern_into(b, kb.interner.value(c_iri));
                        b.push_quad(prop_id, domain, o, None);
                    }
                }
            }
            Task::Range { prop, ref ranges } => {
                let prop_id = intern_into(b, kb.interner.value(prop));
                for &(c_iri, all_id) in ranges {
                    if kb.entails_subclass(kb.top, all_id)? {
                        let o = intern_into(b, kb.interner.value(c_iri));
                        b.push_quad(prop_id, range, o, None);
                    }
                }
            }
        }
    }
    Ok(())
}

/// The named classes that are a sub- (`want_sub`) or super-class of `concept`.
fn subclass_matches(
    kb: &Kb,
    named_cid: &BTreeMap<u32, u32>,
    concept: u32,
    want_sub: bool,
) -> Result<Vec<u32>, EntailError> {
    let mut out = Vec::new();
    for (&c_iri, &c_cid) in named_cid {
        let holds = if want_sub {
            kb.entails_subclass(c_cid, concept)?
        } else {
            kb.entails_subclass(concept, c_cid)?
        };
        if holds {
            out.push(c_iri);
        }
    }
    Ok(out)
}

/// Re-materialize the class-expression sub-graph rooted at `root` under a fresh blank
/// `X`: every reachable query blank is renamed to a fresh builder blank, IRIs/literals
/// are copied verbatim, and every defining triple is re-stated. Returns `X`.
fn reconstruct(
    b: &mut RdfDatasetBuilder,
    interner: &Interner,
    q_index: &TripleIndex,
    root: u32,
    fresh: &mut Fresh,
) -> TermId {
    // The blank scaffold nodes reachable from `root` (only blanks are renamed).
    let mut scaffold: BTreeSet<u32> = BTreeSet::new();
    let mut stack = vec![root];
    while let Some(n) = stack.pop() {
        if !matches!(interner.value(n), TermValue::Blank { .. }) {
            continue;
        }
        if !scaffold.insert(n) {
            continue;
        }
        if let Some(preds) = q_index.get(&n) {
            for objs in preds.values() {
                for &o in objs {
                    stack.push(o);
                }
            }
        }
    }
    // Assign fresh blanks in id order (deterministic labelling).
    let mut rename: BTreeMap<u32, TermId> = BTreeMap::new();
    for &n in &scaffold {
        let fb = fresh.blank(b);
        rename.insert(n, fb);
    }
    // Re-state the defining triples of each scaffold node.
    for &n in &scaffold {
        let s_id = rename[&n];
        if let Some(preds) = q_index.get(&n) {
            for (&p, objs) in preds {
                let p_id = intern_into(b, interner.value(p));
                for &o in objs {
                    let o_id = rename
                        .get(&o)
                        .copied()
                        .unwrap_or_else(|| intern_into(b, interner.value(o)));
                    b.push_quad(s_id, p_id, o_id, None);
                }
            }
        }
    }
    // A blank root is renamed to its fresh scaffold node; a non-blank root (an IRI
    // class expression, e.g. a named class carrying restriction triples) keeps its
    // own identity rather than being renamed away.
    rename
        .get(&root)
        .copied()
        .unwrap_or_else(|| intern_into(b, interner.value(root)))
}

/// A union-find over individuals, seeded by `owl:sameAs` pairs, exposing each
/// individual's equality class as a sorted slice.
struct EqClasses {
    /// individual id → sorted members of its equality class (each member maps to the
    /// same shared vector via the representative).
    classes: BTreeMap<u32, Vec<u32>>,
}

impl EqClasses {
    fn build(individuals: &BTreeSet<u32>, same_as: &[(u32, u32)]) -> Self {
        // Simple union-find keyed by id.
        let mut parent: BTreeMap<u32, u32> = individuals.iter().map(|&i| (i, i)).collect();
        for &(a, b) in same_as {
            parent.entry(a).or_insert(a);
            parent.entry(b).or_insert(b);
        }
        fn find(parent: &mut BTreeMap<u32, u32>, x: u32) -> u32 {
            let mut r = x;
            while parent[&r] != r {
                r = parent[&r];
            }
            let mut c = x;
            while parent[&c] != r {
                let next = parent[&c];
                parent.insert(c, r);
                c = next;
            }
            r
        }
        for &(a, b) in same_as {
            let ra = find(&mut parent, a);
            let rb = find(&mut parent, b);
            if ra != rb {
                parent.insert(ra, rb);
            }
        }
        let keys: Vec<u32> = parent.keys().copied().collect();
        let mut members: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
        for &k in &keys {
            let r = find(&mut parent, k);
            members.entry(r).or_default().push(k);
        }
        for v in members.values_mut() {
            v.sort_unstable();
            v.dedup();
        }
        let mut classes: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
        for &k in &keys {
            let r = find(&mut parent, k);
            classes.insert(k, members[&r].clone());
        }
        Self { classes }
    }

    /// The (sorted) equality class of `i`, or the singleton `[i]` when `i` is not a
    /// recorded individual (e.g. a class IRI or literal endpoint).
    fn members(&self, i: u32) -> Vec<u32> {
        self.classes.get(&i).cloned().unwrap_or_else(|| vec![i])
    }
}
