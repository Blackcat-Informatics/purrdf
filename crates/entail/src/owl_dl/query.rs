// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Query-directed OWL-Direct materialization.
//!
//! `OWL-Direct` is open-world: unlike the RDFS / OWL-RL chase there is no finite "closure
//! graph" whose simple-entailment matching answers every query. Instead the reasoner is
//! handed the query's basic graph pattern and produces an *augmented* dataset over which
//! the unmodified SPARQL evaluator computes the answers.
//!
//! # Exactly what the augmentation delivers
//!
//! This used to be described as "a dataset whose simple-entailment answers coincide with
//! the Direct-Semantics answers for that query", full stop. That claim is stronger than
//! any finite augmentation can support, and nothing tested it. What holds, and what is
//! tested, is narrower and stated here rather than in a design note nobody reads.
//!
//! Write `σ` for a substitution mapping the BGP's variables to terms of the **scoping
//! graph** — the named terms of the data. SPARQL's entailment regimes require exactly that
//! (a solution binding a variable to a term outside the scoping graph is not an answer the
//! regime admits), and for such a `σ` the certain answers of a conjunction decompose:
//! `KB ⊨ (t₁ ∧ … ∧ tₙ)σ` iff `KB ⊨ tᵢσ` for every `i`, because each `tᵢσ` is ground. So a
//! dataset holding every entailed ground atom over the query's own vocabulary, matched by
//! simple entailment, yields precisely the certain answers **for a BGP all of whose
//! variables are distinguished**. That is the claim, and it is the one the injections below
//! are built to satisfy.
//!
//! # The residue, and why it is a boundary rather than a footnote
//!
//! A query BLANK NODE is not a distinguished variable — SPARQL reads it as an existential —
//! and for `∃x. A(x) ∧ B(x)` the decomposition above fails outright: an open-world model
//! may satisfy the conjunction through an ANONYMOUS element that no finite augmentation can
//! name. That is not a gap this construction could close by injecting more facts; it is a
//! statement about the shape of the problem. So a query blank node that is not part of a
//! class expression's scaffold raises
//! [`Construct::NonDistinguishedVariable`](crate::Construct::NonDistinguishedVariable), and
//! the run's [`ReasoningReport`] stops saying [`Completeness::Exact`](crate::Completeness).
//! A caller gets a sound answer set and is told, in data, that it may not be complete.
//!
//! # The injections
//!
//! Each is an entailed fact, never a fabricated one, and each is decided by the reasoner
//! rather than pattern-matched out of the data:
//!
//! 1. **Classification + realization** of the data's named vocabulary — every entailed
//!    `C rdfs:subClassOf D` between named classes (reflexive and `owl:Nothing`/`owl:Thing`
//!    included) and every entailed `i rdf:type C` — so `?c`/`?x`-quantified type and
//!    subclass patterns range over the reasoned vocabulary. The subclass half is ONE
//!    consequence-based saturation ([`crate::owl_dl::saturate`]) rather than a refutation per
//!    ordered pair; the residual pairs an out-of-fragment ontology leaves underived still go
//!    to the tableau, so the injected relation is the same one either way.
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
//! 4. **Entailed ROLE assertions** over the properties the query actually names: for each
//!    ordered pair of named individuals `(a, b)` and each queried property `p`, the tableau
//!    decides `KB ⊨ p(a, b)` and the answer is injected when it holds. Without this a
//!    pattern `?x :q ?y` missed every answer the property hierarchy, an inverse, a symmetry
//!    or a transitivity axiom entails but no triple states — the largest hole in the old
//!    construction, and the reason the old claim was not merely unproven but false. It is
//!    query-DIRECTED, so an ontology whose properties the query never mentions pays nothing
//!    for it.
//!
//! # Determinism
//!
//! Named classes and individuals are visited in interned-id order, tasks in query order,
//! queried properties in interned-id order, and every fresh blank is numbered from a single
//! counter, so the augmented dataset is byte-for-byte reproducible.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use purrdf_core::{RdfDataset, RdfDatasetBuilder, TermId, TermValue};

use crate::EntailError;
use crate::interner::{Interner, intern_into};
use crate::owl_dl::concept::{Concept, Role};
use crate::owl_dl::data::DataRangeTable;
use crate::owl_dl::parser::{CeExtractor, TripleIndex, Vocab, index_insert};
use crate::owl_dl::saturate::{Taxonomy, saturate};
use crate::owl_dl::{Kb, class_concept, tableau};
use crate::report::{Construct, ReasoningReport};
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
/// query are the OWL Direct-Semantics certain answers — AND the [`ReasoningReport`] for
/// the run.
///
/// # There is one entry point, and it carries the evidence
///
/// A report-free twin of this function used to sit beside it, delegating here and throwing
/// the report away. That is the shape this crate forbids everywhere else: two entry points
/// where one discards the evidence means the cheap call wins, and nothing downstream can
/// tell that "OWL Direct-Semantics answers" was computed over an ontology holding an
/// `owl:propertyChainAxiom` the reverse mapping could not read. The twin is gone; a caller
/// who wants only the dataset binds `(dataset, _report)`, which costs one `_`.
///
/// # What the boundaries are
///
/// The reverse mapping meets constructs it cannot fully handle — an
/// `owl:propertyChainAxiom`, an `owl:imports`, a datatype restriction, a term of the
/// reserved vocabulary this release does not know — and every one of them used to vanish
/// without a word. They are [`Boundary`](crate::Boundary)s now, joined by the two this
/// layer's own SHAPE raises: [`Construct::NonDistinguishedVariable`] for a query blank node
/// that is not a class expression's scaffold, and [`Construct::NamedGraph`] for an input
/// whose quads do not all sit in the default graph. The knowledge base is read from the
/// DEFAULT graph alone (the reverse mapping indexes that graph and no other), so a
/// quad outside it constrains nothing the tableau decides — and a certificate that did not
/// say so would look complete while most of the input had been ignored, which is worse than
/// a wrong answer because nothing signals it.
///
/// The report's [`Completeness`](crate::Completeness) is
/// [`Exact`](crate::Completeness::Exact) when the ontology held nothing this layer could
/// not read, and [`ExactWithinBoundaries`](crate::Completeness::ExactWithinBoundaries) when
/// it did — never `Exact` beside a non-empty boundary list, because
/// [`ReasoningReport::completeness`] DERIVES the value from that very list rather than
/// carrying a second claim beside it.
///
/// # Errors
///
/// [`EntailError::Unsatisfiable`] if the data is unsatisfiable (every query would then be
/// entailed, so there is no meaningful answer set); [`EntailError::Parse`] on a
/// malformed class-expression graph; [`EntailError::Build`] on tableau step-cap
/// exhaustion.
pub fn materialize_dl_reported(
    ds: &RdfDataset,
    query_bgp: &[QTriple],
) -> Result<(Arc<RdfDataset>, ReasoningReport), EntailError> {
    let mut kb = Kb::from_dataset(ds)?;
    if !kb.is_consistent()? {
        return Err(EntailError::Unsatisfiable);
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
    // table is a disjoint field, interned below). A class expression written in the QUERY
    // can meet a boundary just as one written in the data can, so the extractor's
    // boundaries join the knowledge base's.
    let (raw_tasks, query_boundaries) =
        extract_tasks(&kb.interner, &mut kb.data_ranges, &q_index, &v, &resolved)?;
    let mut boundaries = kb.boundaries().clone();
    boundaries.extend(query_boundaries);
    // A data range the query itself wrote lands in the knowledge base's own table, so one the
    // decision procedure cannot decide exactly raises the boundary here for the same reason
    // one written in the data raises it in the reverse mapping.
    if !kb.data_ranges.exactly_decided() {
        boundaries.insert(Construct::DataRange);
    }
    if has_non_distinguished_variable(&kb.interner, &q_index, &raw_tasks, &resolved) {
        boundaries.insert(Construct::NonDistinguishedVariable);
    }
    // The knowledge base was read from the DEFAULT graph alone, so a quad outside it is an
    // axiom this run did not have. Raising the boundary is what stops the certificate
    // describing a complete run over an input most of which was never read.
    if ds.quads().any(|q| q.g.is_some()) {
        boundaries.insert(Construct::NamedGraph);
    }

    // Intern all concepts we must reason about, then finalize the negation cache once.
    // `owl:Thing`/`owl:Nothing` reason as `⊤`/`⊥`, never as opaque atomic classes.
    let named_cid: BTreeMap<u32, u32> = named_classes
        .iter()
        .map(|&c| (c, kb.table.intern(class_concept(&v, c))))
        .collect();
    let roles = intern_queried_roles(&mut kb, &v, &resolved);
    let tasks = intern_tasks(&mut kb.table, &v, &named_classes, raw_tasks);
    kb.finalize();

    // Build the output: the data verbatim, plus every entailed augmentation.
    let mut b = RdfDatasetBuilder::new();
    b.push_dataset(ds);
    let mut fresh = Fresh::new();

    // ONE consequence-based saturation over the whole clause set, shared by the
    // classification injection below. The subsumption relation between named classes is a
    // single fixpoint, not `n²` refutations, and it is computed after the consistency check
    // above because an inconsistent knowledge base entails everything and this calculus
    // reads the TBox only.
    let seeds: Vec<u32> = named_cid.values().copied().collect();
    let taxonomy = saturate(&kb, &seeds);

    inject_classification(&mut b, &kb, &named_cid, &taxonomy)?;
    inject_realization(&mut b, &kb, &named_cid)?;
    inject_roles(&mut b, &kb, &roles, &data_index)?;
    inject_same_as(&mut b, &kb, &data_index);
    inject_tasks(&mut b, &kb, &q_index, &named_cid, &tasks, &mut fresh)?;

    let dataset = b
        .freeze()
        .map_err(|e| EntailError::Build(format!("freeze augmented dataset: {e}")))?;
    Ok((dataset, ReasoningReport::of_dl_run(&boundaries)))
}

/// Resolve a query node to an interned id (a variable yields `None`).
fn resolve_node(interner: &mut Interner, node: &QNode) -> Option<u32> {
    match node {
        QNode::Var(_) => None,
        QNode::Term(tv) => Some(interner.intern(tv.clone())),
    }
}

/// Index the data's default-graph triples over the (already-populated) interner.
pub(crate) fn build_data_index(ds: &RdfDataset, interner: &mut Interner) -> TripleIndex {
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
pub(crate) fn collect_named_classes(
    interner: &Interner,
    index: &TripleIndex,
    v: &Vocab,
) -> BTreeSet<u32> {
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

impl RawTask {
    /// The class-expression node this task is rooted at, for the tasks that have one.
    ///
    /// Read by the non-distinguished-variable check: a query blank node reachable from one
    /// of these roots is GROUND SYNTAX the reverse mapping consumes, not an existential
    /// variable, and must not raise the boundary.
    const fn ce_node(&self) -> Option<u32> {
        match *self {
            Self::TypeCe { ce_node, .. }
            | Self::SubOfCe { ce_node, .. }
            | Self::SuperOfCe { ce_node, .. } => Some(ce_node),
            Self::Domain { .. } | Self::Range { .. } => None,
        }
    }
}

/// Whether the query BGP holds a blank node that is not part of a class expression.
///
/// SPARQL reads a query blank node as an existential — a NON-DISTINGUISHED variable — and
/// the certain answers of a BGP with one do not decompose into per-atom entailment checks;
/// see [`Construct::NonDistinguishedVariable`]'s reason. A blank node that is a class
/// expression's scaffold is a different thing entirely: it is syntax for a class, the
/// reverse mapping reads it as such, and it carries no existential force of its own.
fn has_non_distinguished_variable(
    interner: &Interner,
    q_index: &TripleIndex,
    raw_tasks: &[RawTask],
    resolved: &[(Option<u32>, Option<u32>, Option<u32>)],
) -> bool {
    let mut blanks: BTreeSet<u32> = BTreeSet::new();
    for (s, p, o) in resolved {
        for id in s.iter().chain(p).chain(o) {
            if matches!(interner.value(*id), TermValue::Blank { .. }) {
                blanks.insert(*id);
            }
        }
    }
    if blanks.is_empty() {
        return false;
    }
    let mut scaffold: BTreeSet<u32> = BTreeSet::new();
    for root in raw_tasks.iter().filter_map(RawTask::ce_node) {
        collect_scaffold(interner, q_index, root, &mut scaffold);
    }
    blanks.iter().any(|blank| !scaffold.contains(blank))
}

/// The blank query nodes reachable from `root` through the query index.
///
/// Shared by the non-distinguished-variable check and by [`reconstruct`], which renames
/// exactly this set — so the two can never disagree about what "part of a class
/// expression" means.
fn collect_scaffold(
    interner: &Interner,
    q_index: &TripleIndex,
    root: u32,
    out: &mut BTreeSet<u32>,
) {
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        if !matches!(interner.value(node), TermValue::Blank { .. }) {
            continue;
        }
        if !out.insert(node) {
            continue;
        }
        if let Some(preds) = q_index.get(&node) {
            for objs in preds.values() {
                stack.extend(objs.iter().copied());
            }
        }
    }
}

/// One queried object property, with the interned `∃p.{t}` for every candidate target `t`.
///
/// Interned BEFORE [`Kb::finalize`] so the negation cache covers all of them, which is why
/// this is a value assembled up front rather than a concept built inside the injection
/// loop.
struct QueriedRole {
    /// The property's term id.
    property: u32,
    /// `(target term id, concept id of ∃property.{target})`, ascending by target.
    reaches: Vec<(u32, u32)>,
}

/// The properties the QUERY names, with a `∃p.{t}` interned for every candidate target.
///
/// Query-directed on purpose: deciding `KB ⊨ p(a, b)` costs one tableau run per ordered
/// pair, so ranging over every property of the ontology would make the augmentation
/// quadratic in data the caller never asked about. The properties here are the ones the
/// BGP actually mentions, minus the reserved vocabulary (`rdf:type`, `rdfs:subClassOf`,
/// `owl:sameAs`, `rdfs:domain` and `rdfs:range` have their own injections, and no other
/// reserved term denotes a DL role).
///
/// Targets are the named individuals PLUS the objects of asserted role assertions, which
/// is what brings a data property's LITERAL values into range: `p ⊑ q` with `a p "cat"`
/// entails `a q "cat"`, and a literal is a term of the completion graph even though it is
/// not a realization candidate.
fn intern_queried_roles(
    kb: &mut Kb,
    v: &Vocab,
    resolved: &[(Option<u32>, Option<u32>, Option<u32>)],
) -> Vec<QueriedRole> {
    let mut targets: BTreeSet<u32> = kb.individuals.iter().copied().collect();
    targets.extend(kb.abox_roles.iter().map(|&(_, _, object)| object));
    if targets.is_empty() {
        return Vec::new();
    }
    let mut properties: BTreeSet<u32> = BTreeSet::new();
    for &(_, predicate, _) in resolved {
        let Some(predicate) = predicate else {
            continue;
        };
        if predicate == v.ty
            || predicate == v.sub_class
            || predicate == v.same_as
            || predicate == v.domain
            || predicate == v.range
        {
            continue;
        }
        if let TermValue::Iri(iri) = kb.interner.value(predicate)
            && !crate::owl_dl::constructs::is_reserved(iri)
        {
            properties.insert(predicate);
        }
    }
    let targets: Vec<u32> = targets.into_iter().collect();
    let mut out = Vec::with_capacity(properties.len());
    for property in properties {
        let mut reaches = Vec::with_capacity(targets.len());
        for &target in &targets {
            let concept = kb.table.intern(Concept::Some(
                Role::Named(property),
                Box::new(Concept::nominal(vec![target])),
            ));
            reaches.push((target, concept));
        }
        out.push(QueriedRole { property, reaches });
    }
    out
}

/// Inject every entailed `a p b` for a queried property `p`.
///
/// Decided by refutation — `KB ⊨ p(a, b)` exactly when `KB ∪ {a : ¬∃p.{b}}` is
/// inconsistent — so the property hierarchy, inverses, symmetry, transitivity and the
/// nominal machinery are all consulted by the one procedure that already knows them,
/// rather than by a second, partial closure written beside the tableau.
///
/// A pair the data already asserts is skipped: the assertion is in the output verbatim, so
/// the tableau run would buy nothing.
fn inject_roles(
    b: &mut RdfDatasetBuilder,
    kb: &Kb,
    roles: &[QueriedRole],
    data_index: &TripleIndex,
) -> Result<(), EntailError> {
    for role in roles {
        let asserted = data_index
            .iter()
            .filter_map(|(&s, preds)| preds.get(&role.property).map(|objs| (s, objs)));
        let asserted: BTreeSet<(u32, u32)> = asserted
            .flat_map(|(s, objs)| objs.iter().map(move |&o| (s, o)))
            .collect();
        let property = intern_into(b, kb.interner.value(role.property));
        for &subject in &kb.individuals {
            for &(target, reach) in &role.reaches {
                if asserted.contains(&(subject, target)) {
                    continue;
                }
                let negated = kb.table.negate(reach);
                if tableau::consistent(
                    kb,
                    &tableau::Assumptions {
                        types: &[(subject, negated)],
                        ..tableau::Assumptions::of_kb()
                    },
                )? {
                    continue;
                }
                let s = intern_into(b, kb.interner.value(subject));
                let o = intern_into(b, kb.interner.value(target));
                b.push_quad(s, property, o, None);
            }
        }
    }
    Ok(())
}

/// Scan the resolved query triples for class-expression / domain / range patterns,
/// returning the raw tasks and the concepts they reference (in query order).
fn extract_tasks(
    interner: &Interner,
    ranges: &mut DataRangeTable,
    q_index: &TripleIndex,
    v: &Vocab,
    resolved: &[(Option<u32>, Option<u32>, Option<u32>)],
) -> Result<(Vec<RawTask>, BTreeSet<Construct>), EntailError> {
    let mut ce = CeExtractor::new(q_index, interner, v, ranges);
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
    let boundaries = ce.boundaries().clone();
    Ok((tasks, boundaries))
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
///
/// The relation comes from the ONE saturation the caller already ran, not from a second
/// `n²` sweep of the tableau. The emitted triples are unchanged — reflexive `C ⊑ C` and the
/// `owl:Thing`/`owl:Nothing` edges included, because the augmentation's claim is to hold
/// every entailed ground atom over the query's vocabulary and `?c rdfs:subClassOf ?c` is one
/// of them — since inside the saturation's fragment the derivation IS the entailment
/// relation, and outside it the underivable pairs still go to
/// [`Kb::entails_subclass`].
fn inject_classification(
    b: &mut RdfDatasetBuilder,
    kb: &Kb,
    named_cid: &BTreeMap<u32, u32>,
    taxonomy: &Taxonomy,
) -> Result<(), EntailError> {
    let sub_class = b.intern_iri(RDFS_SUBCLASSOF);
    let complete = taxonomy.is_complete();
    for (&c_iri, &c_cid) in named_cid {
        for (&d_iri, &d_cid) in named_cid {
            let holds = taxonomy.derives(c_cid, d_cid)
                || (!complete && kb.entails_subclass(c_cid, d_cid)?);
            if holds {
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
                    // An `owl:sameAs` between an individual and a LITERAL puts the literal
                    // in the individual's equality class, so re-stating the individual's
                    // triples over it would put a literal in SUBJECT position — a
                    // generalized-RDF triple the dataset IR cannot hold, which used to
                    // fail the whole freeze. The conclusion is genuinely entailed and
                    // genuinely unrepresentable, so it is abandoned here exactly as the
                    // forward chase abandons its own; every representable conclusion of
                    // the same equality class is still stated.
                    if !kb.interner.is_subject(s2) {
                        continue;
                    }
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
    collect_scaffold(interner, q_index, root, &mut scaffold);
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

#[cfg(test)]
mod tests {
    use super::*;
    use purrdf_core::RdfDatasetBuilder;

    const NS: &str = "http://example.org/dl#";
    const RDF_TYPE_IRI: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
    const OWL_CLASS: &str = "http://www.w3.org/2002/07/owl#Class";

    /// The constructs a run raised, in the report's own order.
    fn constructs(report: &ReasoningReport) -> Vec<Construct> {
        report
            .boundaries()
            .iter()
            .map(|boundary| boundary.construct())
            .collect()
    }

    /// A minimal ontology: one declared class and one instance of it. `graph` decides
    /// whether the instance assertion sits in the default graph or outside it.
    fn ontology(graph: Option<&str>) -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let ty = b.intern_iri(RDF_TYPE_IRI);
        let class = b.intern_iri(OWL_CLASS);
        let a = b.intern_iri(&format!("{NS}A"));
        let x = b.intern_iri(&format!("{NS}x"));
        let g = graph.map(|g| b.intern_iri(g));
        b.push_quad(a, ty, class, None);
        b.push_quad(x, ty, a, g);
        b.freeze().expect("freeze")
    }

    /// A QUAD THE TABLEAU NEVER READ IS NAMED, NOT ASSUMED AWAY.
    ///
    /// The reverse mapping indexes the DEFAULT graph, so an assertion in a named graph
    /// constrains nothing it decides. Before this boundary the run answered with a
    /// certificate that looked complete while most of an input could have been ignored —
    /// the failure mode a report exists to make impossible, and the one that is worse than
    /// a wrong answer because nothing signals it.
    #[test]
    fn a_named_graph_quad_raises_the_boundary_on_the_dl_lane() {
        let (_, report) =
            materialize_dl_reported(&ontology(Some("http://example.org/g")), &[]).expect("dl run");
        assert!(
            constructs(&report).contains(&Construct::NamedGraph),
            "{:?}",
            constructs(&report)
        );
        // And the certificate stops claiming a flatly-exact run.
        assert_eq!(
            report.completeness(),
            crate::Completeness::ExactWithinBoundaries
        );
    }

    /// …and a default-graph-only ontology raises nothing, so the boundary is EVIDENCE
    /// about an input rather than a standing disclaimer.
    #[test]
    fn a_default_graph_ontology_raises_no_named_graph_boundary() {
        let (_, report) = materialize_dl_reported(&ontology(None), &[]).expect("dl run");
        assert!(!constructs(&report).contains(&Construct::NamedGraph));
    }
}
