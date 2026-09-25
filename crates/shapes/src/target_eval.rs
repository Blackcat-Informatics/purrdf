// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The targets that are EVALUATED rather than looked up: explicit shape targets
//! (`sh:shape` in the data graph), where targets (`sh:targetWhere`) and
//! node-expression node targets (a structured `sh:targetNode`).
//!
//! The Core targets keyed by a class or a predicate become membership indexes
//! (see `crate::engine`), because the question "is this node in the target?" is
//! one pattern lookup. None of the three here is: each is a fact about the data
//! graph as a whole, so each is resolved to an explicit node set once per
//! validation, exactly as a SHACL-SPARQL target is.
//!
//! # Explicit shape targets
//!
//! SHACL 1.2 Core, "Explicit shape targets (sh:shape)": "If s is a shape in a
//! shapes graph and n is a node in the data graph. If n has value s for sh:shape
//! in the data graph, then n is a target for s." And from "Targets":
//! "Furthermore, sh:shape triples can declare targets in the data graph." So the
//! declaration lives in the DATA graph and names the shape; a data node naming a
//! term the shapes graph does not hold as a shape targets nothing, because the
//! rule applies only to a shape.
//!
//! # Where targets
//!
//! SHACL 1.2 Core, "Where Targets (sh:targetWhere)": "If s is a shape in a shapes
//! graph SG and s has value w for sh:targetWhere in SG then the set of nodes in a
//! data graph DG that conform to w is a target from DG for s in SG."
//!
//! "Nodes in a data graph" is RDF 1.2 Concepts' term: "The set of nodes of an RDF
//! graph is the set of subjects and objects of the asserted triples of the graph."
//! A triple term that is an OBJECT is therefore a candidate, and a term that
//! occurs only INSIDE a triple term is not.
//!
//! The spec is candid about cost — "In the worst case, an engine will need to
//! iterate over all nodes in the data graph to filter them one-by-one" — and the
//! word is WORST: a node can conform to `w` only if it satisfies each of `w`'s
//! constraints, so a constraint that can only be satisfied by nodes of a known
//! set NARROWS the candidates to that set without changing the answer. Every
//! narrowed candidate is still checked against the whole of `w`, so a narrowing
//! can only remove nodes that could not have conformed. See [`narrowing`] for the
//! constraints that narrow and the conditions under which they may. When nothing
//! narrows, the scan is over [`ShaclData::graph_nodes`], built once per validation
//! and shared by every where target.
//!
//! # Node-expression node targets
//!
//! SHACL 1.2 Core, "Node targets (sh:targetNode)": "If s is a shape in a shapes
//! graph SG and s has value expr for sh:targetNode in SG, then the output nodes of
//! evalExpr(expr, data graph, s, {}) are targets for the data graph DG as focus
//! graph." The FOCUS NODE of that evaluation is the shape `s` itself, and the
//! focus graph is the data graph; there are no variable bindings.

use ::purrdf::{IdSet, TermId};

use crate::constraints::conforms_with_id_depth;
use crate::data::{GraphFilter, ShaclData, quads_for_pattern_ids, resolve_id};
use crate::data_view::ShaclRead;
use crate::engine::FocusNode;
use crate::expression::NodeExpr;
use crate::model::sh;
use crate::report::ConformanceDisallows;
use crate::shapes::{Constraint, Path, PropertyShape, Shape};
use crate::term::{NamedNode, Term};

/// How deep [`narrowing`] descends through `sh:and` / `sh:node` looking for a
/// narrowing constraint. A bound on the search, not on the answer: past it the
/// candidates are simply not narrowed further.
const MAX_NARROWING_DEPTH: u32 = 8;

/// The data-graph nodes that declare `shape` as their shape through `sh:shape`,
/// deduplicated, in first-seen order.
///
/// Empty when the data graph interns neither `sh:shape` nor the shape's term —
/// the ordinary case, answered by two interner lookups.
pub(crate) fn declared_shape_targets(data: &ShaclData, shape: &Term) -> Vec<TermId> {
    let ds = data.core_view();
    let Some(predicate) = ds.term_id_by_iri(sh::SHAPE) else {
        return Vec::new();
    };
    let Some(shape) = resolve_id(ds, shape) else {
        return Vec::new();
    };
    let mut seen = IdSet::default();
    quads_for_pattern_ids(
        ds,
        None,
        Some(predicate),
        Some(shape),
        GraphFilter::AnyGraph,
    )
    .filter(|quad| seen.insert(quad.s))
    .map(|quad| quad.s)
    .collect()
}

/// The nodes of the data graph that conform to `shape`, the value of one
/// `sh:targetWhere`, judged under the run's `disallows` (see the module docs).
///
/// # Errors
///
/// Propagates a conformance check that hard-fails — a failure SHACL requires be
/// reported, never read as "does not conform".
pub(crate) fn where_targets(
    data: &ShaclData,
    shape: &Shape,
    disallows: &ConformanceDisallows,
) -> Result<Vec<TermId>, String> {
    let lowered = crate::plan::lower_shapes(std::iter::once(shape));
    let mut binding = lowered.bind(data.core_view(), lowered.classes());
    binding.set_conformance_disallows(disallows);
    let plan = lowered.plan(shape, 0, &binding, lowered.classes(), lowered.no_targets())?;
    data.prepare_class_membership();
    let narrowed = narrowing(data, shape, disallows, 0);
    let candidates: &[TermId] = match &narrowed {
        Some(ids) => ids,
        None => data.graph_nodes(),
    };
    let mut out: Vec<TermId> = Vec::new();
    for &node in candidates {
        if conforms_with_id_depth(data, &FocusNode::Interned(node), plan, 0)? {
            out.push(node);
        }
    }
    Ok(out)
}

/// The output nodes of `evalExpr(expr, data graph, shape, {})`, the value of one
/// structured `sh:targetNode` on the shape whose node is `shape`.
///
/// # Errors
///
/// Propagates an evaluation failure: an unanswerable target is not an empty one.
pub(crate) fn node_expression_targets(
    data: &ShaclData,
    shape: &Term,
    expr: &NodeExpr,
    disallows: &ConformanceDisallows,
) -> Result<Vec<Term>, String> {
    crate::expression::eval_node_expr_in_run(data, shape, expr, disallows)
        .map_err(|e| format!("sh:targetNode node expression on shape {shape} failed: {e}"))
}

/// A set of data-graph NODES that holds every node that can conform to `shape`,
/// or `None` when no constraint of `shape` bounds it (the caller then scans every
/// node).
///
/// A constraint narrows only when a node that violates it CANNOT conform to
/// `shape`. That holds when the constraint is live and its violation counts
/// against conformance: the shape (and, for a property shape, the property shape)
/// is not deactivated, its severity is in the run's conformance-disallow set, and
/// it carries no per-constraint reifier annotation, which could re-grade the
/// violation to a severity that does not count. Under those conditions:
///
/// * `sh:class` at node level — the focus node is its own value node, so it must
///   be a SHACL instance of one of the classes: the candidates are their
///   instances;
/// * `sh:hasValue v` at node level — the focus node must be `v`;
/// * `sh:in (…)` at node level — the focus node must be a member;
/// * a property shape on a predicate path `p` (or its inverse) with `sh:minCount`
///   at least 1 — the node must be a subject (object) of `p`;
/// * `sh:and` / `sh:node` — the node must conform to each operand, so any
///   operand's narrowing bounds it too.
///
/// Every set returned holds only nodes of the data graph: the `sh:hasValue` and
/// `sh:in` members are filtered by [`ShaclData::is_graph_node`], and instances and
/// subjects / objects of a predicate are nodes by construction. The smallest set
/// found is returned.
fn narrowing(
    data: &ShaclData,
    shape: &Shape,
    disallows: &ConformanceDisallows,
    depth: u32,
) -> Option<Vec<TermId>> {
    if depth > MAX_NARROWING_DEPTH
        || shape.deactivated
        || !disallows.contains(&shape.severity)
        || !shape.constraint_annotations.is_empty()
    {
        return None;
    }
    let mut best: Option<Vec<TermId>> = None;
    let mut offer = |candidate: Vec<TermId>| {
        if best
            .as_ref()
            .is_none_or(|current| candidate.len() < current.len())
        {
            best = Some(candidate);
        }
    };
    for constraint in &shape.constraints {
        match constraint {
            Constraint::Class(classes) => offer(instances_of_any(data, classes)),
            Constraint::HasValue(value) => offer(graph_nodes_among(data, std::iter::once(value))),
            Constraint::In(members) => offer(graph_nodes_among(data, members.iter())),
            Constraint::And(operands) => {
                for operand in operands {
                    if let Some(ids) = narrowing(data, operand, disallows, depth + 1) {
                        offer(ids);
                    }
                }
            }
            Constraint::Node(operand) => {
                if let Some(ids) = narrowing(data, operand, disallows, depth + 1) {
                    offer(ids);
                }
            }
            _ => {}
        }
    }
    for property in &shape.property_shapes {
        if let Some(ids) = property_narrowing(data, property, disallows) {
            offer(ids);
        }
    }
    best
}

/// The narrowing one property shape offers: the subjects (objects) of its
/// predicate (inverse predicate) path when it requires at least one value. See
/// [`narrowing`] for the conditions.
fn property_narrowing(
    data: &ShaclData,
    property: &PropertyShape,
    disallows: &ConformanceDisallows,
) -> Option<Vec<TermId>> {
    if property.deactivated
        || !disallows.contains(&property.severity)
        || !property.constraint_annotations.is_empty()
    {
        return None;
    }
    let requires_a_value = property
        .constraints
        .iter()
        .any(|constraint| matches!(constraint, Constraint::MinCount(min) if *min >= 1));
    if !requires_a_value {
        return None;
    }
    let (predicate, subject_end) = match &property.path {
        Path::Predicate(predicate) => (predicate, true),
        Path::Inverse(inner) => match inner.as_ref() {
            Path::Predicate(predicate) => (predicate, false),
            _ => return None,
        },
        _ => return None,
    };
    Some(ends_of(data, predicate, subject_end))
}

/// The distinct subjects (`subject_end`) or objects of `predicate` in the data
/// graph.
fn ends_of(data: &ShaclData, predicate: &NamedNode, subject_end: bool) -> Vec<TermId> {
    let ds = data.core_view();
    let Some(predicate) = ds.term_id_by_iri(predicate.as_str()) else {
        return Vec::new();
    };
    let mut seen = IdSet::default();
    quads_for_pattern_ids(ds, None, Some(predicate), None, GraphFilter::AnyGraph)
        .map(|quad| if subject_end { quad.s } else { quad.o })
        .filter(|node| seen.insert(*node))
        .collect()
}

/// The SHACL instances, in the data graph, of any of `classes`.
fn instances_of_any(data: &ShaclData, classes: &[NamedNode]) -> Vec<TermId> {
    let ds = data.core_view();
    let mut seen = IdSet::default();
    let mut out: Vec<TermId> = Vec::new();
    for class in classes {
        let Some(class) = ds.term_id_by_iri(class.as_str()) else {
            continue;
        };
        for instance in data.class_view().instances_of(class) {
            if seen.insert(instance) {
                out.push(instance);
            }
        }
    }
    out
}

/// Those of `terms` that are nodes of the data graph.
fn graph_nodes_among<'t>(data: &ShaclData, terms: impl Iterator<Item = &'t Term>) -> Vec<TermId> {
    let ds = data.core_view();
    let mut seen = IdSet::default();
    terms
        .filter_map(|term| resolve_id(ds, term))
        .filter(|&node| data.is_graph_node(node) && seen.insert(node))
        .collect()
}
